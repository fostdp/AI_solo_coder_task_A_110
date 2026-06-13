//! 天文学计算模块入口

pub mod constants;
pub mod precession;

pub use constants::*;

use serde::{Deserialize, Serialize};

/// 自行轨迹采样点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProperMotionPoint {
    pub year: f64,
    pub ra_deg: f64,
    pub dec_deg: f64,
}

/// 计算从 year_start 到 year_end 的自行轨迹 (n_points 采样点)
pub fn proper_motion_trajectory(
    ra_j2000: f64,
    dec_j2000: f64,
    pm_ra_mas: f64,
    pm_dec_mas: f64,
    year_start: f64,
    year_end: f64,
    n_points: usize,
) -> Vec<ProperMotionPoint> {
    let n = n_points.max(2);
    (0..n).map(|i| {
        let t = (i as f64) / ((n - 1) as f64);
        let yr = year_start + (year_end - year_start) * t;
        let (ra, dec) = precession::apply_proper_motion(
            ra_j2000, dec_j2000, pm_ra_mas, pm_dec_mas, yr - 2000.0);
        ProperMotionPoint { year: yr, ra_deg: ra, dec_deg: dec }
    }).collect()
}

/// 计算自行箭头 (J2000 位置, 再推 delta_yr 年后的位置差)
/// 返回 (end_ra, end_dec, delta_ra_deg, delta_dec_deg, magnitude_arcsec)
pub fn proper_motion_arrow(
    ra_j2000: f64,
    dec_j2000: f64,
    pm_ra_mas: f64,
    pm_dec_mas: f64,
    delta_yr: f64,
) -> (f64, f64, f64, f64, f64) {
    let (ra2, dec2) = precession::apply_proper_motion(
        ra_j2000, dec_j2000, pm_ra_mas, pm_dec_mas, delta_yr);
    let dra = norm180(ra2 - ra_j2000);
    let dde = dec2 - dec_j2000;
    let mag_arcsec = (dra * dra + dde * dde).sqrt() * 3600.0;
    (ra2, dec2, dra, dde, mag_arcsec)
}

/// 跨朝代坐标比较: 同一颗恒星在不同朝代的古代坐标转换为 J2000 后比较
pub fn compare_coords_across_epochs(
    ra_j2000: f64,
    dec_j2000: f64,
    pm_ra_mas: f64,
    pm_dec_mas: f64,
    epochs: &[f64],
) -> Vec<(f64, f64, f64)> {
    // 对每个 epoch: J2000 -> 古代 -> 反推回 J2000 (模拟测量再转换过程)
    epochs.iter().map(|&ep| {
        // 正推到古代
        let (ra_ep, dec_ep) = precession::j2000_to_ancient_full(
            ra_j2000, dec_j2000, ep, pm_ra_mas, pm_dec_mas);
        // 反推回 J2000 (相当于 "如果古代人完美测量，再被我们转换回来")
        let (ra_back, dec_back) = precession::ancient_to_j2000_full(
            ra_ep, dec_ep, ep, pm_ra_mas, pm_dec_mas);
        (ep, ra_back, dec_back)
    }).collect()
}
