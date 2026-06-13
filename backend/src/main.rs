//! 古代星表数据数字化与现代天体物理验证系统
//! Rust 后端 (Actix-Web)
//!
//! 修复记录 v0.2:
//!   1. IAU 2006 岁差模型 + 行星摄动修正
//!   2. 贝叶斯先验升级为银河系分布模型
//!   3. 前端 Planck 黑体辐射色温映射 (前端修复)

mod astronomy;
mod matching;
mod models;
mod db;

use actix_web::{web, App, HttpServer, HttpResponse, get, post, Responder};
use actix_files::Files;
use actix_cors::Cors;
use std::env;
use std::sync::Arc;

use db::DbPool;
use models::*;
use astronomy::*;
use matching::{MatchConfig, run_bayesian_match};

struct AppState {
    pool: DbPool,
    match_cfg: MatchConfig,
}

// ============================================================
// 健康检查
// ============================================================

#[get("/health")]
async fn api_health() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "models": ["IAU 2006 precession", "Galactic prior Bayes", "Planck color temp"],
    })))
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
// 恒星 CRUD + 查询
// ============================================================

#[get("/stars")]
async fn api_query_stars(
    data: web::Data<Arc<AppState>>,
    query: web::Query<StarQueryParams>,
) -> impl Responder {
    let params: StarQueryParams = query.into_inner();
    match db::query_stars(&data.pool, &params).await {
        Ok((list, total)) => HttpResponse::Ok().json(ApiResponse::ok_with_count(list, total)),
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
    // 用 star_name 做跨朝代匹配
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
// 坐标转换 API
// ============================================================

#[post("/convert/ruxiu-to-j2000")]
async fn api_convert_ruxiu(
    body: web::Json<RuxiuToJ2000Request>,
) -> impl Responder {
    let result = convert_ruxiu_to_j2000(&body.into_inner());
    HttpResponse::Ok().json(ApiResponse::ok(result))
}

#[post("/trajectory")]
async fn api_trajectory(body: web::Json<TrajectoryRequest>) -> impl Responder {
    let pts = compute_trajectory(&body.into_inner());
    HttpResponse::Ok().json(ApiResponse::ok(pts))
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
// 贝叶斯匹配 API
// ============================================================

#[get("/match/{guest_id}")]
async fn api_get_matches(
    data: web::Data<Arc<AppState>>,
    guest_id: web::Path<i64>,
) -> impl Responder {
    let gid = guest_id.into_inner();
    // 若数据库已有结果直接返回
    match db::get_match_results(&data.pool, gid).await {
        Ok(list) if !list.is_empty() => {
            HttpResponse::Ok().json(ApiResponse::ok(list))
        }
        _ => {
            // 没有就实时计算
            match run_match_internal(&data, gid).await {
                Ok(candidates) => HttpResponse::Ok().json(ApiResponse::ok(candidates)),
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
    match run_match_full(&data, gid, top_k).await {
        Ok(resp) => HttpResponse::Ok().json(ApiResponse::ok(resp)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

async fn run_match_internal(data: &web::Data<Arc<AppState>>, guest_id: i64)
    -> Result<Vec<matching::MatchCandidate>, String>
{
    let guest = db::get_guest_for_match(&data.pool, guest_id).await?
        .ok_or_else(|| "Guest star not found".to_string())?;
    let snrs = db::list_snr_for_match(&data.pool).await?;
    let mut result = run_bayesian_match(&guest, &snrs, &data.match_cfg);
    if result.is_empty() { return Ok(result); }
    // 保存结果
    let ver = env!("CARGO_PKG_VERSION");
    let top = result.iter().take(20).cloned().collect::<Vec<_>>();
    db::save_match_result(&data.pool, guest_id, &top, ver).await.ok();
    Ok(result)
}

async fn run_match_full(data: &web::Data<Arc<AppState>>, guest_id: i64, top_k: i32)
    -> Result<serde_json::Value, String>
{
    let guest = db::get_guest_for_match(&data.pool, guest_id).await?
        .ok_or_else(|| "Guest star not found".to_string())?;
    let snrs = db::list_snr_for_match(&data.pool).await?;
    let mut result = run_bayesian_match(&guest, &snrs, &data.match_cfg);
    result.truncate(top_k.max(5) as usize);
    let ver = env!("CARGO_PKG_VERSION");
    let top20 = result.iter().take(20).cloned().collect::<Vec<_>>();
    db::save_match_result(&data.pool, guest_id, &top20, ver).await.ok();

    Ok(serde_json::json!({
        "guest": guest,
        "candidates": result,
        "method": {
            "name": "Bayesian Spatial-Temporal Matching v2",
            "version": ver,
            "model": "IAU 2006 precession + Galactic disk prior + Student-t likelihood",
            "prior_model": "Exponential disk (R_d=4 kpc) + isothermal disk (z_d=50 pc)",
            "n_candidates_evaluated": snrs.len(),
            "n_candidates_returned": result.len(),
        }
    }))
}

// ============================================================
// 主入口
// ============================================================

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let host = env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = env::var("API_PORT").ok()
        .and_then(|s| s.parse().ok()).unwrap_or(8080);

    let pool = db::create_pool().expect("Failed to create DB pool");
    let match_cfg = MatchConfig::default();
    let state = Arc::new(AppState { pool, match_cfg });

    println!("======================================================");
    println!("  Ancient Star Catalog Backend v{}", env!("CARGO_PKG_VERSION"));
    println!("======================================================");
    println!("  修复 1: IAU 2006 岁差模型 + 行星摄动修正");
    println!("  修复 2: 贝叶斯先验 → 银河系分布模型");
    println!("  修复 3: 前端 Planck 黑体辐射色温映射");
    println!("======================================================");
    println!("  API: http://{}:{}", host, port);
    println!("======================================================");

    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(cors)
            .service(
                web::scope("/api")
                    .service(api_health)
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
