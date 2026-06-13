//! 岁差 (Precession)、章动 (Nutation)、自行 (Proper Motion) 模型
//!
//! 参考:
//!  - IAU 2000A/B 简化岁差模型 (Vondrak et al. 2011 近似)
//!  - 使用 Euler Z-X-Z 旋转矩阵
//!  - 章动采用 IAU 2000B (截断的低阶项，对古代数据精度足够)

use super::constants::*;

// ====================================================================
// 自行 (Proper Motion) 修正
// ====================================================================

/// 基于自行的位置外推
///
/// # Arguments
/// * `ra_deg`, `dec_deg` - J2000.0 坐标 (度)
/// * `pm_ra_mas`, `pm_dec_mas` - 自行 (mas/yr)，pm_ra 已含 cos(dec) 因子
/// * `delta_yr` - 时间差 (年，正=未来，负=过去)
///
/// # Returns
/// `(new_ra_deg, new_dec_deg)`
pub fn apply_proper_motion(
    ra_deg: f64, dec_deg: f64,
    pm_ra_mas: f64, pm_dec_mas: f64,
    delta_yr: f64,
) -> (f64, f64) {
    let cos_dec = (dec_deg * DEG2RAD).cos().max(1e-9);
    let d_ra = pm_ra_mas * MAS2DEG * delta_yr / cos_dec;
    let d_dec = pm_dec_mas * MAS2DEG * delta_yr;
    (norm360(ra_deg + d_ra), (dec_deg + d_dec).max(-90.0).min(90.0))
}

/// 反推自行: 给定 ancient_epoch 位置 反推 J2000
pub fn reverse_proper_motion(
    ancient_ra: f64, ancient_dec: f64,
    pm_ra_mas: f64, pm_dec_mas: f64,
    epoch_yr: f64,
) -> (f64, f64) {
    // delta_yr 负的, 从 ancient -> 2000
    apply_proper_motion(ancient_ra, ancient_dec, pm_ra_mas, pm_dec_mas, 2000.0 - epoch_yr)
}

// ====================================================================
// 岁差 Euler 角 (Z-X-Z)
//   采用 IAU 2006 (Vondrak) 模型，系数取自 EXPLANATORY SUPPLEMENT
// ====================================================================

/// 计算从 J2000.0 到 epoch_julian_year 的岁差 Euler 角 (度)
///
/// 输出: `(zeta, z, theta)` —— Z-X-Z Euler 角，按惯例定义
///   R = R3(-zeta) * R1(theta) * R3(-z)
///   用于 J2000 -> epoch:  v_epoch = R * v_j2000
fn precession_angles_deg(epoch_yr: f64) -> (f64, f64, f64) {
    let t = (epoch_yr - 2000.0) / 100.0;  // 距 J2000 儒略世纪
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    // IAU 2006 precession (Vondrak et al. 2011), units: arcseconds -> degree
    // zeta_A
    let zeta = ( 2.650545
               + 2306.083227 * t
               + 0.2988499  * t2
               + 0.01801828 * t3
               - 5.971e-6   * t4
               - 3.1736e-7  * t5) / 3600.0;
    // z_A
    let z    = (-2.650545
               + 2306.077181 * t
               + 1.0927348   * t2
               + 0.01826837  * t3
               - 2.8596e-5   * t4
               - 2.904e-8    * t5) / 3600.0;
    // theta_A
    let theta = (   0.0
               + 2004.191903 * t
               - 0.4294934   * t2
               - 0.04182264  * t3
               - 7.089e-6    * t4
               - 1.274e-7    * t5) / 3600.0;

    (zeta, z, theta)
}

// ====================================================================
// 章动 (Nutation) - IAU 2000B (截断级数, 77 项简化为主要项)
// 对古代 1000+ 年数据，章动 (±15") 可近似，不必精确每一项
// 这里仅采用最主要的 4 项 (月球交点章动 + 太阳章动)
// ====================================================================

fn mean_obliquity_deg(t: f64) -> f64 {
    // epsilon_0 (J2000) = 84381.406"
    let e = 84381.406
          - 46.836769   * t
          - 0.0001831   * t * t
          + 0.00200340  * t * t * t;
    e / 3600.0
}

/// 章动值 (delta_psi, delta_epsilon)，单位度
/// 对于古代数据 (千年尺度) 可忽略此项，仅为保持模型完整性
fn nutation_angles_deg(jd_tt: f64) -> (f64, f64) {
    let t = (jd_tt - JD_J2000) / 36525.0;
    // 四个基本 Delaunay 参数 (度)
    let l    = norm360(134.96298139 + t * (1325.0 * 360.0 + 198.8675605));
    let lp   = norm360(357.52910918 + t * (  99.0 * 360.0 + 359.0502911));
    let f    = norm360( 93.27209062 + t * (1342.0 * 360.0 + 307.1289929));
    let om   = norm360(125.04455501 + t * (  -5.0 * 360.0 - 134.1361849));
    let d    = norm360(297.85019547 + t * (1236.0 * 360.0 + 307.0884006));

    // 弧度版
    let l_r  = l  * DEG2RAD;
    let lp_r = lp * DEG2RAD;
    let f_r  = f  * DEG2RAD;
    let om_r = om * DEG2RAD;
    let d_r  = d  * DEG2RAD;

    // 主要项的系数 (单位: mas) —— 前 5 项主导
    // 格式: (sin(dpsi) coef, cos(dpsi) coef, sin(deps) coef, cos(deps) coef,
    //        l mult, lp mult, f mult, om mult, d mult)
    let terms: [(f64, f64, f64, f64, i32, i32, i32, i32, i32); 5] = [
        (-171996.0, -174.2,  92025.0,   8.9,   0,  0,  0, -2,  0),
        ( -13187.0,   -1.6,   5736.0,  -3.1,   0,  0,  0,  0, -2),
        (  -2274.0,   -0.2,    977.0,  -0.5,  -2,  0,  2, -2,  2),
        (   2062.0,    0.2,   -895.0,   0.5,   0,  0,  0,  0,  2),
        (   1426.0,   -3.4,     54.0,  -0.1,   0,  1,  0,  0,  0),
    ];

    let mut dpsi_mas = 0.0;
    let mut deps_mas = 0.0;
    for (sp, cp, se, ce, ml, mlp, mf, mom, md) in terms {
        let arg = (ml as f64) * l_r
                + (mlp as f64) * lp_r
                + (mf  as f64) * f_r
                + (mom as f64) * om_r
                + (md  as f64) * d_r;
        let (s, c) = (arg.sin(), arg.cos());
        dpsi_mas += sp * s + cp * c;
        deps_mas += se * s + ce * c;
    }
    (dpsi_mas / 3_600_000.0, deps_mas / 3_600_000.0)
}

// ====================================================================
// 旋转矩阵操作 (内联, 避免 nalgebra 依赖)
// ====================================================================

#[inline]
fn rot_z((x, y, z): (f64, f64, f64), a: f64) -> (f64, f64, f64) {
    let (c, s) = (a.cos(), a.sin());
    (c * x - s * y, s * x + c * y, z)
}
#[inline]
fn rot_x((x, y, z): (f64, f64, f64), a: f64) -> (f64, f64, f64) {
    let (c, s) = (a.cos(), a.sin());
    (x, c * y - s * z, s * y + c * z)
}

/// 将 (ra, dec) -> 单位 3-矢量
#[inline]
fn sphere_to_cart(ra_deg: f64, dec_deg: f64) -> (f64, f64, f64) {
    let (ra, dec) = (ra_deg * DEG2RAD, dec_deg * DEG2RAD);
    (dec.cos() * ra.cos(), dec.cos() * ra.sin(), dec.sin())
}

/// 单位 3-矢量 -> (ra, dec) 度
#[inline]
fn cart_to_sphere((x, y, z): (f64, f64, f64)) -> (f64, f64) {
    let r2 = x * x + y * y;
    let ra = if r2 < 1e-24 { 0.0 } else { y.atan2(x) * RAD2DEG };
    let dec = z.atan2(r2.sqrt()) * RAD2DEG;
    (norm360(ra), dec)
}

// ====================================================================
// 对外的岁差转换接口
// ====================================================================

/// J2000  -> 目标 epoch 的岁差 (含近似章动)
pub fn precess_j2000_to_epoch(
    ra_j2000: f64, dec_j2000: f64,
    epoch_julian_yr: f64,
    apply_nutation: bool,
) -> (f64, f64) {
    let (zeta, z, theta) = precession_angles_deg(epoch_julian_yr);

    let mut v = sphere_to_cart(ra_j2000, dec_j2000);
    // 岁差 R = R3(-zeta) R1(theta) R3(-z)
    v = rot_z(v, -z   * DEG2RAD);
    v = rot_x(v,  theta * DEG2RAD);
    v = rot_z(v, -zeta * DEG2RAD);

    let (mut ra, mut dec) = cart_to_sphere(v);

    if apply_nutation {
        let jd = julian_year_to_jd(epoch_julian_yr);
        let (dpsi, deps) = nutation_angles_deg(jd);
        let eps = mean_obliquity_deg((epoch_julian_yr - 2000.0) / 100.0) * DEG2RAD;
        // 简化章动变换 (仅在 GCRS 近似，对古代精度影响可忽略)
        let dec_r = dec * DEG2RAD;
        ra  += dpsi * (eps + dec_r).sin().max(0.0).asin().cos() / dec_r.cos().max(1e-9);
        dec += deps;
        ra = norm360(ra);
    }
    (ra, dec)
}

/// 反向: 古代 epoch 坐标 -> J2000
pub fn precess_epoch_to_j2000(
    ra_epoch: f64, dec_epoch: f64,
    epoch_julian_yr: f64,
    apply_nutation: bool,
) -> (f64, f64) {
    // 先扣掉章动 (若启用)
    let (ra_ep, dec_ep) = if apply_nutation {
        let jd = julian_year_to_jd(epoch_julian_yr);
        let (dpsi, deps) = nutation_angles_deg(jd);
        // 反向章动修正 (近似)
        let eps = mean_obliquity_deg((epoch_julian_yr - 2000.0) / 100.0) * DEG2RAD;
        let dec_r = dec_epoch * DEG2RAD;
        let ra0 = ra_epoch - dpsi * (eps + dec_r).sin().asin().cos() / dec_r.cos().max(1e-9);
        (norm360(ra0), dec_epoch - deps)
    } else {
        (ra_epoch, dec_epoch)
    };

    // 反向岁差: R' = R3(z) R1(-theta) R3(zeta)
    let (zeta, z, theta) = precession_angles_deg(epoch_julian_yr);
    let mut v = sphere_to_cart(ra_ep, dec_ep);
    v = rot_z(v,  zeta * DEG2RAD);
    v = rot_x(v, -theta * DEG2RAD);
    v = rot_z(v,  z   * DEG2RAD);
    cart_to_sphere(v)
}

// ====================================================================
// 完整的 "古代坐标系 -> J2000.0" 流程
// ====================================================================

/// 完整坐标转换: 古代观测坐标 -> J2000 现代坐标
///
/// 步骤:
///   1. 古代观测 (ra_epoch, dec_epoch) 去章动 -> 平均赤道
///   2. 平均赤道 岁差回 J2000
///   3. 自行修正 (正向，从 epoch -> 2000)
pub fn ancient_to_j2000_full(
    ra_epoch: f64, dec_epoch: f64,
    epoch_yr: f64,
    pm_ra_mas: f64, pm_dec_mas: f64,
) -> (f64, f64) {
    // 步骤 1+2
    let (ra_j, dec_j) = precess_epoch_to_j2000(ra_epoch, dec_epoch, epoch_yr, true);
    // 步骤 3: 自行 (正向 ancient -> 2000)
    reverse_proper_motion(ra_j, dec_j, pm_ra_mas, pm_dec_mas, epoch_yr)
}

/// 完整坐标转换: J2000 -> 古代 epoch 视位置
///
/// 步骤:
///   1. 自行修正 (2000 -> ancient)
///   2. J2000 岁差 至 古代
///   3. + 章动
pub fn j2000_to_ancient_full(
    ra_j2000: f64, dec_j2000: f64,
    epoch_yr: f64,
    pm_ra_mas: f64, pm_dec_mas: f64,
) -> (f64, f64) {
    let (ra0, dec0) = apply_proper_motion(ra_j2000, dec_j2000, pm_ra_mas, pm_dec_mas, epoch_yr - 2000.0);
    precess_j2000_to_epoch(ra0, dec0, epoch_yr, true)
}

// ====================================================================
// 入宿度/去极度 <-> 赤经/赤纬 转换
// ====================================================================

/// 去极度 (度) -> 赤纬 (度)
///   quji = 90° - δ
#[inline]
pub fn quji_to_dec(quji_deg: f64) -> f64 {
    90.0 - quji_deg
}

/// 赤纬 (度) -> 去极度 (度)
#[inline]
pub fn dec_to_quji(dec_deg: f64) -> f64 {
    90.0 - dec_deg
}

/// 二十八宿距星 J2000 赤经表 (同 seed_data.py 中 LUNAR_MANSIONS 的 ra 列)
/// 顺序按 mansion_order 1..=28
pub const MANSION_J2000_RAS: [f64; 28] = [
    187.75, 198.20, 209.45, 221.35, 231.55, 240.73, 251.10,  // 角..箕
    262.52, 280.26, 291.10, 303.80, 315.77, 334.25, 349.00,  // 斗..壁
      0.25,  16.45,  30.05,  44.35,  57.25,  73.60,  78.75,  // 奎..参
     90.15, 117.50, 124.80, 138.30, 147.10, 161.60, 175.00,  // 井..轸
];

pub const MANSION_EXTENTS: [f64; 28] = [
    12.0,  9.0, 15.0,  5.0,  5.0, 18.0, 11.0,
    26.0,  8.0, 12.0, 10.0, 17.0, 16.0,  9.0,
    16.0, 12.0, 14.0, 11.0, 16.0,  3.0, 10.0,
    33.0,  4.0, 15.0,  7.0, 18.0, 18.0, 17.0,
];

/// 入宿度 + 距星索引 -> 古代赤经 (度, 古代历元)
///
/// # Arguments
/// * `mansion_order` - 1..=28
/// * `ruxiu_deg`     - 入宿度 (度)
/// * `epoch_yr`      - 观测历元 (儒略年)
///
/// 注意: 这里假设入宿度 = 古代赤经 - 距星古代赤经
///       近似地, 距星赤经也按岁差做同样旋转
pub fn ruxiu_to_ra_ancient(
    mansion_order: usize,
    ruxiu_deg: f64,
    epoch_yr: f64,
) -> f64 {
    let idx = mansion_order.saturating_sub(1).min(27);
    let mra_j2000 = MANSION_J2000_RAS[idx];
    // 距星在 ancient epoch 的赤经 (忽略距星自身的自行，近似即可)
    let (mra_epoch, _) = precess_j2000_to_epoch(mra_j2000, 0.0, epoch_yr, false);
    norm360(mra_epoch + ruxiu_deg)
}

/// 赤经 (古代 epoch) + 该 epoch 的距星赤经 -> (mansion_order, ruxiu)
/// 返回: (mansion_order 1..=28, ruxiu_deg)
pub fn ra_to_ruxiu_ancient(ra_epoch: f64, epoch_yr: f64) -> (usize, f64) {
    // 计算每宿的古代距星赤经
    let mut ancient_ras: Vec<(usize, f64)> = (0..28).map(|i| {
        let mra = MANSION_J2000_RAS[i];
        let (ma, _) = precess_j2000_to_epoch(mra, 0.0, epoch_yr, false);
        (i + 1, ma)
    }).collect();
    ancient_ras.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let ra = norm360(ra_epoch);
    // 找到区间: ancient_ras[i].1  <= ra < ancient_ras[i+1].1
    for i in 0..28 {
        let a1 = ancient_ras[i].1;
        let a2 = ancient_ras[(i + 1) % 28].1;
        let wrap = a2 < a1;
        let in_range = if wrap { ra >= a1 || ra < a2 } else { ra >= a1 && ra < a2 };
        if in_range {
            let ruxiu = if wrap && ra < a1 { ra + 360.0 - a1 } else { ra - a1 };
            return (ancient_ras[i].0, ruxiu);
        }
    }
    (ancient_ras[0].0, 0.0)
}

/// 入宿度/去极度 -> J2000 赤经赤纬 (完整管道)
pub fn ruxiu_quji_to_j2000(
    mansion_order: usize,
    ruxiu_deg: f64,
    quji_deg: f64,
    epoch_yr: f64,
    pm_ra_mas: f64,
    pm_dec_mas: f64,
) -> (f64, f64) {
    let ra_ep  = ruxiu_to_ra_ancient(mansion_order, ruxiu_deg, epoch_yr);
    let dec_ep = quji_to_dec(quji_deg);
    ancient_to_j2000_full(ra_ep, dec_ep, epoch_yr, pm_ra_mas, pm_dec_mas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proper_motion_roundtrip() {
        let (ra0, dec0) = (123.4, 45.6);
        let (ra1, dec1) = apply_proper_motion(ra0, dec0, 100.0, 50.0, 100.0);
        let (ra2, dec2) = reverse_proper_motion(ra1, dec1, 100.0, 50.0, 2000.0 + 100.0);
        assert!((ra0 - ra2).abs() < 1e-5);
        assert!((dec0 - dec2).abs() < 1e-5);
    }

    #[test]
    fn test_precession_roundtrip() {
        let (ra0, dec0) = (83.6331, 22.0145); // Crab Nebula
        let (ra_an, dec_an) = precess_j2000_to_epoch(ra0, dec0, 1054.0, true);
        let (ra_back, dec_back) = precess_epoch_to_j2000(ra_an, dec_an, 1054.0, true);
        assert!((ra0 - ra_back).abs() < 1e-3);  // < 3.6"
        assert!((dec0 - dec_back).abs() < 1e-3);
    }

    #[test]
    fn test_quji_dec_roundtrip() {
        assert!((quji_to_dec(60.0) - 30.0).abs() < 1e-9);
        assert!((dec_to_quji(30.0) - 60.0).abs() < 1e-9);
    }
}
