//! 数据库访问层 (PostgreSQL + PostGIS)

use deadpool_postgres::{Config, Pool, PoolConfig, Runtime, ManagerConfig, RecyclingMethod};
use tokio_postgres::{NoTls, Row, types::ToSql};

use crate::models::*;
use crate::matching::{GuestStarObs, SupernovaRemnant as SnrMatchInput};

/// 数据库连接池
pub type DbPool = Pool;

/// 创建数据库连接池
pub fn create_pool(
    host: &str, port: u16, dbname: &str,
    user: &str, password: &str, max_size: usize,
) -> Result<DbPool, String> {
    let mut cfg = Config::new();
    cfg.host = Some(host.into());
    cfg.port = Some(port);
    cfg.dbname = Some(dbname.into());
    cfg.user = Some(user.into());
    cfg.password = Some(password.into());

    cfg.pool = Some(PoolConfig::new(max_size));
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    cfg.create_pool(Some(Runtime::Tokio1), NoTls)
        .map_err(|e| format!("create pool failed: {}", e))
}

// ======================================================================
// 基础查询辅助
// ======================================================================

fn q(sql: &str) -> String { sql.into() }

// ======================================================================
// 朝代
// ======================================================================

pub async fn list_dynasties(pool: &DbPool) -> Result<Vec<Dynasty>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let rows = client.query(
        "SELECT id, name_cn, name_en, start_year, end_year, canonical_epoch,
                epoch_jd, description, created_at FROM dynasties ORDER BY start_year",
        &[]
    ).await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_to_dynasty).collect())
}

fn row_to_dynasty(row: &Row) -> Dynasty {
    Dynasty {
        id: row.get("id"),
        name_cn: row.get("name_cn"),
        name_en: row.get("name_en"),
        start_year: row.get("start_year"),
        end_year: row.get("end_year"),
        canonical_epoch: row.get("canonical_epoch"),
        epoch_jd: row.get("epoch_jd"),
        description: row.get("description"),
        created_at: row.get("created_at"),
    }
}

// ======================================================================
// 二十八宿
// ======================================================================

pub async fn list_mansions(pool: &DbPool) -> Result<Vec<LunarMansion>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let rows = client.query(
        "SELECT * FROM lunar_mansions ORDER BY mansion_order",
        &[]
    ).await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_to_mansion).collect())
}

fn row_to_mansion(row: &Row) -> LunarMansion {
    LunarMansion {
        id: row.get("id"),
        mansion_order: row.get("mansion_order"),
        name_cn: row.get("name_cn"),
        name_pinyin: row.get("name_pinyin"),
        animal: row.get("animal"),
        azimuth: row.get("azimuth"),
        standard_ra_deg: row.get("standard_ra_deg"),
        extent_deg: row.get("extent_deg"),
        description: row.get("description"),
    }
}

// ======================================================================
// 恒星查询
// ======================================================================

pub async fn query_stars(pool: &DbPool, params: &StarQueryParams)
    -> Result<(Vec<AncientStar>, i64), String>
{
    let mut sql = String::from(
        "SELECT s.*, d.name_cn AS dynasty_name, m.name_cn AS mansion_name,
                m.mansion_order AS mansion_order
         FROM ancient_stars s
         LEFT JOIN dynasties d ON s.dynasty_id = d.id
         LEFT JOIN lunar_mansions m ON s.mansion_id = m.id
         WHERE 1=1"
    );
    let mut psql: Vec<Box<dyn ToSql + Sync>> = Vec::new();
    let mut idx: i32 = 1;

    if let Some(v) = params.dynasty_id {
        sql.push_str(&format!(" AND s.dynasty_id = ${}", idx));
        idx += 1;
        psql.push(Box::new(v));
    }
    if let Some(ref v) = params.dynasty_name {
        sql.push_str(&format!(" AND d.name_cn ILIKE ${}", idx));
        idx += 1;
        psql.push(Box::new(v.clone()));
    }
    if let Some(v) = params.mansion_id {
        sql.push_str(&format!(" AND s.mansion_id = ${}", idx));
        idx += 1;
        psql.push(Box::new(v));
    }
    if let Some(ref v) = params.constellation {
        sql.push_str(&format!(" AND s.constellation ILIKE ${}", idx));
        idx += 1;
        psql.push(Box::new(v.clone()));
    }
    if let Some(ref v) = params.star_name {
        sql.push_str(&format!(" AND (s.star_name_cn ILIKE ${} OR s.star_name_alt ILIKE ${})", idx, idx));
        idx += 1;
        psql.push(Box::new(v.clone()));
    }
    if let Some(v) = params.mag_min {
        sql.push_str(&format!(" AND s.magnitude_num >= ${}", idx));
        idx += 1;
        psql.push(Box::new(v));
    }
    if let Some(v) = params.mag_max {
        sql.push_str(&format!(" AND s.magnitude_num <= ${}", idx));
        idx += 1;
        psql.push(Box::new(v));
    }
    if let Some(v) = params.ra_min {
        sql.push_str(&format!(" AND s.ra_j2000 >= ${}", idx));
        idx += 1;
        psql.push(Box::new(v));
    }
    if let Some(v) = params.ra_max {
        sql.push_str(&format!(" AND s.ra_j2000 <= ${}", idx));
        idx += 1;
        psql.push(Box::new(v));
    }
    if let Some(v) = params.dec_min {
        sql.push_str(&format!(" AND s.dec_j2000 >= ${}", idx));
        idx += 1;
        psql.push(Box::new(v));
    }
    if let Some(v) = params.dec_max {
        sql.push_str(&format!(" AND s.dec_j2000 <= ${}", idx));
        idx += 1;
        psql.push(Box::new(v));
    }
    if let Some(v) = params.quality_min {
        sql.push_str(&format!(" AND s.quality_flag >= ${}", idx));
        idx += 1;
        psql.push(Box::new(v));
    }
    if let Some(ref v) = params.source_book {
        sql.push_str(&format!(" AND s.source_book ILIKE ${}", idx));
        idx += 1;
        psql.push(Box::new(v.clone()));
    }

    let psql_ref: Vec<&(dyn ToSql + Sync)> = psql.iter().map(|b| b.as_ref()).collect();

    let count_sql = format!("SELECT COUNT(*) FROM ({}) q", sql);
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let cnt_row = client.query_one(&count_sql, &psql_ref).await.map_err(|e| e.to_string())?;
    let count: i64 = cnt_row.get(0);

    sql.push_str(" ORDER BY s.magnitude_num NULLS LAST, s.id");
    if let Some(v) = params.limit {
        sql.push_str(&format!(" LIMIT ${}", idx));
        idx += 1;
        psql.push(Box::new(v));
    }
    if let Some(v) = params.offset {
        sql.push_str(&format!(" OFFSET ${}", idx));
        psql.push(Box::new(v));
    }

    let psql_ref2: Vec<&(dyn ToSql + Sync)> = psql.iter().map(|b| b.as_ref()).collect();
    let rows = client.query(&sql, &psql_ref2).await.map_err(|e| e.to_string())?;
    Ok((rows.iter().map(row_to_star).collect(), count))
}

fn row_to_star(row: &Row) -> AncientStar {
    AncientStar {
        id: row.get("id"),
        star_name_cn: row.get("star_name_cn"),
        star_name_alt: row.get("star_name_alt"),
        constellation: row.get("constellation"),
        mansion_id: row.get("mansion_id"),
        dynasty_id: row.get("dynasty_id"),
        source_book: row.get("source_book"),
        source_chapter: row.get("source_chapter"),
        ruxiu_du: row.get("ruxiu_du"),
        quji_du: row.get("quji_du"),
        ruxiu_du_raw: row.get("ruxiu_du_raw"),
        quji_du_raw: row.get("quji_du_raw"),
        magnitude_ancient: row.get("magnitude_ancient"),
        magnitude_num: row.get("magnitude_num"),
        color_desc: row.get("color_desc"),
        color_class: row.get("color_class"),
        ra_j2000: row.get("ra_j2000"),
        dec_j2000: row.get("dec_j2000"),
        ra_ancient_conv: row.get("ra_ancient_conv"),
        dec_ancient_conv: row.get("dec_ancient_conv"),
        proper_motion_ra: row.get("proper_motion_ra"),
        proper_motion_dec: row.get("proper_motion_dec"),
        parallax: row.get("parallax"),
        hipparcos_id: row.get("hipparcos_id"),
        henry_draper_id: row.get("henry_draper_id"),
        quality_flag: row.get("quality_flag"),
        notes: row.get("notes"),
        created_at: row.get("created_at"),
        dynasty_name: row.get("dynasty_name"),
        mansion_name: row.get("mansion_name"),
        mansion_order: row.get("mansion_order"),
    }
}

pub async fn get_star_by_id(pool: &DbPool, id: i64) -> Result<Option<AncientStar>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let row = client.query_opt(
        "SELECT s.*, d.name_cn AS dynasty_name, m.name_cn AS mansion_name,
                m.mansion_order AS mansion_order
         FROM ancient_stars s
         LEFT JOIN dynasties d ON s.dynasty_id = d.id
         LEFT JOIN lunar_mansions m ON s.mansion_id = m.id
         WHERE s.id = $1",
        &[&id]
    ).await.map_err(|e| e.to_string())?;
    Ok(row.map(|r| row_to_star(&r)))
}

pub async fn get_star_cross_dynasty(
    pool: &DbPool,
    star_id: Option<i64>,
    star_name: Option<&str>,
) -> Result<Vec<CrossDynastyPair>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let sql = "
        SELECT
            s1.star_name_cn, s1.constellation,
            d1.id AS d1_id, d1.name_cn AS d1_name, d1.canonical_epoch AS d1_epoch,
            d2.id AS d2_id, d2.name_cn AS d2_name, d2.canonical_epoch AS d2_epoch,
            s1.ruxiu_du AS r1, s1.quji_du AS q1,
            s1.ra_ancient_conv AS ra1, s1.dec_ancient_conv AS dec1,
            s1.magnitude_num AS mag1, s1.color_desc AS col1, s1.source_book AS src1,
            s2.ruxiu_du AS r2, s2.quji_du AS q2,
            s2.ra_ancient_conv AS ra2, s2.dec_ancient_conv AS dec2,
            s2.magnitude_num AS mag2, s2.color_desc AS col2, s2.source_book AS src2,
            (s2.ruxiu_du - s1.ruxiu_du) AS delta_ruxiu,
            (s2.quji_du  - s1.quji_du)  AS delta_quji,
            s1.ra_j2000, s1.dec_j2000
        FROM ancient_stars s1
        JOIN ancient_stars s2
            ON s1.star_name_cn = s2.star_name_cn
            AND s1.dynasty_id < s2.dynasty_id
            AND s1.id != s2.id
        JOIN dynasties d1 ON s1.dynasty_id = d1.id
        JOIN dynasties d2 ON s2.dynasty_id = d2.id
        WHERE 1=1
            AND ($1::bigint IS NULL OR s1.id = $1)
            AND ($2::text   IS NULL OR s1.star_name_cn ILIKE '%' || $2 || '%')
        ORDER BY s1.star_name_cn, d1.start_year
        LIMIT 500";
    let rows = client.query(sql, &[&star_id, &star_name]).await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(|r| CrossDynastyPair {
        star_name: r.get("star_name_cn"),
        constellation: r.get("constellation"),
        dynasty_1: DynastyInfo {
            id: r.get("d1_id"), name: r.get("d1_name"), epoch: r.get("d1_epoch"),
        },
        dynasty_2: DynastyInfo {
            id: r.get("d2_id"), name: r.get("d2_name"), epoch: r.get("d2_epoch"),
        },
        coord_1: CoordAncient {
            ruxiu_du: r.get("r1"), quji_du: r.get("q1"),
            ra_conv: r.get("ra1"), dec_conv: r.get("dec1"),
            magnitude_num: r.get("mag1"), color_desc: r.get("col1"),
            source_book: r.get("src1"),
        },
        coord_2: CoordAncient {
            ruxiu_du: r.get("r2"), quji_du: r.get("q2"),
            ra_conv: r.get("ra2"), dec_conv: r.get("dec2"),
            magnitude_num: r.get("mag2"), color_desc: r.get("col2"),
            source_book: r.get("src2"),
        },
        delta_ruxiu: r.get("delta_ruxiu"),
        delta_quji: r.get("delta_quji"),
        j2000_ra: r.get("ra_j2000"),
        j2000_dec: r.get("dec_j2000"),
    }).collect())
}

// ======================================================================
// 彗星 & 客星
// ======================================================================

pub async fn list_comets(pool: &DbPool, dynasty_id: Option<i32>)
    -> Result<Vec<AncientComet>, String>
{
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let sql = "SELECT c.*, d.name_cn AS dynasty_name
               FROM ancient_comets c
               LEFT JOIN dynasties d ON c.dynasty_id = d.id
               WHERE $1::int IS NULL OR c.dynasty_id = $1
               ORDER BY c.start_jd";
    let rows = client.query(sql, &[&dynasty_id]).await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_to_comet).collect())
}

fn row_to_comet(row: &Row) -> AncientComet {
    AncientComet {
        id: row.get("id"),
        comet_name: row.get("comet_name"),
        appearance_id: row.get("appearance_id"),
        dynasty_id: row.get("dynasty_id"),
        source_book: row.get("source_book"),
        start_date_text: row.get("start_date_text"),
        end_date_text: row.get("end_date_text"),
        start_jd: row.get("start_jd"),
        end_jd: row.get("end_jd"),
        duration_days: row.get("duration_days"),
        ruxiu_du: row.get("ruxiu_du"),
        quji_du: row.get("quji_du"),
        position_desc: row.get("position_desc"),
        brightness_desc: row.get("brightness_desc"),
        estimated_mag: row.get("estimated_mag"),
        tail_length: row.get("tail_length"),
        tail_direction: row.get("tail_direction"),
        ra_apparent: row.get("ra_apparent"),
        dec_apparent: row.get("dec_apparent"),
        notes: row.get("notes"),
        dynasty_name: row.get("dynasty_name"),
    }
}

pub async fn list_guest_stars(pool: &DbPool, dynasty_id: Option<i32>)
    -> Result<Vec<GuestStar>, String>
{
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let sql = "SELECT g.*, d.name_cn AS dynasty_name
               FROM guest_stars g
               LEFT JOIN dynasties d ON g.dynasty_id = d.id
               WHERE $1::int IS NULL OR g.dynasty_id = $1
               ORDER BY g.start_jd";
    let rows = client.query(sql, &[&dynasty_id]).await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_to_guest).collect())
}

fn row_to_guest(row: &Row) -> GuestStar {
    GuestStar {
        id: row.get("id"),
        guest_name: row.get("guest_name"),
        guest_id_code: row.get("guest_id_code"),
        dynasty_id: row.get("dynasty_id"),
        source_book: row.get("source_book"),
        appearance_date: row.get("appearance_date"),
        disappearance_date: row.get("disappearance_date"),
        start_jd: row.get("start_jd"),
        end_jd: row.get("end_jd"),
        visibility_days: row.get("visibility_days"),
        ruxiu_du: row.get("ruxiu_du"),
        quji_du: row.get("quji_du"),
        position_desc: row.get("position_desc"),
        peak_mag: row.get("peak_mag"),
        peak_mag_err: row.get("peak_mag_err"),
        light_curve_desc: row.get("light_curve_desc"),
        color_at_peak: row.get("color_at_peak"),
        ra_est: row.get("ra_est"),
        dec_est: row.get("dec_est"),
        ra_err: row.get("ra_err"),
        dec_err: row.get("dec_err"),
        remnant_candidate: row.get("remnant_candidate"),
        sn_type_hint: row.get("sn_type_hint"),
        notes: row.get("notes"),
        dynasty_name: row.get("dynasty_name"),
    }
}

pub async fn get_guest_star_by_id(pool: &DbPool, id: i64)
    -> Result<Option<GuestStar>, String>
{
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let row = client.query_opt(
        "SELECT g.*, d.name_cn AS dynasty_name
         FROM guest_stars g LEFT JOIN dynasties d ON g.dynasty_id = d.id
         WHERE g.id = $1",
        &[&id]
    ).await.map_err(|e| e.to_string())?;
    Ok(row.map(|r| row_to_guest(&r)))
}

pub async fn list_snr(pool: &DbPool) -> Result<Vec<SupernovaRemnant>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let rows = client.query("SELECT * FROM supernova_remnants ORDER BY age_yr", &[])
        .await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_to_snr).collect())
}

fn row_to_snr(row: &Row) -> SupernovaRemnant {
    SupernovaRemnant {
        id: row.get("id"),
        remnant_name: row.get("remnant_name"),
        alias_names: row.get("alias_names"),
        sn_type: row.get("sn_type"),
        ra_deg: row.get("ra_deg"),
        dec_deg: row.get("dec_deg"),
        ra_err: row.get("ra_err"),
        dec_err: row.get("dec_err"),
        age_yr: row.get("age_yr"),
        age_err: row.get("age_err"),
        explosion_jd: row.get("explosion_jd"),
        explosion_year_est: row.get("explosion_year_est"),
        distance_kpc: row.get("distance_kpc"),
        distance_err: row.get("distance_err"),
        diameter_pc: row.get("diameter_pc"),
        radio_flux_ghz: row.get("radio_flux_ghz"),
        xray_luminosity: row.get("xray_luminosity"),
        gamma_detected: row.get("gamma_detected"),
        expansion_vel: row.get("expansion_vel"),
        spectral_index: row.get("spectral_index"),
        notes: row.get("notes"),
    }
}

// ======================================================================
// 获取匹配用的输入结构
// ======================================================================

pub async fn get_guest_for_match(pool: &DbPool, id: i64)
    -> Result<Option<GuestStarObs>, String>
{
    let g = match get_guest_star_by_id(pool, id).await? {
        Some(g) => g, None => return Ok(None),
    };
    Ok(Some(GuestStarObs {
        id: g.id,
        name: g.guest_name.unwrap_or_default(),
        id_code: g.guest_id_code.unwrap_or_default(),
        ra_est: g.ra_est.unwrap_or(0.0),
        dec_est: g.dec_est.unwrap_or(0.0),
        ra_err: g.ra_err.unwrap_or(0.5),
        dec_err: g.dec_err.unwrap_or(0.5),
        start_jd: g.start_jd.unwrap_or(0.0),
        end_jd: g.end_jd,
        visibility_days: g.visibility_days,
        peak_mag: g.peak_mag.unwrap_or(0.0),
        peak_mag_err: g.peak_mag_err.unwrap_or(0.5),
        sn_type_hint: g.sn_type_hint,
        color_at_peak: g.color_at_peak,
    }))
}

pub async fn list_snr_for_match(pool: &DbPool)
    -> Result<Vec<SnrMatchInput>, String>
{
    let list = list_snr(pool).await?;
    Ok(list.into_iter().map(|s| SnrMatchInput {
        id: s.id,
        name: s.remnant_name,
        sn_type: s.sn_type.unwrap_or_else(|| "II".into()),
        ra_deg: s.ra_deg,
        dec_deg: s.dec_deg,
        ra_err: s.ra_err.unwrap_or(0.5),
        dec_err: s.dec_err.unwrap_or(0.5),
        age_yr: s.age_yr.unwrap_or(1000.0),
        age_err: s.age_err.unwrap_or(200.0),
        explosion_year_est: s.explosion_year_est.unwrap_or(1000.0),
        distance_kpc: s.distance_kpc.unwrap_or(5.0),
        distance_err: s.distance_err.unwrap_or(1.0),
        diameter_pc: s.diameter_pc.unwrap_or(10.0),
        radio_flux_ghz: s.radio_flux_ghz.unwrap_or(50.0),
        xray_luminosity: s.xray_luminosity.unwrap_or(1e36),
        expansion_vel: s.expansion_vel.unwrap_or(500.0),
        gamma_detected: s.gamma_detected.unwrap_or(false),
    }).collect())
}

// ======================================================================
// 保存匹配结果到数据库
// ======================================================================

pub async fn save_match_result(
    pool: &DbPool,
    guest_id: i64,
    matches: &[crate::matching::MatchCandidate],
    method_version: &str,
) -> Result<usize, String>
{
    let client = pool.get().await.map_err(|e| e.to_string())?;
    // 先删除旧结果
    client.execute(
        "DELETE FROM guest_star_matches WHERE guest_star_id = $1",
        &[&guest_id]
    ).await.map_err(|e| e.to_string())?;

    let mut saved = 0;
    for m in matches {
        client.execute(
            "INSERT INTO guest_star_matches
                (guest_star_id, remnant_id,
                 spatial_score, temporal_score, magnitude_score,
                 total_log_posterior, match_probability, rank_within_guest,
                 angular_sep_arcmin, time_delta_yr, bayes_factor, method_version)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            &[
                &m.guest_id, &m.remnant_id,
                &m.log_p_spatial, &m.log_p_temporal, &m.log_p_magnitude,
                &m.log_posterior, &m.match_probability, &m.rank_within_guest,
                &m.angular_sep_arcmin, &m.time_delta_yr, &m.bayes_factor,
                &method_version.to_string(),
            ]
        ).await.map_err(|e| e.to_string())?;
        saved += 1;
    }
    Ok(saved)
}

pub async fn get_saved_matches(pool: &DbPool, guest_id: i64)
    -> Result<Vec<crate::matching::MatchCandidate>, String>
{
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let rows = client.query(
        "SELECT m.*, s.remnant_name, s.sn_type
         FROM guest_star_matches m
         JOIN supernova_remnants s ON m.remnant_id = s.id
         WHERE m.guest_star_id = $1
         ORDER BY m.match_probability DESC, m.rank_within_guest",
        &[&guest_id]
    ).await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(|r| crate::matching::MatchCandidate {
        guest_id: r.get("guest_star_id"),
        remnant_id: r.get("remnant_id"),
        remnant_name: r.get("remnant_name"),
        remnant_type: r.get("sn_type"),
        angular_sep_deg: r.get::<_, Option<f64>>("angular_sep_arcmin").unwrap_or(0.0) / 60.0,
        angular_sep_arcmin: r.get("angular_sep_arcmin"),
        time_delta_yr: r.get("time_delta_yr"),
        log_p_spatial: r.get("spatial_score"),
        log_p_temporal: r.get("temporal_score"),
        log_p_magnitude: r.get("magnitude_score"),
        log_p_lc: 0.0,
        log_prior: 0.0,
        log_posterior: r.get("total_log_posterior"),
        match_probability: r.get("match_probability"),
        rank_within_guest: r.get("rank_within_guest"),
        bayes_factor: r.get("bayes_factor"),
        score_breakdown: crate::matching::ScoreBreakdown::default(),
    }).collect())
}
