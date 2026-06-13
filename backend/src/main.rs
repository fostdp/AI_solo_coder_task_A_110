//! 古代星表数据数字化与现代天体物理验证系统
//! Rust Backend - REST API Server

mod astronomy;
mod matching;
mod models;
mod db;

use actix_web::{web, App, HttpResponse, HttpServer, Responder, get, post, middleware};
use actix_cors::Cors;
use serde::Deserialize;
use std::env;
use std::sync::Arc;

use db::DbPool;
use models::*;
use astronomy::*;
use matching::{MatchConfig, run_bayesian_match};

// ======================================================================
// 应用状态
// ======================================================================

struct AppState {
    pool: DbPool,
    match_cfg: MatchConfig,
}

// ======================================================================
// 健康检查
// ======================================================================

#[get("/api/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "ancient-star-catalog-api",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// ======================================================================
// 基础数据 API
// ======================================================================

#[get("/api/dynasties")]
async fn get_dynasties(data: web::Data<Arc<AppState>>) -> impl Responder {
    match db::list_dynasties(&data.pool).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok(list)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[get("/api/mansions")]
async fn get_mansions(data: web::Data<Arc<AppState>>) -> impl Responder {
    match db::list_mansions(&data.pool).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok(list)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

// ======================================================================
// 恒星查询 API
// ======================================================================

#[get("/api/stars")]
async fn query_stars(
    data: web::Data<Arc<AppState>>,
    qs: web::Query<StarQueryParams>,
) -> impl Responder {
    match db::query_stars(&data.pool, &qs.into_inner()).await {
        Ok((list, count)) => HttpResponse::Ok().json(ApiResponse::ok_with_count(list, count)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[get("/api/stars/{id}")]
async fn get_star(
    data: web::Data<Arc<AppState>>,
    path: web::Path<i64>,
) -> impl Responder {
    match db::get_star_by_id(&data.pool, path.into_inner()).await {
        Ok(Some(s)) => HttpResponse::Ok().json(ApiResponse::ok(s)),
        Ok(None) => HttpResponse::NotFound().json(ApiResponse::<()>::err("Star not found")),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

// ======================================================================
// 坐标转换 API
// ======================================================================

#[post("/api/convert/ruxiu-to-j2000")]
async fn api_convert_ruxiu_to_j2000(
    body: web::Json<ConvertCoordRequest>,
) -> impl Responder {
    let r = body.into_inner();
    if !(1..=28).contains(&r.mansion_order) {
        return HttpResponse::BadRequest().json(
            ApiResponse::<()>::err("mansion_order must be 1..=28"));
    }
    // 转换
    let pm_ra  = r.pm_ra_mas.unwrap_or(0.0);
    let pm_dec = r.pm_dec_mas.unwrap_or(0.0);

    // 仅岁差的转换结果
    let ancient_ra = precession::ruxiu_to_ra_ancient(
        r.mansion_order as usize, r.ruxiu_du, r.epoch_yr);
    let ancient_dec = precession::quji_to_dec(r.quji_du);
    let only_prec = precession::ancient_to_j2000_full(
        ancient_ra, ancient_dec, r.epoch_yr, 0.0, 0.0);

    // 含自行的完整转换
    let with_pm = precession::ancient_to_j2000_full(
        ancient_ra, ancient_dec, r.epoch_yr, pm_ra, pm_dec);

    // 古代度的中文表示
    fn ruxiu_to_cn(du: f64) -> String {
        let whole = du.trunc() as i32;
        let frac = du - (whole as f64);
        let frac_str = if (frac - 0.5).abs() < 0.2 { "半" }
            else if (frac - 0.25).abs() < 0.1 { "少" }
            else if (frac - 0.75).abs() < 0.1 { "太" }
            else { "" };
        format!("{}{}度", whole, frac_str)
    }

    let resp = ConvertCoordResponse {
        input_ruxiu_du: r.ruxiu_du,
        input_quji_du: r.quji_du,
        epoch_yr: r.epoch_yr,
        ancient_ra,
        ancient_dec,
        j2000_ra: with_pm.0,
        j2000_dec: with_pm.1,
        only_precession: only_prec,
        with_proper_motion: with_pm,
        ruxiu_raw_cn: ruxiu_to_cn(r.ruxiu_du),
        quji_raw_cn: format!("{}度", r.quji_du.round() as i32),
    };
    HttpResponse::Ok().json(ApiResponse::ok(resp))
}

#[post("/api/trajectory")]
async fn api_trajectory(
    body: web::Json<TrajectoryRequest>,
) -> impl Responder {
    let r = body.into_inner();
    let n = r.n_points.unwrap_or(50);
    let traj = proper_motion_trajectory(
        r.ra_j2000, r.dec_j2000, r.pm_ra_mas, r.pm_dec_mas,
        r.year_start, r.year_end, n,
    );
    // 计算箭头信息
    let arrow = proper_motion_arrow(
        r.ra_j2000, r.dec_j2000, r.pm_ra_mas, r.pm_dec_mas,
        r.year_end - r.year_start,
    );
    HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
        "trajectory": traj,
        "arrow": {
            "end_ra": arrow.0,
            "end_dec": arrow.1,
            "delta_ra_deg": arrow.2,
            "delta_dec_deg": arrow.3,
            "magnitude_arcsec": arrow.4,
        }
    })))
}

#[get("/api/stars/{id}/cross-dynasty")]
async fn api_cross_dynasty(
    data: web::Data<Arc<AppState>>,
    path: web::Path<i64>,
    qs: web::Query<CrossDynastyRequest>,
) -> impl Responder {
    let id = Some(path.into_inner());
    let name = qs.star_name.as_deref();
    match db::get_star_cross_dynasty(&data.pool, id, name).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok_with_count(
            list.clone(),
            list.len() as i64,
        )),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

// ======================================================================
// 彗星 & 客星 API
// ======================================================================

#[get("/api/comets")]
async fn get_comets(
    data: web::Data<Arc<AppState>>,
    qs: web::Query<CometQuery>,
) -> impl Responder {
    match db::list_comets(&data.pool, qs.dynasty_id).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok(list)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[derive(Debug, Deserialize)]
struct CometQuery {
    dynasty_id: Option<i32>,
}

#[get("/api/guest-stars")]
async fn get_guest_stars(
    data: web::Data<Arc<AppState>>,
    qs: web::Query<CometQuery>,
) -> impl Responder {
    match db::list_guest_stars(&data.pool, qs.dynasty_id).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok(list)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[get("/api/guest-stars/{id}")]
async fn get_guest_star(
    data: web::Data<Arc<AppState>>,
    path: web::Path<i64>,
) -> impl Responder {
    match db::get_guest_star_by_id(&data.pool, path.into_inner()).await {
        Ok(Some(g)) => HttpResponse::Ok().json(ApiResponse::ok(g)),
        Ok(None) => HttpResponse::NotFound().json(ApiResponse::<()>::err("Guest star not found")),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

#[get("/api/snr")]
async fn get_snr(data: web::Data<Arc<AppState>>) -> impl Responder {
    match db::list_snr(&data.pool).await {
        Ok(list) => HttpResponse::Ok().json(ApiResponse::ok(list)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    }
}

// ======================================================================
// 贝叶斯匹配 API
// ======================================================================

#[post("/api/match/{guest_id}")]
async fn api_run_match(
    data: web::Data<Arc<AppState>>,
    path: web::Path<i64>,
    qs: web::Query<MatchRequest>,
) -> impl Responder {
    let guest_id = path.into_inner();

    // 1. 获取客星
    let guest = match db::get_guest_for_match(&data.pool, guest_id).await {
        Ok(Some(g)) => g,
        Ok(None) => return HttpResponse::NotFound().json(
            ApiResponse::<()>::err("Guest star not found")),
        Err(e) => return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    };

    // 2. 获取 SNR 目录
    let snrs = match db::list_snr_for_match(&data.pool).await {
        Ok(s) if !s.is_empty() => s,
        Ok(_) => return HttpResponse::InternalServerError().json(
            ApiResponse::<()>::err("No SNR catalog available")),
        Err(e) => return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    };

    // 3. 运行匹配
    let mut result = run_bayesian_match(&guest, &snrs, &data.match_cfg);
    if let Some(k) = qs.top_k {
        result.truncate(k);
    }

    // 4. 可选: 保存到数据库
    let version = env!("CARGO_PKG_VERSION");
    if let Err(e) = db::save_match_result(&data.pool, guest_id, &result, version).await {
        log::warn!("save match result failed: {}", e);
    }

    HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
        "guest": guest,
        "candidates": result,
        "method": {
            "name": "Bayesian Spatial-Temporal Matching",
            "version": version,
            "model": "IAU 2006 precession + Student-t likelihood",
            "n_candidates_evaluated": snrs.len(),
            "n_candidates_returned": result.len(),
        }
    })))
}

#[get("/api/match/{guest_id}")]
async fn api_get_saved_matches(
    data: web::Data<Arc<AppState>>,
    path: web::Path<i64>,
) -> impl Responder {
    let guest_id = path.into_inner();
    // 先查已保存
    let saved = match db::get_saved_matches(&data.pool, guest_id).await {
        Ok(list) => list,
        Err(e) => return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
    };
    if saved.is_empty() {
        // 自动运行匹配: 内联重复逻辑以避免 handler 函数调用问题
        let guest = match db::get_guest_for_match(&data.pool, guest_id).await {
            Ok(Some(g)) => g,
            Ok(None) => return HttpResponse::NotFound().json(
                ApiResponse::<()>::err("Guest star not found")),
            Err(e) => return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
        };
        let snrs = match db::list_snr_for_match(&data.pool).await {
            Ok(s) if !s.is_empty() => s,
            Ok(_) => return HttpResponse::Ok().json(ApiResponse::ok(Vec::<matching::MatchCandidate>::new())),
            Err(e) => return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
        };
        let mut result = matching::run_bayesian_match(&guest, &snrs, &data.match_cfg);
        result.truncate(20);
        let ver = env!("CARGO_PKG_VERSION");
        if let Err(e) = db::save_match_result(&data.pool, guest_id, &result, ver).await {
            log::warn!("save match result failed: {}", e);
        }
        return HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
            "guest": guest,
            "candidates": result,
            "method": {
                "name": "Bayesian Spatial-Temporal Matching",
                "version": ver,
                "model": "IAU 2006 precession + Student-t likelihood",
                "n_candidates_evaluated": snrs.len(),
                "n_candidates_returned": result.len(),
            }
        })));
    }
    HttpResponse::Ok().json(ApiResponse::ok(saved))
}

// ======================================================================
// 主入口
// ======================================================================

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let host = env::var("DB_HOST").unwrap_or_else(|_| "localhost".into());
    let port: u16 = env::var("DB_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(5432);
    let dbname = env::var("DB_NAME").unwrap_or_else(|_| "ancient_star_catalog".into());
    let user = env::var("DB_USER").unwrap_or_else(|_| "postgres".into());
    let password = env::var("DB_PASSWORD").unwrap_or_else(|_| "postgres".into());

    let listen_host = env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let listen_port: u16 = env::var("API_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8080);
    let max_conn: usize = env::var("MAX_DB_CONN").ok().and_then(|v| v.parse().ok()).unwrap_or(16);

    log::info!("Connecting to DB: {}:{}/{} as {}", host, port, dbname, user);
    let pool = db::create_pool(&host, port, &dbname, &user, &password, max_conn)
        .expect("Failed to create DB pool");

    // 测试连接
    {
        let conn = pool.get().await.map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("DB connect failed: {}", e))
        })?;
        conn.query("SELECT 1", &[]).await.map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("DB test failed: {}", e))
        })?;
        log::info!("DB connection OK");
    }

    let state = Arc::new(AppState {
        pool,
        match_cfg: MatchConfig::default(),
    });

    log::info!("Starting API server on {}:{}", listen_host, listen_port);
    HttpServer::new(move || {
        let cors = Cors::permissive(); // 开发环境放宽 CORS
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .service(health)
            .service(get_dynasties)
            .service(get_mansions)
            .service(query_stars)
            .service(get_star)
            .service(api_convert_ruxiu_to_j2000)
            .service(api_trajectory)
            .service(api_cross_dynasty)
            .service(get_comets)
            .service(get_guest_stars)
            .service(get_guest_star)
            .service(get_snr)
            .service(api_run_match)
            .service(api_get_saved_matches)
            // 额外的静态文件服务 (前端)
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
    })
    .bind((listen_host.as_str(), listen_port))?
    .run()
    .await
}
