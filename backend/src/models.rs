//! 数据模型定义

use serde::{Deserialize, Serialize};

// ============================================================
// 数据库实体
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dynasty {
    pub id: i64,
    pub name_cn: String,
    pub name_pinyin: String,
    pub start_year: i32,
    pub end_year: i32,
    pub canonical_epoch: f64,
    pub color_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LunarMansion {
    pub id: i64,
    pub mansion_order: i32,
    pub name_cn: String,
    pub name_pinyin: String,
    pub ruxiu_width_deg: f64,
    pub ra_start_deg: f64,
    pub ra_end_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AncientStar {
    pub id: i64,
    pub star_id_code: String,
    pub dynasty_id: i64,
    pub mansion_id: Option<i64>,
    pub star_name_cn: String,
    pub star_name_alt: Option<String>,
    pub constellation: Option<String>,
    pub ruxiu_du: Option<f64>,
    pub quji_du: Option<f64>,
    pub ra_ancient_conv: Option<f64>,
    pub dec_ancient_conv: Option<f64>,
    pub ra_j2000: Option<f64>,
    pub dec_j2000: Option<f64>,
    pub magnitude_ancient: Option<String>,
    pub magnitude_num: Option<f64>,
    pub color_desc: Option<String>,
    pub color_class: Option<String>,
    pub color_temp_k: Option<f64>,
    pub proper_motion_ra: Option<f64>,
    pub proper_motion_dec: Option<f64>,
    pub parallax: Option<f64>,
    pub source_book: Option<String>,
    pub quality_flag: i32,
    pub notes: Option<String>,
    pub modern_hd_id: Option<i64>,
    pub cross_match_id: Option<i64>,
    // JOIN 字段
    pub dynasty_name: Option<String>,
    pub mansion_name: Option<String>,
    pub mansion_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AncientComet {
    pub id: i64,
    pub comet_id_code: String,
    pub dynasty_id: i64,
    pub year_ancient: Option<String>,
    pub year_ce: Option<f64>,
    pub ruxiu_du: Option<f64>,
    pub quji_du: Option<f64>,
    pub ra_deg: Option<f64>,
    pub dec_deg: Option<f64>,
    pub magnitude: Option<f64>,
    pub color_desc: Option<String>,
    pub tail_direction: Option<String>,
    pub tail_length: Option<f64>,
    pub duration_days: Option<i32>,
    pub description: Option<String>,
    pub dynasty_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestStar {
    pub id: i64,
    pub guest_id_code: String,
    pub dynasty_id: i64,
    pub star_name: Option<String>,
    pub year_ancient: i32,
    pub year_ce: f64,
    pub month_ancient: Option<i32>,
    pub day_ancient: Option<i32>,
    pub ruxiu_du: Option<f64>,
    pub quji_du: Option<f64>,
    pub ra_deg: Option<f64>,
    pub dec_deg: Option<f64>,
    pub ra_err: f64,
    pub dec_err: f64,
    pub peak_mag: f64,
    pub peak_mag_err: f64,
    pub visibility_days: Option<i32>,
    pub lightcurve_type: String,
    pub description: Option<String>,
    pub position_desc: Option<String>,
    pub dynasty_name: Option<String>,
    pub matched_snr_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupernovaRemnantDb {
    pub id: i64,
    pub remnant_name: String,
    pub sn_type: String,
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub gal_l: Option<f64>,
    pub gal_b: Option<f64>,
    pub age_yr: f64,
    pub age_err_yr: f64,
    pub distance_kpc: f64,
    pub distance_err: f64,
    pub diameter_pc: Option<f64>,
    pub radio_flux_ghz: Option<f64>,
    pub xray_luminosity: Option<f64>,
    pub gamma_detected: bool,
    pub historical_sn_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub id: i64,
    pub guest_id: i64,
    pub remnant_id: i64,
    pub remnant_name: String,
    pub remnant_type: String,
    pub rank_within_guest: i32,
    pub match_probability: f64,
    pub log_posterior: f64,
    pub log_likelihood: f64,
    pub log_prior: f64,
    pub bayes_factor: f64,
    pub angular_sep_arcmin: f64,
    pub time_delta_yr: f64,
    pub spatial_score: f64,
    pub temporal_score: f64,
    pub magnitude_score: f64,
    pub lightcurve_score: f64,
    pub model_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDynastyPair {
    pub dynasty_1: DynastyInfo,
    pub dynasty_2: DynastyInfo,
    pub star_id_1: i64,
    pub star_id_2: i64,
    pub delta_ruxiu: f64,
    pub delta_quji: f64,
    pub delta_ra: f64,
    pub delta_dec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynastyInfo {
    pub id: i64,
    pub name: String,
    pub year: i32,
}

// ============================================================
// API 请求 / 响应
// ============================================================

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StarQueryParams {
    pub dynasty_id: Option<i64>,
    pub dynasty_name: Option<String>,
    pub mansion_id: Option<i64>,
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
pub struct MatchRequest {
    pub guest_id: Option<i64>,
    pub include_snr: Option<bool>,
    pub top_k: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
    pub total: Option<i64>,
    pub version: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: 0,
            message: "success".into(),
            data: Some(data),
            total: None,
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    pub fn ok_with_count(data: T, total: i64) -> Self {
        Self {
            code: 0,
            message: "success".into(),
            data: Some(data),
            total: Some(total),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    pub fn err<S: Into<String>>(msg: S) -> Self {
        Self {
            code: -1,
            message: msg.into(),
            data: None,
            total: None,
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}
