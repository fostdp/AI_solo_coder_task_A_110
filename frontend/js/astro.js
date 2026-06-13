/* ============================================================
 * 天文计算工具 (前端精简版)
 * 用于前端展示时的快速坐标换算、距离计算
 * ============================================================ */

const DEG2RAD = Math.PI / 180.0;
const RAD2DEG = 180.0 / Math.PI;
const MAS2DEG = 1.0 / 3600000.0;
const JD_J2000 = 2451545.0;

// ===================== 数学工具 =====================

function norm360(deg) {
    let r = deg % 360;
    if (r < 0) r += 360;
    return r;
}

function norm180(deg) {
    let r = deg % 360;
    if (r > 180) r -= 360;
    if (r < -180) r += 360;
    return r;
}

function clamp(x, lo, hi) {
    return Math.max(lo, Math.min(hi, x));
}

// ===================== 坐标转换 =====================

/**
 * 天球坐标 -> 三维单位矢量 (用于 Three.js 球面)
 * ra, dec 单位: 度
 * 返回: {x, y, z}
 */
function sphereToCartesian(raDeg, decDeg, radius = 1) {
    const ra = raDeg * DEG2RAD;
    const dec = decDeg * DEG2RAD;
    return {
        x: radius * Math.cos(dec) * Math.cos(ra),
        y: radius * Math.sin(dec),
        z: radius * Math.cos(dec) * Math.sin(ra),
    };
}

/**
 * 三维矢量 -> 天球坐标 (度)
 */
function cartesianToSphere(x, y, z) {
    const r = Math.sqrt(x*x + y*y + z*z);
    if (r < 1e-12) return { ra: 0, dec: 0 };
    const dec = Math.asin(y / r) * RAD2DEG;
    let ra = Math.atan2(z, x) * RAD2DEG;
    return { ra: norm360(ra), dec: dec };
}

/**
 * 天球角距离 (度)
 */
function angularDistance(ra1, dec1, ra2, dec2) {
    const r1 = ra1 * DEG2RAD;
    const d1 = dec1 * DEG2RAD;
    const r2 = ra2 * DEG2RAD;
    const d2 = dec2 * DEG2RAD;
    const cosD = Math.sin(d1) * Math.sin(d2)
               + Math.cos(d1) * Math.cos(d2) * Math.cos(r2 - r1);
    return Math.acos(clamp(cosD, -1, 1)) * RAD2DEG;
}

// ===================== 简化岁差模型 (前端仅用于粗略展示) =====================
// 精确转换由后端完成

function approximatePrecession(raDeg, decDeg, fromYear, toYear) {
    // 线性近似: 岁差速率 ~50.3"/yr 沿黄道
    // 近似为 RA 方向 ~ 46"/yr (因黄道倾角而异，这里简化)
    const dt = toYear - fromYear;
    const dRa = 46.0 / 3600.0 * dt / Math.cos(decDeg * DEG2RAD);
    const dDec = 20.0 / 3600.0 * dt; // 粗略值
    return {
        ra: norm360(raDeg + dRa),
        dec: clamp(decDeg + dDec, -90, 90),
    };
}

// ===================== 自行轨迹计算 (前端快速近似) =====================

function properMotionTrajectory(raJ2000, decJ2000, pmRaMas, pmDecMas,
                                 yearStart, yearEnd, nPoints = 50) {
    const points = [];
    for (let i = 0; i < nPoints; i++) {
        const t = i / (nPoints - 1);
        const yr = yearStart + (yearEnd - yearStart) * t;
        const delta = yr - 2000.0;
        const cosDec = Math.cos(decJ2000 * DEG2RAD) || 1e-9;
        const dRa = (pmRaMas * MAS2DEG * delta) / cosDec;
        const dDec = pmDecMas * MAS2DEG * delta;
        points.push({
            year: yr,
            ra_deg: norm360(raJ2000 + dRa),
            dec_deg: clamp(decJ2000 + dDec, -90, 90),
        });
    }
    return points;
}

// ===================== 颜色映射 =====================

/**
 * 古代星等和颜色描述 -> 前端颜色
 */
const COLOR_MAP = {
    '白':   '#f5f7ff',
    '青':   '#a0c8ff',
    '赤':   '#ffa070',
    '黄':   '#fff0c0',
    '苍':   '#c0d8ff',
    '黑':   '#808080',
    '白赤': '#ffe0d0',
    'default': '#ffffff',
};

function getStarColor(colorDesc, magnitude) {
    const base = COLOR_MAP[colorDesc] || COLOR_MAP.default;
    // 根据星等调整亮度
    const mag = magnitude !== undefined ? magnitude : 4;
    const brightness = clamp(1.0 - (mag - 0) / 8.0, 0.3, 1.2);
    return adjustBrightness(base, brightness);
}

function adjustBrightness(hex, factor) {
    const r = parseInt(hex.slice(1,3), 16);
    const g = parseInt(hex.slice(3,5), 16);
    const b = parseInt(hex.slice(5,7), 16);
    const nr = Math.min(255, Math.floor(r * factor));
    const ng = Math.min(255, Math.floor(g * factor));
    const nb = Math.min(255, Math.floor(b * factor));
    return '#' + nr.toString(16).padStart(2,'0')
               + ng.toString(16).padStart(2,'0')
               + nb.toString(16).padStart(2,'0');
}

/**
 * 星等 -> 像素大小
 */
function magToSize(mag) {
    // 6 等星 ~1px, 0 等星 ~5px, -2 等 ~10px
    const base = 6.5;
    const size = Math.max(0.5, (base - mag) * 0.9);
    return size;
}

// ===================== 朝代风格映射 =====================

const DYNASTY_STYLES = {
    '西汉': 'han',
    '东汉': 'han',
    '三国': 'other',
    '晋':   'jin',
    '南北朝':'other',
    '隋':   'sui',
    '唐':   'tang',
    '五代': 'other',
    '宋':   'song',
    '元':   'yuan',
    '明':   'ming',
    '清':   'qing',
};

// ===================== 儒略年/儒略日互转 =====================

function julianYearToJd(year) {
    return JD_J2000 + (year - 2000.0) * 365.25;
}

function jdToJulianYear(jd) {
    return 2000.0 + (jd - JD_J2000) / 365.25;
}

// ===================== 暴露到全局 =====================

window.Astro = {
    DEG2RAD, RAD2DEG, MAS2DEG,
    norm360, norm180, clamp,
    sphereToCartesian, cartesianToSphere,
    angularDistance,
    approximatePrecession,
    properMotionTrajectory,
    getStarColor, adjustBrightness, magToSize,
    DYNASTY_STYLES,
    julianYearToJd, jdToJulianYear,
    COLOR_MAP,
};
