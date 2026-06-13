#![allow(dead_code)]
//! 古代星表数据数字化与现代天体物理验证系统
//! Rust 后端 (Actix-Web) v0.3
//!
//! 架构重构 v0.3:
//!   - 拆分为 3 个独立模块，通过 tokio channel 通信
//!   - 模型参数全部从 config/ JSON 文件加载
//!
//! 模块职责:
//!   catalog_loader        星表数据导入 + 清洗 (DB → Channel)
//!   coordinate_transformer 岁差/章动/自行 + 误差估计
//!   transient_matcher      客星-超新星贝叶斯匹配
//!
//! main.rs 职责:
//!   - 加载配置
//!   - 启动子模块任务
//!   - 作为 REST API 层 + 模块协调器

mod config;
mod telemetry;
mod catalog_loader;
mod coordinate_transformer;
mod transient_matcher;
mod astronomy;
mod matching;
mod models;
mod db;

use actix_web::{web, App, HttpServer, HttpResponse, get, post, Responder};
use actix_files::Files;
use actix_cors::Cors;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use tracing::{info, error};

use config::AppConfig;
use db::DbPool;
use models::*;
use astronomy::{RuxiuToJ2000Request, TrajectoryRequest};
use catalog_loader::{LoaderCommand, LoaderEvent};
use coordinate_transformer::{TransformCommand, TransformEvent, TransformResult};
use transient_matcher::{MatchCommand, MatchEvent, MatchMethodInfo};
use telemetry::MetricsRegistry;

struct AppState {
    pool: DbPool,
    config: AppConfig,
    loader_tx: tokio::sync::mpsc::Sender<LoaderCommand>,
    loader_rx: Arc<Mutex<tokio::sync::mpsc::Receiver<LoaderEvent>>>,
    transform_tx: tokio::sync::mpsc::Sender<TransformCommand>,
    transform_rx: Arc<Mutex<tokio::sync::mpsc::Receiver<TransformEvent>>>,
    match_tx: tokio::sync::mpsc::Sender<MatchCommand>,
    match_rx: Arc<Mutex<tokio::sync::mpsc::Receiver<MatchEvent>>>,
    metrics: Arc<MetricsRegistry>,
}

const CHANNEL_TIMEOUT_MS: u64 = 30000;

// ============================================================
// 健康检查
// ============================================================

#[get("/health")]
async fn api_health(data: web::Data<Arc<AppState>>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "models": ["IAU 2006 precession", "Galactic prior Bayes", "Planck color temp"],
        "modules": {
            "precession": data.config.precession.model_name.clone(),
            "matching": data.config.matching.model_name.clone(),
            "catalog": data.config.catalog.model_name.clone(),
        },
        "architecture": "3-modules + channels (catalog_loader → coordinate_transformer → transient_matcher)",
    })))
}

#[get("/metrics")]
async fn api_metrics(data: web::Data<Arc<AppState>>) -> impl Responder {
    match data.metrics.encode_text() {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4; charset=utf-8")
            .body(body),
        Err(e) => {
            error!("Failed to encode metrics: {}", e);
            HttpResponse::InternalServerError().body(e)
        }
    }
}

// ============================================================
// 朝代 / 星宿
// ============================================================

#[get("/dynasties")]
async fn api_dynasties(data: web::Data<Arc<AppState>>) -> impl Responder {
    match db::list_dynasties(&data.pool).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok(list)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[get("/mansions")]
async fn api_mansions(data: web::Data<Arc<AppState>>) -> impl Responder {
    match db::list_mansions(&data.pool).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok(list)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

// ============================================================
// 恒星 CRUD + 查询 (通过 catalog_loader 模块)
// ============================================================

#[get("/stars")]
async fn api_query_stars(
    data: web::Data<Arc<AppState>>,
    query: web::Query<StarQueryParams>,
) -> impl Responder {
    let params: StarQueryParams = query.into_inner();

    match db::query_stars(&data.pool, &params).await {
        Ok((list, total)) => {
            let mut rx = data.loader_rx.lock().await;
            if data.loader_tx.send(LoaderCommand::CleanStars {
                stars: list.clone()
            }).await.is_err() {
                return HttpResponse::InternalServerError().json(
                    ApiResponse::<()>::err("Loader channel send failed"));
            }
            match timeout(Duration::from_millis(CHANNEL_TIMEOUT_MS), rx.recv()).await {
                Ok(Some(LoaderEvent::StarsCleaned { records, .. })) => {
                    let response = serde_json::json!({
                        "raw": list,
                        "cleaned": records,
                    });
                    HttpResponse::Ok().json(ApiResponse::ok_with_count(response, total))
                }
                Ok(Some(LoaderEvent::Error { message })) => {
                    HttpResponse::InternalServerError().json(
                        ApiResponse::<()>::err(format!("Loader: {}", message)))
                }
                _ => HttpResponse::Ok().json(ApiResponse::ok_with_count(list, total)),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[get("/stars/{id}")]
async fn api_get_star(
    data: web::Data<Arc<AppState>>,
    id: web::Path<i64>,
) -> impl Responder {
    match db::get_star(&data.pool, id.into_inner()).await {
        Ok(Some(s)) => HttpResponse::Ok().json(ApiResponse::ok(s)),
        Ok(None) => HttpResponse::NotFound().json(ApiResponse::<()>::err("Star not found")),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[get("/stars/{id}/cross-dynasty")]
async fn api_cross_dynasty(
    data: web::Data<Arc<AppState>>,
    id: web::Path<i64>,
) -> impl Responder {
    let star_id = id.into_inner();
    let star_opt = db::get_star(&data.pool, star_id).await.ok().flatten();
    let name = star_opt.as_ref().map(|s| s.star_name_cn.clone());
    match db::get_star_cross_dynasty(&data.pool, Some(star_id), name).await {
        Ok(list) => {
            let total = list.len() as i64;
            HttpResponse::Ok().json(ApiResponse::ok_with_count(list, total))
        }
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

// ============================================================
// 坐标转换 API (通过 coordinate_transformer 模块)
// ============================================================

#[post("/convert/ruxiu-to-j2000")]
async fn api_convert_ruxiu(
    data: web::Data<Arc<AppState>>,
    body: web::Json<RuxiuToJ2000Request>,
) -> impl Responder {
    let req = body.into_inner();
    let cmd = TransformCommand::ConvertSingle {
        ruxiu_du: req.ruxiu_du,
        quji_du: req.quji_du,
        mansion_order: req.mansion_order,
        epoch_yr: req.epoch_yr,
        pm_ra_mas: req.pm_ra_mas,
        pm_dec_mas: req.pm_dec_mas,
    };

    let mut rx = data.transform_rx.lock().await;
    if data.transform_tx.send(cmd).await.is_err() {
        return HttpResponse::InternalServerError().json(
            ApiResponse::<()>::err("Transform channel send failed"));
    }

    match timeout(Duration::from_millis(CHANNEL_TIMEOUT_MS), rx.recv()).await {
        Ok(Some(TransformEvent::SingleConverted(result))) => {
            let resp = build_convert_response(&result);
            HttpResponse::Ok().json(ApiResponse::ok(resp))
        }
        Ok(Some(TransformEvent::Error { message })) => {
            HttpResponse::InternalServerError().json(
                ApiResponse::<()>::err(format!("Transform: {}", message)))
        }
        Ok(None) => {
            HttpResponse::InternalServerError().json(
                ApiResponse::<()>::err("Transform channel closed"))
        }
        Err(_) => {
            HttpResponse::RequestTimeout().json(
                ApiResponse::<()>::err("Transform timeout"))
        }
        _ => HttpResponse::InternalServerError().json(
            ApiResponse::<()>::err("Unexpected transform event")),
    }
}

#[post("/trajectory")]
async fn api_trajectory(
    data: web::Data<Arc<AppState>>,
    body: web::Json<TrajectoryRequest>,
) -> impl Responder {
    let req = body.into_inner();
    let cmd = TransformCommand::ComputeTrajectory {
        ra_j2000: req.ra_j2000,
        dec_j2000: req.dec_j2000,
        pm_ra_mas: req.pm_ra_mas,
        pm_dec_mas: req.pm_dec_mas,
        year_start: req.year_start,
        year_end: req.year_end,
        n_points: req.n_points,
    };

    let mut rx = data.transform_rx.lock().await;
    if data.transform_tx.send(cmd).await.is_err() {
        return HttpResponse::InternalServerError().json(
            ApiResponse::<()>::err("Transform channel send failed"));
    }

    match timeout(Duration::from_millis(CHANNEL_TIMEOUT_MS), rx.recv()).await {
        Ok(Some(TransformEvent::TrajectoryComputed { points })) => {
            HttpResponse::Ok().json(ApiResponse::ok(points))
        }
        Ok(Some(TransformEvent::Error { message })) => {
            HttpResponse::InternalServerError().json(
                ApiResponse::<()>::err(format!("Transform: {}", message)))
        }
        Err(_) => HttpResponse::RequestTimeout().json(
            ApiResponse::<()>::err("Transform timeout")),
        _ => HttpResponse::InternalServerError().json(
            ApiResponse::<()>::err("Unexpected transform event")),
    }
}

// ============================================================
// 彗星 / 客星 / SNR
// ============================================================

#[get("/comets")]
async fn api_comets(data: web::Data<Arc<AppState>>) -> impl Responder {
    match db::list_comets(&data.pool).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok(list)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[get("/guest-stars")]
async fn api_guest_stars(data: web::Data<Arc<AppState>>) -> impl Responder {
    match db::list_guest_stars(&data.pool).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok(list)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[get("/guest-stars/{id}")]
async fn api_get_guest(
    data: web::Data<Arc<AppState>>,
    id: web::Path<i64>,
) -> impl Responder {
    match db::get_guest_star(&data.pool, id.into_inner()).await {
        Ok(Some(g)) => HttpResponse::Ok().json(ApiResponse::ok(g)),
        Ok(None) => HttpResponse::NotFound().json(ApiResponse::<()>::err("Not found")),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[get("/snr")]
async fn api_snr(data: web::Data<Arc<AppState>>) -> impl Responder {
    match db::list_snr(&data.pool).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok(list)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

// ============================================================
// 贝叶斯匹配 API (通过 transient_matcher 模块)
// ============================================================

#[get("/match/{guest_id}")]
async fn api_get_matches(
    data: web::Data<Arc<AppState>>,
    guest_id: web::Path<i64>,
) -> impl Responder {
    let gid = guest_id.into_inner();
    match db::get_match_results(&data.pool, gid).await {
        Ok(list) if !list.is_empty() => {
            HttpResponse::Ok().json(ApiResponse::ok(list))
        }
        _ => {
            match run_match_via_matcher(&data, gid, 20).await {
                Ok((candidates, _)) => HttpResponse::Ok().json(ApiResponse::ok(candidates)),
                Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
            }
        }
    }
}

#[post("/match/{guest_id}")]
async fn api_run_match(
    data: web::Data<Arc<AppState>>,
    guest_id: web::Path<i64>,
    query: web::Query<MatchRequest>,
) -> impl Responder {
    let gid = guest_id.into_inner();
    let top_k = query.top_k.unwrap_or(10);
    match run_match_via_matcher(&data, gid, top_k).await {
        Ok((candidates, method)) => {
            let guest = db::get_guest_for_match(&data.pool, gid).await.ok().flatten();
            Ok(serde_json::json!({
                "guest": guest,
                "candidates": candidates,
                "method": method,
            }))
        }
        Err(e) => Err(e),
    }.map_or_else(
        |e| HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
        |v| HttpResponse::Ok().json(ApiResponse::ok(v))
    )
}

async fn run_match_via_matcher(
    data: &web::Data<Arc<AppState>>,
    guest_id: i64,
    top_k: i32,
) -> Result<(Vec<matching::MatchCandidate>, MatchMethodInfo), String> {
    let guest = db::get_guest_for_match(&data.pool, guest_id).await?
        .ok_or_else(|| "Guest star not found".to_string())?;
    let snrs = db::list_snr_for_match(&data.pool).await?;

    let cmd = MatchCommand::RunMatch {
        guest: guest.clone(),
        snrs: snrs.clone(),
        top_k,
    };

    let mut rx = data.match_rx.lock().await;
    data.match_tx.send(cmd).await
        .map_err(|_| "Matcher channel send failed".to_string())?;

    match timeout(Duration::from_millis(CHANNEL_TIMEOUT_MS), rx.recv()).await {
        Ok(Some(MatchEvent::MatchCompleted { candidates, method, .. })) => {
            let ver = env!("CARGO_PKG_VERSION");
            let top20 = candidates.iter().take(20).cloned().collect::<Vec<_>>();
            db::save_match_result(&data.pool, guest_id, &top20, ver).await.ok();
            Ok((candidates, method))
        }
        Ok(Some(MatchEvent::Error { message })) => {
            Err(format!("Matcher: {}", message))
        }
        Ok(None) => Err("Matcher channel closed".to_string()),
        Err(_) => Err("Matcher timeout".to_string()),
        _ => Err("Unexpected matcher event".to_string()),
    }
}

// ============================================================
// 响应构造
// ============================================================

fn build_convert_response(r: &TransformResult) -> serde_json::Value {
    serde_json::json!({
        "ancient_equatorial": {
            "ra_deg": r.ancient_ra,
            "dec_deg": r.ancient_dec,
        },
        "j2000": {
            "ra_deg": r.ra_j2000,
            "dec_deg": r.dec_j2000,
            "without_proper_motion": {
                "ra_deg": r.ra_without_pm,
                "dec_deg": r.dec_without_pm,
            }
        },
        "precession_matrix": r.precession_matrix,
        "corrections": {
            "nutation_psi_arcsec": r.nutation_correction[0],
            "nutation_eps_arcsec": r.nutation_correction[1],
            "planetary_chi_arcsec": r.planetary_correction_arcsec,
        },
        "proper_motion_1000yr": {
            "dra_deg": r.proper_motion_arrow[0],
            "ddec_deg": r.proper_motion_arrow[1],
            "position_angle_deg": r.proper_motion_arrow[2],
        },
        "error_estimate": {
            "ra_arcsec": r.error_estimate.ra_error_arcsec,
            "dec_arcsec": r.error_estimate.dec_error_arcsec,
            "model_arcsec": r.error_estimate.model_error_arcsec,
            "observation_arcsec": r.error_estimate.observation_error_arcsec,
            "proper_motion_arcsec": r.error_estimate.proper_motion_error_arcsec,
        }
    })
}

// ============================================================
// 主入口
// ============================================================

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    telemetry::init_tracing();

    info!("======================================================");
    info!("  Ancient Star Catalog Backend v{}", env!("CARGO_PKG_VERSION"));
    info!("  Architecture: 3-modules + tokio channels");
    info!("======================================================");

    let config_dir = env::var("CONFIG_DIR").unwrap_or_else(|_| "./config".into());
    let config = match config::AppConfig::load(&config_dir) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load config from {}: {}", config_dir, e);
            error!("Make sure config/precession.json, config/matching.json, config/catalog.json exist");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    };
    info!("  Config loaded from: {}", config_dir);
    info!("  - Precession: {}", config.precession.model_name);
    info!("  - Matching:   {}", config.matching.model_name);
    info!("  - Catalog:    {}", config.catalog.model_name);

    let host = env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = env::var("API_PORT").ok()
        .and_then(|s| s.parse().ok()).unwrap_or(8080);

    let pool = db::create_pool().expect("Failed to create DB pool");

    let metrics = Arc::new(telemetry::register_metrics()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?);

    info!("Spawning modules...");
    let (loader_tx, loader_rx) = catalog_loader::spawn_loader(
        config.catalog.clone());
    info!("catalog_loader started (DB import + cleaning)");

    let (transform_tx, transform_rx) = coordinate_transformer::spawn_transformer(
        config.precession.clone());
    info!("coordinate_transformer started (IAU 2006 + error estimate)");

    let (match_tx, match_rx) = transient_matcher::spawn_matcher(
        config.matching.clone());
    info!("transient_matcher started (Galactic prior Bayes)");

    let state = Arc::new(AppState {
        pool,
        config,
        loader_tx,
        loader_rx: Arc::new(Mutex::new(loader_rx)),
        transform_tx,
        transform_rx: Arc::new(Mutex::new(transform_rx)),
        match_tx,
        match_rx: Arc::new(Mutex::new(match_rx)),
        metrics: metrics.clone(),
    });

    info!("======================================================");
    info!("  API: http://{}:{}", host, port);
    info!("======================================================");

    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(cors)
            .service(
                web::scope("/api")
                    .service(api_health)
                    .service(api_metrics)
                    .service(api_dynasties)
                    .service(api_mansions)
                    .service(api_query_stars)
                    .service(api_get_star)
                    .service(api_cross_dynasty)
                    .service(api_convert_ruxiu)
                    .service(api_trajectory)
                    .service(api_comets)
                    .service(api_guest_stars)
                    .service(api_get_guest)
                    .service(api_snr)
                    .service(api_get_matches)
                    .service(api_run_match)
            )
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
