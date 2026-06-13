//! 数据模型定义 (与 PostgreSQL 表对应 + API 请求/响应结构)

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ======================================================================
// 数据库实体
// ======================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dynasty {
    pub id: i32,
    pub name_cn: String,
    pub name_en: String,
    pub start_year: i32,
    pub end_year: i32,
    pub canonical_epoch: f64,
    pub epoch_jd: f64,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LunarMansion {
    pub id: i32,
    pub mansion_order: i32,
    pub name_cn: String,
    pub name_pinyin: String,
    pub animal: Option<String>,
    pub azimuth: Option<String>,
    pub standard_ra_deg: f64,
    pub extent_deg: f64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AncientStar {
    pub id: i64,
    pub star_name_cn: String,
    pub star_name_alt: Option<String>,
    pub constellation: Option<String>,
    pub mansion_id: Option<i32>,
    pub dynasty_id: i32,
    pub source_book: String,
    pub source_chapter: Option<String>,

    pub ruxiu_du: f64,
    pub quji_du: f64,
    pub ruxiu_du_raw: Option<String>,
    pub quji_du_raw: Option<String>,

    pub magnitude_ancient: Option<String>,
    pub magnitude_num: Option<f64>,
    pub color_desc: Option<String>,
    pub color_class: Option<String>,

    pub ra_j2000: Option<f64>,
    pub dec_j2000: Option<f64>,
    pub ra_ancient_conv: Option<f64>,
    pub dec_ancient_conv: Option<f64>,
    pub proper_motion_ra: Option<f64>,
    pub proper_motion_dec: Option<f64>,
    pub parallax: Option<f64>,
    pub hipparcos_id: Option<i32>,
    pub henry_draper_id: Option<i32>,

    pub quality_flag: Option<i32>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,

    // 关联信息 (JOIN 得到)
    pub dynasty_name: Option<String>,
    pub mansion_name: Option<String>,
    pub mansion_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AncientComet {
    pub id: i64,
    pub comet_name: Option<String>,
    pub appearance_id: Option<String>,
    pub dynasty_id: Option<i32>,
    pub source_book: Option<String>,
    pub start_date_text: Option<String>,
    pub end_date_text: Option<String>,
    pub start_jd: Option<f64>,
    pub end_jd: Option<f64>,
    pub duration_days: Option<i32>,
    pub ruxiu_du: Option<f64>,
    pub quji_du: Option<f64>,
    pub position_desc: Option<String>,
    pub brightness_desc: Option<String>,
    pub estimated_mag: Option<f64>,
    pub tail_length: Option<String>,
    pub tail_direction: Option<String>,
    pub ra_apparent: Option<f64>,
    pub dec_apparent: Option<f64>,
    pub notes: Option<String>,

    pub dynasty_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestStar {
    pub id: i64,
    pub guest_name: Option<String>,
    pub guest_id_code: Option<String>,
    pub dynasty_id: Option<i32>,
    pub source_book: Option<String>,
    pub appearance_date: Option<String>,
    pub disappearance_date: Option<String>,
    pub start_jd: Option<f64>,
    pub end_jd: Option<f64>,
    pub visibility_days: Option<i32>,
    pub ruxiu_du: Option<f64>,
    pub quji_du: Option<f64>,
    pub position_desc: Option<String>,
    pub peak_mag: Option<f64>,
    pub peak_mag_err: Option<f64>,
    pub light_curve_desc: Option<String>,
    pub color_at_peak: Option<String>,
    pub ra_est: Option<f64>,
    pub dec_est: Option<f64>,
    pub ra_err: Option<f64>,
    pub dec_err: Option<f64>,
    pub remnant_candidate: Option<i64>,
    pub sn_type_hint: Option<String>,
    pub notes: Option<String>,

    pub dynasty_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupernovaRemnant {
    pub id: i64,
    pub remnant_name: String,
    pub alias_names: Option<String>,
    pub sn_type: Option<String>,
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub ra_err: Option<f64>,
    pub dec_err: Option<f64>,
    pub age_yr: Option<f64>,
    pub age_err: Option<f64>,
    pub explosion_jd: Option<f64>,
    pub explosion_year_est: Option<f64>,
    pub distance_kpc: Option<f64>,
    pub distance_err: Option<f64>,
    pub diameter_pc: Option<f64>,
    pub radio_flux_ghz: Option<f64>,
    pub xray_luminosity: Option<f64>,
    pub gamma_detected: Option<bool>,
    pub expansion_vel: Option<f64>,
    pub spectral_index: Option<f64>,
    pub notes: Option<String>,
}

// ======================================================================
// API 请求
// ======================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct StarQueryParams {
    pub dynasty_id: Option<i32>,
    pub dynasty_name: Option<String>,
    pub mansion_id: Option<i32>,
    pub constellation: Option<String>,
    pub star_name: Option<String>,
    pub mag_min: Option<f64>,
    pub mag_max: Option<f64>,
    pub ra_min: Option<f64>,
    pub ra_max: Option<f64>,
    pub dec_min: Option<f64>,
    pub dec_max: Option<f64>,
    pub quality_min: Option<i32>,
    pub source_book: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConvertCoordRequest {
    /// 入宿度 (度)
    pub ruxiu_du: f64,
    /// 去极度 (度)
    pub quji_du: f64,
    /// 二十八宿序号 1..=28
    pub mansion_order: i32,
    /// 观测历元 (儒略年)
    pub epoch_yr: f64,
    /// 自行 RA (mas/yr, 可选)
    pub pm_ra_mas: Option<f64>,
    /// 自行 Dec (mas/yr, 可选)
    pub pm_dec_mas: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrajectoryRequest {
    pub ra_j2000: f64,
    pub dec_j2000: f64,
    pub pm_ra_mas: f64,
    pub pm_dec_mas: f64,
    pub year_start: f64,
    pub year_end: f64,
    pub n_points: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CrossDynastyRequest {
    pub star_id: Option<i64>,
    pub star_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchRequest {
    pub guest_id: Option<i64>,
    pub include_snr: Option<bool>,
    pub top_k: Option<usize>,
}

// ======================================================================
// API 响应
// ======================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ConvertCoordResponse {
    pub input_ruxiu_du: f64,
    pub input_quji_du: f64,
    pub epoch_yr: f64,

    pub ancient_ra: f64,
    pub ancient_dec: f64,
    pub j2000_ra: f64,
    pub j2000_dec: f64,

    pub only_precession: (f64, f64),
    pub with_proper_motion: (f64, f64),

    pub ruxiu_raw_cn: String,
    pub quji_raw_cn: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossDynastyPair {
    pub star_name: String,
    pub constellation: Option<String>,
    pub dynasty_1: DynastyInfo,
    pub dynasty_2: DynastyInfo,
    pub coord_1: CoordAncient,
    pub coord_2: CoordAncient,
    pub delta_ruxiu: f64,
    pub delta_quji: f64,
    pub j2000_ra: Option<f64>,
    pub j2000_dec: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DynastyInfo {
    pub id: i32,
    pub name: String,
    pub epoch: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoordAncient {
    pub ruxiu_du: f64,
    pub quji_du: f64,
    pub ra_conv: Option<f64>,
    pub dec_conv: Option<f64>,
    pub magnitude_num: Option<f64>,
    pub color_desc: Option<String>,
    pub source_book: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
    pub count: Option<i64>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, message: "ok".into(),
               data: Some(data), count: None, error: None }
    }
    pub fn ok_with_count(data: T, count: i64) -> Self {
        Self { success: true, message: "ok".into(),
               data: Some(data), count: Some(count), error: None }
    }
    pub fn err(msg: impl Into<String>) -> Self {
        Self { success: false, message: "error".into(),
               data: None, count: None, error: Some(msg.into()) }
    }
}
