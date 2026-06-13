//! 客星 (Guest Star) 与超新星遗迹 (SNR) 的贝叶斯匹配模型
//!
//! 概率模型:
//!   后验概率 P(M | D) ∝ P(D | M) · P(M)
//!
//!   D = 观测数据 {位置(RA,Dec,误差), 时间(出现年份), 峰值星等, 光变曲线描述}
//!   M = 匹配假设 (某客星 对应 某遗迹)
//!
//!   似然分解:
//!   P(D|M) = P_pos(D|M) · P_time(D|M) · P_mag(D|M) · P_lc(D|M)
//!
//!   其中:
//!   - P_pos : 2D 高斯 (天球角距离投影近似)
//!   - P_time: 高斯 + 长尾 (遗迹年龄估计有不确定性)
//!   - P_mag : 星等-距离-类型关系 (近似对数正态)
//!   - 先验 P(M): 均匀 (所有候选对先验等权)
//!
//! 参考:
//!   - Green & Stephenson (2003), "Astronomical Evidence for Supernovae"
//!   - Schaefer (1996), "Proper Motion Studies of Possible Supernova Remnants"

use std::f64::consts::{LN_2, PI};
use serde::{Deserialize, Serialize};

use crate::astronomy::constants::{angular_distance_deg, jd_to_julian_year};

// ====================================================================
// 数据结构
// ====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestStarObs {
    pub id: i64,
    pub name: String,
    pub id_code: String,
    pub ra_est: f64,       // 估算赤经 (J2000, 度)
    pub dec_est: f64,      // 估算赤纬 (J2000, 度)
    pub ra_err: f64,       // RA 误差 (度)
    pub dec_err: f64,      // Dec 误差 (度)
    pub start_jd: f64,     // 出现时间 (儒略日)
    pub end_jd: Option<f64>,
    pub visibility_days: Option<i32>,
    pub peak_mag: f64,     // 峰值目视星等
    pub peak_mag_err: f64, // 星等误差
    pub sn_type_hint: Option<String>,
    pub color_at_peak: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupernovaRemnant {
    pub id: i64,
    pub name: String,
    pub sn_type: String,
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub ra_err: f64,
    pub dec_err: f64,
    pub age_yr: f64,       // 遗迹年龄 (年, 当前=2000)
    pub age_err: f64,      // 年龄误差
    pub explosion_year_est: f64,  // 估计爆炸年份 (儒略年, 如 1054)
    pub distance_kpc: f64,
    pub distance_err: f64,
    pub diameter_pc: f64,
    pub radio_flux_ghz: f64,
    pub xray_luminosity: f64,
    pub expansion_vel: f64,
    pub gamma_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCandidate {
    pub guest_id: i64,
    pub remnant_id: i64,
    pub remnant_name: String,
    pub remnant_type: String,

    pub angular_sep_deg: f64,
    pub angular_sep_arcmin: f64,
    pub time_delta_yr: f64,

    pub log_p_spatial: f64,
    pub log_p_temporal: f64,
    pub log_p_magnitude: f64,
    pub log_p_lc: f64,
    pub log_prior: f64,

    pub log_posterior: f64,
    pub match_probability: f64,  // 归一化概率 (所有候选内)
    pub rank_within_guest: i32,
    pub bayes_factor: f64,

    pub score_breakdown: ScoreBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreBreakdown {
    pub spatial_match: String,   // 等级: Excellent/Good/Fair/Poor
    pub temporal_match: String,
    pub magnitude_match: String,
    pub overall_confidence: String,
}

// ====================================================================
// 超参数 (可配置)
// ====================================================================

pub struct MatchConfig {
    pub spatial_sigma_scale: f64,    // 空间误差放大因子
    pub temporal_sigma_scale: f64,   // 时间误差放大因子
    pub mag_sigma: f64,              // 星等偏差 sigma
    pub spatial_exclusion_deg: f64,  // 超过此角距离直接排除
    pub temporal_exclusion_yr: f64,  // 超过此时间差直接排除
    pub log10_bayes_thr: f64,        // log10(BF) 阈值用于等级评定
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            spatial_sigma_scale: 1.5,
            temporal_sigma_scale: 2.0,
            mag_sigma: 1.5,
            spatial_exclusion_deg: 8.0,
            temporal_exclusion_yr: 2500.0,
            log10_bayes_thr: 2.0,
        }
    }
}

// ====================================================================
// 核心概率计算
// ====================================================================

/// 2D 高斯对数似然 (独立 RA/Dec 近似, 坐标用度)
fn log_gaussian_2d(dx: f64, dy: f64, sx: f64, sy: f64) -> f64 {
    let sx = sx.max(1e-6);
    let sy = sy.max(1e-6);
    let k = 2.0 * PI * sx * sy;
    -0.5 * (dx * dx / (sx * sx) + dy * dy / (sy * sy)) - 0.5 * k.ln()
}

/// 带长尾的高斯 (student-t-like, 用于时间分布，容许异常值)
fn log_student_t(x: f64, mu: f64, sigma: f64, nu: f64) -> f64 {
    let d = (x - mu) / sigma.max(1e-6);
    // log pdf, proportional (忽略归一化常数差异不影响排序)
    let log_term = (1.0 + d * d / nu).ln();
    -0.5 * (nu + 1.0) * log_term
}

/// 空间似然: P(RA_Guest, Dec_Guest | SNR_RA, SNR_Dec, sigma_tot)
fn spatial_log_likelihood(g: &GuestStarObs, s: &SupernovaRemnant, cfg: &MatchConfig) -> (f64, f64) {
    let sep = angular_distance_deg(g.ra_est, g.dec_est, s.ra_deg, s.dec_deg);

    // 合并误差 (几何合成 + 缩放因子)
    let s_ra  = cfg.spatial_sigma_scale * (g.ra_err * g.ra_err  + s.ra_err  * s.ra_err ).sqrt();
    let s_dec = cfg.spatial_sigma_scale * (g.dec_err * g.dec_err + s.dec_err * s.dec_err).sqrt();

    // 在小角度近似下, 以 SNR 为中心, 用 RA/Dec 差近似 (cos Dec 修正)
    let cos_dec = (g.dec_est.to_radians()).cos().max(1e-9);
    let dra = (g.ra_est - s.ra_deg) * cos_dec;
    let dde = g.dec_est - s.dec_deg;

    // 加一点点 2D Cauchy 长尾混合 (权重 10%), 避免完全排除边缘候选
    let w_core = 0.9;
    let w_tail = 0.1;
    let core = log_gaussian_2d(dra, dde, s_ra, s_dec);
    let tail = log_student_t(sep, 0.0, (s_ra * s_dec).sqrt().max(0.1), 3.0);
    let ll = (w_core * core.exp() + w_tail * tail.exp()).ln();

    (ll, sep)
}

/// 时间似然: P(Year_Guest | Age_SNR)
///   SNR 年龄 = 2000 - explosion_year_est
///   Guest 观测 year 应 ≈ SNR explosion_year
fn temporal_log_likelihood(g: &GuestStarObs, s: &SupernovaRemnant, cfg: &MatchConfig) -> (f64, f64) {
    let guest_year = jd_to_julian_year(g.start_jd);
    let delta_yr = guest_year - s.explosion_year_est;

    // 合并误差: SNR age_err + 客星日期误差 (给 50 年默认)
    let guest_date_err = 30.0;  // 年, 近似古代记录日期不确定性
    let sigma_tot = cfg.temporal_sigma_scale * (
        s.age_err * s.age_err + guest_date_err * guest_date_err
    ).sqrt();

    // Student-t (nu=4), 容许一些系统偏差
    let ll = log_student_t(delta_yr, 0.0, sigma_tot.max(5.0), 4.0);
    (ll, delta_yr)
}

/// 星等似然: 基于绝对星等 + 距离的预测
///
///   M_abs 范围:
///     Ia 型超新星: -19.3 ± 0.3
///     II 型: -17 ± 1.5
///     Ib/Ic: -18 ± 1
///   距离模数: μ = 5 log10(d/10pc) = 5 log10(d_kpc) + 10
///   预测视星等: m_pred = M_abs + μ + 消光 (A_v ~ 1-2 均值 1.5)
fn magnitude_log_likelihood(g: &GuestStarObs, s: &SupernovaRemnant, cfg: &MatchConfig) -> f64 {
    let (m_abs_mean, m_abs_err): (f64, f64) = match s.sn_type.as_str() {
        "Ia" => (-19.3, 0.3),
        "Ib" | "Ic" | "Ibc" => (-17.8, 0.8),
        "II" | "IIP" | "IIL" | "IIn" => (-16.8, 1.2),
        _ => (-17.5, 1.5),
    };

    let dist_pc = s.distance_kpc * 1000.0;
    let dist_err_pc = s.distance_err * 1000.0;
    let mu = if dist_pc > 10.0 { 5.0 * (dist_pc / 10.0).log10() } else { 0.0 };
    let mu_err: f64 = if dist_pc > 10.0 {
        (5.0 / (dist_pc * LN_2)) * (dist_err_pc / dist_pc).ln_1p().abs()
    } else { 1.0 };

    let av: f64 = 1.5;
    let av_err: f64 = 0.8;

    let m_pred = m_abs_mean + mu + av;
    let m_sigma = (
        cfg.mag_sigma.powi(2)
        + m_abs_err.powi(2)
        + mu_err.powi(2)
        + av_err.powi(2)
        + g.peak_mag_err.powi(2)
    ).sqrt();

    let bonus: f64 = match (s.gamma_detected, s.radio_flux_ghz > 50.0, s.xray_luminosity > 1e36) {
        (true, _, _)   => 0.15,
        (_, true, true) => 0.10,
        (_, true, _)   => 0.05,
        _ => 0.0,
    };

    let x = g.peak_mag - m_pred;
    log_student_t(x, 0.0, m_sigma.max(0.5), 5.0) + bonus.ln_1p()
}

/// 光变曲线似然 (近似，基于 visibility_days)
///   - Ia 型: ~20-60 天下降 2 mag, 典型可见 100-300 天(肉眼)
///   - II 型: 缓降, 可见 200-600 天
///   - 特亮 (mag < -4): 白天可见, 总持续时间长
fn lightcurve_log_likelihood(g: &GuestStarObs, s: &SupernovaRemnant) -> f64 {
    let vis_days = g.visibility_days.unwrap_or(180) as f64;
    let (vis_mean, vis_sigma) = match s.sn_type.as_str() {
        "Ia" => (180.0, 60.0),
        "II" | "IIL" => (250.0, 100.0),
        "IIP" => (300.0, 120.0),
        _ => (200.0, 120.0),
    };
    // 峰值超亮 -> 额外持续 bonus
    let bonus_days = if g.peak_mag < -4.0 { 100.0 } else { 0.0 };
    log_student_t(vis_days, vis_mean + bonus_days, vis_sigma, 4.0)
}

// ====================================================================
// 主匹配函数
// ====================================================================

pub fn run_bayesian_match(
    guest: &GuestStarObs,
    remnants: &[SupernovaRemnant],
    cfg: &MatchConfig,
) -> Vec<MatchCandidate> {
    // 先筛选候选 (快速剔除明显不相关的)
    let mut candidates: Vec<MatchCandidate> = remnants.iter()
        .filter(|s| {
            let sep = angular_distance_deg(guest.ra_est, guest.dec_est, s.ra_deg, s.dec_deg);
            if sep > cfg.spatial_exclusion_deg { return false; }
            let guest_year = jd_to_julian_year(guest.start_jd);
            if (guest_year - s.explosion_year_est).abs() > cfg.temporal_exclusion_yr {
                return false;
            }
            true
        })
        .map(|s| {
            let (log_p_spatial, sep) = spatial_log_likelihood(guest, s, cfg);
            let (log_p_temporal, time_delta_yr) = temporal_log_likelihood(guest, s, cfg);
            let log_p_magnitude = magnitude_log_likelihood(guest, s, cfg);
            let log_p_lc = lightcurve_log_likelihood(guest, s);

            // 先验: 均匀 (相对 SNR 数量; 这里取常数不影响排序)
            let log_prior = 0.0;

            let log_post = log_p_spatial + log_p_temporal + log_p_magnitude + log_p_lc + log_prior;

            MatchCandidate {
                guest_id: guest.id,
                remnant_id: s.id,
                remnant_name: s.name.clone(),
                remnant_type: s.sn_type.clone(),
                angular_sep_deg: sep,
                angular_sep_arcmin: sep * 60.0,
                time_delta_yr,
                log_p_spatial,
                log_p_temporal,
                log_p_magnitude,
                log_p_lc,
                log_prior,
                log_posterior: log_post,
                match_probability: 0.0,
                rank_within_guest: 0,
                bayes_factor: 0.0,
                score_breakdown: ScoreBreakdown::default(),
            }
        })
        .collect();

    // 按 log_posterior 降序
    candidates.sort_by(|a, b| b.log_posterior.partial_cmp(&a.log_posterior).unwrap());

    // 归一化: softmax 到概率
    if !candidates.is_empty() {
        let max_log = candidates[0].log_posterior;
        let weights: Vec<f64> = candidates.iter().map(|c| (c.log_posterior - max_log).exp()).collect();
        let sum_w: f64 = weights.iter().sum();
        if sum_w > 0.0 {
            for (c, w) in candidates.iter_mut().zip(weights.iter()) {
                c.match_probability = w / sum_w;
            }
        }
        // Bayes factor: K = P(D|M1) / P(D|M2) —— 取 第一/第二 (或 第一/平均)
        if candidates.len() >= 2 {
            let logbf = candidates[0].log_posterior - candidates[1].log_posterior;
            candidates[0].bayes_factor = logbf.exp().min(1e9);
            for i in 1..candidates.len() {
                candidates[i].bayes_factor =
                    (candidates[i].log_posterior - candidates[0].log_posterior).exp();
            }
        }
        // 排名
        for (i, c) in candidates.iter_mut().enumerate() {
            c.rank_within_guest = (i + 1) as i32;
            // 等级打分
            c.score_breakdown = make_breakdown(c, cfg);
        }
    }
    candidates
}

fn make_breakdown(c: &MatchCandidate, cfg: &MatchConfig) -> ScoreBreakdown {
    // 空间
    let sp = if c.angular_sep_arcmin < 30.0 { "Excellent" }
        else if c.angular_sep_arcmin < 120.0 { "Good" }
        else if c.angular_sep_deg < 3.0 { "Fair" }
        else { "Poor" }.to_string();
    let tm = if c.time_delta_yr.abs() < 100.0 { "Excellent" }
        else if c.time_delta_yr.abs() < 400.0 { "Good" }
        else if c.time_delta_yr.abs() < 1000.0 { "Fair" }
        else { "Poor" }.to_string();
    // 星等: 简单根据 log_p_magnitude
    let mg = if c.log_p_magnitude > -1.0 { "Excellent" }
        else if c.log_p_magnitude > -2.5 { "Good" }
        else if c.log_p_magnitude > -5.0 { "Fair" }
        else { "Poor" }.to_string();
    // 总置信度
    let overall = if c.match_probability > 0.7
        || (c.match_probability > 0.4 && c.bayes_factor.log10() > cfg.log10_bayes_thr)
        { "High" }
        else if c.match_probability > 0.2 { "Medium" }
        else if c.match_probability > 0.05 { "Low" }
        else { "Very Low" }.to_string();
    ScoreBreakdown {
        spatial_match: sp,
        temporal_match: tm,
        magnitude_match: mg,
        overall_confidence: overall,
    }
}

// ====================================================================
// 批处理接口
// ====================================================================

pub fn run_bayesian_match_all(
    guests: &[GuestStarObs],
    remnants: &[SupernovaRemnant],
    cfg: &MatchConfig,
) -> Vec<(i64, Vec<MatchCandidate>)> {
    guests.iter().map(|g| (g.id, run_bayesian_match(g, remnants, cfg))).collect()
}

// ====================================================================
// 测试: 以蟹状星云 (SN 1054) 为基准
// ====================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crab_nebula_should_match_sn1054() {
        let sn1054 = GuestStarObs {
            id: 1,
            name: "天关客星".into(),
            id_code: "SN-1054".into(),
            ra_est: 83.6, dec_est: 22.0,
            ra_err: 0.5, dec_err: 0.5,
            start_jd: 2_136_455.0, // 1054-07-04
            end_jd: None,
            visibility_days: Some(650),
            peak_mag: -6.0, peak_mag_err: 0.5,
            sn_type_hint: None,
            color_at_peak: Some("赤".into()),
        };
        let crab = SupernovaRemnant {
            id: 99,
            name: "蟹状星云".into(),
            sn_type: "II".into(),
            ra_deg: 83.6331, dec_deg: 22.0145,
            ra_err: 0.01, dec_err: 0.01,
            age_yr: 970.0, age_err: 30.0,
            explosion_year_est: 1054.0,
            distance_kpc: 2.0, distance_err: 0.3,
            diameter_pc: 3.4, radio_flux_ghz: 200.0,
            xray_luminosity: 1e37,
            expansion_vel: 1400.0, gamma_detected: true,
        };
        let other = SupernovaRemnant {
            id: 100, name: "G327.6+14.6".into(), sn_type: "Ia".into(),
            ra_deg: 225.0, dec_deg: -42.0,
            ra_err: 0.2, dec_err: 0.2,
            age_yr: 1020.0, age_err: 40.0,
            explosion_year_est: 1006.0,
            distance_kpc: 2.2, distance_err: 0.3,
            diameter_pc: 30.0, radio_flux_ghz: 170.0,
            xray_luminosity: 1e36,
            expansion_vel: 2900.0, gamma_detected: false,
        };
        let cfg = MatchConfig::default();
        let res = run_bayesian_match(&sn1054, &[crab, other], &cfg);
        assert!(!res.is_empty());
        assert_eq!(res[0].rank_within_guest, 1);
        assert_eq!(res[0].remnant_name, "蟹状星云");
        assert!(res[0].angular_sep_arcmin < 30.0);
        assert!(res[0].time_delta_yr.abs() < 100.0);
        // 蟹状星云概率应显著高于 SN 1006
        assert!(res[0].match_probability > res.last().unwrap().match_probability);
    }

    #[test]
    fn test_log_gaussian_basic() {
        let at_center = log_gaussian_2d(0.0, 0.0, 1.0, 1.0);
        let far = log_gaussian_2d(5.0, 5.0, 1.0, 1.0);
        assert!(at_center > far);
    }
}
