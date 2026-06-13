//! 配置加载模块
//! 从 JSON 文件加载岁差系数、匹配参数等模型参数
//! 替代原硬编码

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecessionConfig {
    pub model_name: String,
    pub version: String,
    pub omega_a_t0_arcsec: f64,
    pub j2000_jd: f64,
    pub julian_century_days: f64,
    pub psi_a_coeffs_mas: Vec<f64>,
    pub omega_a_coeffs_mas: Vec<f64>,
    pub chi_a_coeffs_mas: Vec<f64>,
    pub zeta_a_coeffs_arcsec: Vec<f64>,
    pub theta_a_coeffs_arcsec: Vec<f64>,
    pub z_a_coeffs_arcsec: Vec<f64>,
    pub iau2000b_nutation_terms: Vec<NutationTerm>,
    pub nutation_delaunay_rates_arcsec_per_cy: DelaunayRates,
    pub nutation_delaunay_constants_arcsec: DelaunayConstants,
    pub proper_motion: ProperMotionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct NutationTerm {
    pub l: f64,
    pub lp: f64,
    pub F: f64,
    pub D: f64,
    pub Om: f64,
    pub dpsi_sin: f64,
    pub deps_sin: f64,
    pub dpsi_cos: f64,
    pub deps_cos: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct DelaunayRates {
    pub l: f64,
    pub lp: f64,
    pub F: f64,
    pub D: f64,
    pub Om: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct DelaunayConstants {
    pub l: f64,
    pub lp: f64,
    pub F: f64,
    pub D: f64,
    pub Om: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProperMotionConfig {
    pub default_pm_ra_mas_per_yr: f64,
    pub default_pm_dec_mas_per_yr: f64,
    pub cos_dec_eps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    pub model_name: String,
    pub version: String,
    pub default_config: MatchDefaultConfig,
    pub galactic_prior: GalacticPriorConfig,
    pub likelihood: LikelihoodConfig,
    pub channel_buffer_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDefaultConfig {
    pub spatial_sigma_scale: f64,
    pub temporal_sigma_yr: f64,
    pub magnitude_sigma: f64,
    pub lightcurve_sigma_days: f64,
    pub min_sep_arcmin: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalacticPriorConfig {
    pub r_sun_kpc: f64,
    pub r_disk_scale_kpc: f64,
    pub z_disk_scale_kpc: f64,
    pub prior_floor_log: f64,
    pub ngp_ra_deg: f64,
    pub ngp_dec_deg: f64,
    pub lon_cp_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LikelihoodConfig {
    pub spatial: SpatialLikelihoodConfig,
    pub temporal: TemporalLikelihoodConfig,
    pub magnitude: MagnitudeLikelihoodConfig,
    pub lightcurve: LightcurveLikelihoodConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialLikelihoodConfig {
    pub snr_position_uncertainty_deg: f64,
    pub cauchy_scale_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalLikelihoodConfig {
    pub nu: f64,
    pub min_sigma_yr: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagnitudeLikelihoodConfig {
    pub nu: f64,
    pub min_sigma: f64,
    pub default_extinction_av: f64,
    pub default_extinction_err: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightcurveLikelihoodConfig {
    pub nu: f64,
    pub min_sigma_days: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogConfig {
    pub model_name: String,
    pub version: String,
    pub data_sources: Vec<DataSource>,
    pub cleaning_rules: CleaningRules,
    pub batch_size: usize,
    pub parallelism: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub name: String,
    pub dynasty: String,
    pub epoch_year: f64,
    pub quality_weight: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleaningRules {
    pub max_ruxiu_du: f64,
    pub max_quji_du: f64,
    pub min_magnitude: f64,
    pub max_magnitude: f64,
    pub valid_color_descriptions: Vec<String>,
    pub default_color_temp_k: f64,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub precession: PrecessionConfig,
    pub matching: MatchingConfig,
    pub catalog: CatalogConfig,
}

impl AppConfig {
    pub fn load(config_dir: &str) -> Result<Self, String> {
        let prec = load_json::<PrecessionConfig>(&format!("{}/precession.json", config_dir))?;
        let match_cfg = load_json::<MatchingConfig>(&format!("{}/matching.json", config_dir))?;
        let cat = load_json::<CatalogConfig>(&format!("{}/catalog.json", config_dir))?;
        Ok(Self {
            precession: prec,
            matching: match_cfg,
            catalog: cat,
        })
    }
}

fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("Config file not found: {}", path));
    }
    let content = fs::read_to_string(p).map_err(|e| format!("Read {}: {}", path, e))?;
    serde_json::from_str(&content).map_err(|e| format!("Parse {}: {}", path, e))
}
