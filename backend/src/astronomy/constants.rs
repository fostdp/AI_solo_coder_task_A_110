//! 天文常量与数学工具
//! 所有角度计算统一使用:
//!   - 函数输入输出: 度 (degree)
//!   - 内部计算: 弧度 (radian)

use std::f64::consts::PI;

pub const DEG2RAD: f64 = PI / 180.0;
pub const RAD2DEG: f64 = 180.0 / PI;
pub const ARCSEC2DEG: f64 = 1.0 / 3600.0;
pub const MAS2DEG: f64 = 1.0 / 3_600_000.0;
pub const DEG_PER_CENTURY: f64 = 36525.0 / 365.25;

/// J2000.0 儒略日
pub const JD_J2000: f64 = 2_451_545.0;

/// 简化: normalize angle to [0, 360)
#[inline]
pub fn norm360(deg: f64) -> f64 {
    let mut r = deg % 360.0;
    if r < 0.0 { r += 360.0; }
    r
}

/// normalize angle to [-180, 180)
#[inline]
pub fn norm180(deg: f64) -> f64 {
    let mut r = deg % 360.0;
    if r > 180.0 { r -= 360.0; }
    if r < -180.0 { r += 360.0; }
    r
}

/// clamp to -1..=1 for safe acos
#[inline]
pub fn clamp1(x: f64) -> f64 {
    x.max(-1.0).min(1.0)
}

/// 天球角距离 (Haversine formula), 输出度
pub fn angular_distance_deg(ra1: f64, dec1: f64, ra2: f64, dec2: f64) -> f64 {
    let (r1, d1, r2, d2) = (ra1 * DEG2RAD, dec1 * DEG2RAD,
                             ra2 * DEG2RAD, dec2 * DEG2RAD);
    let dra = r2 - r1;
    let cos_d = d1.sin() * d2.sin() + d1.cos() * d2.cos() * dra.cos();
    clamp1(cos_d).acos() * RAD2DEG
}

/// 儒略年 -> 儒略日
pub fn julian_year_to_jd(year: f64) -> f64 {
    JD_J2000 + (year - 2000.0) * 365.25
}

/// 儒略日 -> 儒略年 (J2000-based 近似)
pub fn jd_to_julian_year(jd: f64) -> f64 {
    2000.0 + (jd - JD_J2000) / 365.25
}
