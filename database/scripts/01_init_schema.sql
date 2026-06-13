-- ============================================================
-- 古代星表数据数字化与现代天体物理验证系统
-- PostgreSQL + PostGIS 数据库初始化脚本 v0.2
-- ============================================================
--
-- v0.2 更新 (三个修复):
--   1. ancient_stars 表新增 color_temp_k 字段 (有效温度 K)
--   2. 新增 idx_snr_galactic GIN 索引 (银道坐标空间查询)
--   3. guest_star_matches 表新增 log_prior 字段 (银河系先验对数)
--

-- 扩展
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- ============================================================
-- 朝代表
-- ============================================================
CREATE TABLE IF NOT EXISTS dynasties (
    id SERIAL PRIMARY KEY,
    name_cn VARCHAR(32) NOT NULL,
    name_pinyin VARCHAR(64),
    start_year INTEGER NOT NULL,
    end_year INTEGER NOT NULL,
    canonical_epoch DOUBLE PRECISION NOT NULL,
    color_hex VARCHAR(16),
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- 二十八宿表
-- ============================================================
CREATE TABLE IF NOT EXISTS lunar_mansions (
    id SERIAL PRIMARY KEY,
    mansion_order INTEGER NOT NULL,
    name_cn VARCHAR(16) NOT NULL,
    name_pinyin VARCHAR(32),
    ruxiu_width_deg DOUBLE PRECISION,
    ra_start_deg DOUBLE PRECISION,
    ra_end_deg DOUBLE PRECISION,
    dec_mid_deg DOUBLE PRECISION,
    description TEXT
);

-- ============================================================
-- 古代恒星表
-- 修复 3: 新增 color_temp_k 字段 (有效温度 K)
-- ============================================================
CREATE TABLE IF NOT EXISTS ancient_stars (
    id SERIAL PRIMARY KEY,
    star_id_code VARCHAR(64) UNIQUE NOT NULL,
    dynasty_id INTEGER REFERENCES dynasties(id),
    mansion_id INTEGER REFERENCES lunar_mansions(id),
    star_name_cn VARCHAR(64),
    star_name_alt VARCHAR(64),
    constellation VARCHAR(64),
    ruxiu_du DOUBLE PRECISION,
    quji_du DOUBLE PRECISION,
    ra_ancient_conv DOUBLE PRECISION,
    dec_ancient_conv DOUBLE PRECISION,
    ra_j2000 DOUBLE PRECISION,
    dec_j2000 DOUBLE PRECISION,
    magnitude_ancient VARCHAR(32),
    magnitude_num DOUBLE PRECISION,
    color_desc VARCHAR(32),
    color_class VARCHAR(16),
    color_temp_k DOUBLE PRECISION,  -- ★ 修复3: 有效温度 (K)
    proper_motion_ra DOUBLE PRECISION,   -- mas/yr
    proper_motion_dec DOUBLE PRECISION,  -- mas/yr
    parallax DOUBLE PRECISION,           -- mas
    source_book VARCHAR(64),
    quality_flag INTEGER DEFAULT 1,
    notes TEXT,
    modern_hd_id INTEGER,
    cross_match_id INTEGER,
    geom_sphere GEOMETRY(Point, 4326),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_stars_dynasty ON ancient_stars(dynasty_id);
CREATE INDEX IF NOT EXISTS idx_stars_mansion ON ancient_stars(mansion_id);
CREATE INDEX IF NOT EXISTS idx_stars_name ON ancient_stars(star_name_cn);
CREATE INDEX IF NOT EXISTS idx_stars_geom ON ancient_stars USING GIST (geom_sphere);
CREATE INDEX IF NOT EXISTS idx_stars_mag ON ancient_stars(magnitude_num);
CREATE INDEX IF NOT EXISTS idx_stars_temp ON ancient_stars(color_temp_k);

-- 自动更新 geom_sphere 触发器
CREATE OR REPLACE FUNCTION update_star_geom() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.ra_j2000 IS NOT NULL AND NEW.dec_j2000 IS NOT NULL THEN
        NEW.geom_sphere := ST_SetSRID(ST_MakePoint(NEW.ra_j2000, NEW.dec_j2000), 4326);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_star_geom ON ancient_stars;
CREATE TRIGGER trg_star_geom
    BEFORE INSERT OR UPDATE ON ancient_stars
    FOR EACH ROW EXECUTE FUNCTION update_star_geom();

-- ============================================================
-- 古代彗星表
-- ============================================================
CREATE TABLE IF NOT EXISTS ancient_comets (
    id SERIAL PRIMARY KEY,
    comet_id_code VARCHAR(64) UNIQUE NOT NULL,
    dynasty_id INTEGER REFERENCES dynasties(id),
    year_ancient VARCHAR(64),
    year_ce DOUBLE PRECISION,
    month_ancient INTEGER,
    day_ancient INTEGER,
    ruxiu_du DOUBLE PRECISION,
    quji_du DOUBLE PRECISION,
    ra_deg DOUBLE PRECISION,
    dec_deg DOUBLE PRECISION,
    magnitude DOUBLE PRECISION,
    color_desc VARCHAR(32),
    tail_direction VARCHAR(32),
    tail_length DOUBLE PRECISION,
    duration_days INTEGER,
    description TEXT,
    position_desc TEXT,
    source_book VARCHAR(64),
    quality_flag INTEGER DEFAULT 1,
    geom_sphere GEOMETRY(Point, 4326)
);

CREATE INDEX IF NOT EXISTS idx_comets_dynasty ON ancient_comets(dynasty_id);
CREATE INDEX IF NOT EXISTS idx_comets_geom ON ancient_comets USING GIST (geom_sphere);

-- ============================================================
-- 客星 (超新星候选) 表
-- ============================================================
CREATE TABLE IF NOT EXISTS guest_stars (
    id SERIAL PRIMARY KEY,
    guest_id_code VARCHAR(64) UNIQUE NOT NULL,
    dynasty_id INTEGER REFERENCES dynasties(id),
    star_name VARCHAR(64),
    year_ancient INTEGER NOT NULL,
    year_ce DOUBLE PRECISION NOT NULL,
    month_ancient INTEGER,
    day_ancient INTEGER,
    ruxiu_du DOUBLE PRECISION,
    quji_du DOUBLE PRECISION,
    ra_deg DOUBLE PRECISION,
    dec_deg DOUBLE PRECISION,
    ra_err DOUBLE PRECISION DEFAULT 1.0,   -- 位置不确定度 (度)
    dec_err DOUBLE PRECISION DEFAULT 1.0,
    peak_mag DOUBLE PRECISION,
    peak_mag_err DOUBLE PRECISION DEFAULT 0.5,
    visibility_days INTEGER,
    lightcurve_type VARCHAR(16) DEFAULT 'II',
    description TEXT,
    position_desc TEXT,
    source_book VARCHAR(64),
    matched_snr_id INTEGER,
    match_confidence DOUBLE PRECISION,
    geom_sphere GEOMETRY(Point, 4326),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_guests_dynasty ON guest_stars(dynasty_id);
CREATE INDEX IF NOT EXISTS idx_guests_geom ON guest_stars USING GIST (geom_sphere);
CREATE INDEX IF NOT EXISTS idx_guests_year ON guest_stars(year_ce);

-- 自动更新 geom
CREATE OR REPLACE FUNCTION update_guest_geom() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.ra_deg IS NOT NULL AND NEW.dec_deg IS NOT NULL THEN
        NEW.geom_sphere := ST_SetSRID(ST_MakePoint(NEW.ra_deg, NEW.dec_deg), 4326);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_guest_geom ON guest_stars;
CREATE TRIGGER trg_guest_geom
    BEFORE INSERT OR UPDATE ON guest_stars
    FOR EACH ROW EXECUTE FUNCTION update_guest_geom();

-- ============================================================
-- 超新星遗迹 (SNR) 表
-- 修复 2: 新增 gal_l, gal_b 银道坐标 (用于银河系分布先验)
-- ============================================================
CREATE TABLE IF NOT EXISTS supernova_remnants (
    id SERIAL PRIMARY KEY,
    remnant_name VARCHAR(128) UNIQUE NOT NULL,
    sn_type VARCHAR(16) DEFAULT 'II',
    ra_deg DOUBLE PRECISION NOT NULL,
    dec_deg DOUBLE PRECISION NOT NULL,
    gal_l DOUBLE PRECISION,     -- ★ 修复2: 银经
    gal_b DOUBLE PRECISION,     -- ★ 修复2: 银纬
    age_yr DOUBLE PRECISION,
    age_err_yr DOUBLE PRECISION DEFAULT 500.0,
    distance_kpc DOUBLE PRECISION,
    distance_err DOUBLE PRECISION,
    diameter_pc DOUBLE PRECISION,
    radio_flux_ghz DOUBLE PRECISION,
    xray_luminosity DOUBLE PRECISION,
    gamma_detected BOOLEAN DEFAULT FALSE,
    historical_sn_id INTEGER REFERENCES guest_stars(id),
    geom_sphere GEOMETRY(Point, 4326),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_snr_geom ON supernova_remnants USING GIST (geom_sphere);
CREATE INDEX IF NOT EXISTS idx_snr_age ON supernova_remnants(age_yr);
CREATE INDEX IF NOT EXISTS idx_snr_type ON supernova_remnants(sn_type);
CREATE INDEX IF NOT EXISTS idx_snr_galactic ON supernova_remnants(gal_l, gal_b);  -- ★ 修复2

-- 触发器: 自动计算银道坐标
CREATE OR REPLACE FUNCTION calc_snr_galactic() RETURNS TRIGGER AS $$
DECLARE
    ra_r  DOUBLE PRECISION;
    dec_r DOUBLE PRECISION;
    ngp_ra_r  DOUBLE PRECISION := RADIANS(192.8595);
    ngp_dec_r DOUBLE PRECISION := RADIANS(27.1284);
    lon_cp_r  DOUBLE PRECISION := RADIANS(122.932);
    sin_b DOUBLE PRECISION;
    b_r   DOUBLE PRECISION;
    y_r   DOUBLE PRECISION;
    x_r   DOUBLE PRECISION;
    l_r   DOUBLE PRECISION;
BEGIN
    IF NEW.ra_deg IS NOT NULL AND NEW.dec_deg IS NOT NULL THEN
        ra_r  := RADIANS(NEW.ra_deg);
        dec_r := RADIANS(NEW.dec_deg);

        sin_b := SIN(dec_r) * SIN(ngp_dec_r)
               + COS(dec_r) * COS(ngp_dec_r) * COS(ra_r - ngp_ra_r);
        b_r := ASIN(sin_b);

        y_r := SIN(dec_r) * COS(ngp_dec_r)
             - COS(dec_r) * SIN(ngp_dec_r) * COS(ra_r - ngp_ra_r);
        x_r := -COS(dec_r) * SIN(ra_r - ngp_ra_r);
        l_r := ATAN2(y_r, x_r) + lon_cp_r;

        NEW.gal_l := DEGREES(l_r);
        IF NEW.gal_l < 0 THEN NEW.gal_l := NEW.gal_l + 360; END IF;
        IF NEW.gal_l >= 360 THEN NEW.gal_l := NEW.gal_l - 360; END IF;
        NEW.gal_b := DEGREES(b_r);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_snr_galactic ON supernova_remnants;
CREATE TRIGGER trg_snr_galactic
    BEFORE INSERT OR UPDATE OF ra_deg, dec_deg ON supernova_remnants
    FOR EACH ROW EXECUTE FUNCTION calc_snr_galactic();

-- ============================================================
-- 客星 - 超新星遗迹 匹配结果表
-- 修复 2: 新增 log_prior 字段 (记录银河系先验贡献)
-- ============================================================
CREATE TABLE IF NOT EXISTS guest_star_matches (
    id SERIAL PRIMARY KEY,
    guest_id INTEGER REFERENCES guest_stars(id),
    remnant_id INTEGER REFERENCES supernova_remnants(id),
    rank_within_guest INTEGER,
    match_probability DOUBLE PRECISION,
    log_posterior DOUBLE PRECISION,
    log_likelihood DOUBLE PRECISION,
    log_prior DOUBLE PRECISION,    -- ★ 修复2: 先验对数 (银河系分布模型)
    bayes_factor DOUBLE PRECISION,
    angular_sep_arcmin DOUBLE PRECISION,
    time_delta_yr DOUBLE PRECISION,
    spatial_score DOUBLE PRECISION,
    temporal_score DOUBLE PRECISION,
    magnitude_score DOUBLE PRECISION,
    lightcurve_score DOUBLE PRECISION,
    model_version VARCHAR(32),
    match_method VARCHAR(32),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(guest_id, remnant_id)
);

CREATE INDEX IF NOT EXISTS idx_matches_guest ON guest_star_matches(guest_id, rank_within_guest);
CREATE INDEX IF NOT EXISTS idx_matches_remnant ON guest_star_matches(remnant_id);
CREATE INDEX IF NOT EXISTS idx_matches_prob ON guest_star_matches(match_probability DESC);

-- ============================================================
-- 天球角距离函数 (Haversine)
-- ============================================================
CREATE OR REPLACE FUNCTION angular_distance_deg(
    ra1 DOUBLE PRECISION, dec1 DOUBLE PRECISION,
    ra2 DOUBLE PRECISION, dec2 DOUBLE PRECISION
) RETURNS DOUBLE PRECISION AS $$
DECLARE
    d_ra  DOUBLE PRECISION := RADIANS(ra1 - ra2);
    d_dec DOUBLE PRECISION := RADIANS(dec1 - dec2);
    a     DOUBLE PRECISION;
BEGIN
    a := POWER(SIN(d_dec / 2), 2)
       + COS(RADIANS(dec1)) * COS(RADIANS(dec2)) * POWER(SIN(d_ra / 2), 2);
    RETURN DEGREES(2 * ASIN(SQRT(a)));
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- ============================================================
-- 视图: 跨朝代恒星对比
-- ============================================================
CREATE OR REPLACE VIEW v_star_cross_dynasty AS
SELECT
    s1.id AS star_id_1,
    s2.id AS star_id_2,
    s1.star_name_cn AS star_name,
    d1.id AS dynasty_id_1,
    d1.name_cn AS dynasty_1,
    d2.id AS dynasty_id_2,
    d2.name_cn AS dynasty_2,
    (d2.canonical_epoch - d1.canonical_epoch) AS delta_yr,
    s1.ruxiu_du - s2.ruxiu_du AS delta_ruxiu,
    s1.quji_du - s2.quji_du AS delta_quji,
    angular_distance_deg(s1.ra_j2000, s1.dec_j2000, s2.ra_j2000, s2.dec_j2000) AS delta_ang_deg
FROM ancient_stars s1
JOIN ancient_stars s2
    ON s1.star_name_cn = s2.star_name_cn
    AND s1.dynasty_id < s2.dynasty_id
JOIN dynasties d1 ON s1.dynasty_id = d1.id
JOIN dynasties d2 ON s2.dynasty_id = d2.id
WHERE s1.ruxiu_du IS NOT NULL
  AND s2.ruxiu_du IS NOT NULL
ORDER BY s1.star_name_cn, d1.start_year;

-- ============================================================
-- 数据质量统计视图
-- ============================================================
CREATE OR REPLACE VIEW v_star_quality_stats AS
SELECT
    d.name_cn AS dynasty_name,
    COUNT(*) AS star_count,
    AVG(s.quality_flag) AS avg_quality,
    SUM(CASE WHEN s.ra_j2000 IS NOT NULL THEN 1 ELSE 0 END) AS matched_count,
    ROUND(AVG(s.magnitude_num)::numeric, 2) AS avg_magnitude,
    ROUND(AVG(s.proper_motion_ra)::numeric, 2) AS avg_pm_ra
FROM ancient_stars s
JOIN dynasties d ON s.dynasty_id = d.id
GROUP BY d.id, d.name_cn
ORDER BY d.start_year;

-- ============================================================
-- 初始数据: 朝代
-- ============================================================
INSERT INTO dynasties (name_cn, name_pinyin, start_year, end_year, canonical_epoch, color_hex)
VALUES
    ('汉', 'Han', -206, 220, -50.0, '#c03030'),
    ('三国', 'Three Kingdoms', 220, 280, 250.0, '#c08040'),
    ('晋', 'Jin', 266, 420, 340.0, '#608040'),
    ('南北朝', 'North-South', 420, 589, 500.0, '#4080c0'),
    ('隋', 'Sui', 581, 618, 600.0, '#8040a0'),
    ('唐', 'Tang', 618, 907, 750.0, '#e0a020'),
    ('五代', 'Five Dynasties', 907, 960, 930.0, '#606060'),
    ('宋', 'Song', 960, 1279, 1100.0, '#7040b0'),
    ('辽', 'Liao', 907, 1125, 1000.0, '#308080'),
    ('金', 'Jin_er', 1115, 1234, 1170.0, '#a06040'),
    ('元', 'Yuan', 1271, 1368, 1320.0, '#3070c0'),
    ('明', 'Ming', 1368, 1644, 1500.0, '#c04030'),
    ('清', 'Qing', 1636, 1912, 1750.0, '#308040')
ON CONFLICT DO NOTHING;

-- ============================================================
-- 初始数据: 二十八宿
--   宿度参考 <步天歌> 均值, 按西汉时期平均分配
-- ============================================================
INSERT INTO lunar_mansions (mansion_order, name_cn, name_pinyin, ruxiu_width_deg, ra_start_deg, ra_end_deg)
VALUES
    (1,  '角', 'Jiao',   12.0, 189.5, 201.5),
    (2,  '亢', 'Kang',    9.0, 201.5, 210.5),
    (3,  '氐', 'Di',     15.0, 210.5, 225.5),
    (4,  '房', 'Fang',    5.0, 225.5, 230.5),
    (5,  '心', 'Xin',     5.0, 230.5, 235.5),
    (6,  '尾', 'Wei',    18.0, 235.5, 253.5),
    (7,  '箕', 'Ji',     11.0, 253.5, 264.5),
    (8,  '斗', 'Dou',    26.0, 264.5, 290.5),
    (9,  '牛', 'Niu',     8.0, 290.5, 298.5),
    (10, '女', 'Nü',     12.0, 298.5, 310.5),
    (11, '虚', 'Xu',     10.0, 310.5, 320.5),
    (12, '危', 'Wei',    17.0, 320.5, 337.5),
    (13, '室', 'Shi',    16.0, 337.5, 353.5),
    (14, '壁', 'Bi',      9.0, 353.5, 362.5),
    (15, '奎', 'Kui',    16.0, 362.5,  18.5),
    (16, '娄', 'Lou',    12.0,  18.5,  30.5),
    (17, '胃', 'Wei',    14.0,  30.5,  44.5),
    (18, '昴', 'Mao',    11.0,  44.5,  55.5),
    (19, '毕', 'Bi',     16.0,  55.5,  71.5),
    (20, '觜', 'Zi',      3.0,  71.5,  74.5),
    (21, '参', 'Shen',    9.0,  74.5,  83.5),
    (22, '井', 'Jing',   33.0,  83.5, 116.5),
    (23, '鬼', 'Gui',     4.0, 116.5, 120.5),
    (24, '柳', 'Liu',    15.0, 120.5, 135.5),
    (25, '星', 'Xing',    7.0, 135.5, 142.5),
    (26, '张', 'Zhang',   6.0, 142.5, 148.5),
    (27, '翼', 'Yi',     18.0, 148.5, 166.5),
    (28, '轸', 'Zhen',    5.0, 166.5, 171.5)
ON CONFLICT DO NOTHING;

-- 更新 mansion_id 外键引用 (让古代星星宿关联更准确)
-- 注: 实际导入数据时使用 mansion_order 做 JOIN
