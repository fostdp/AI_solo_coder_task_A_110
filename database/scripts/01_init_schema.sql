-- ============================================================
-- 古代星表数据数字化与现代天体物理验证系统 - 数据库初始化脚本
-- Database: Ancient Star Catalog Digitization System
-- ============================================================

-- 启用PostGIS扩展
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS postgis_topology;

-- ============================================================
-- 1. 朝代表 (Dynasties)
-- ============================================================
CREATE TABLE IF NOT EXISTS dynasties (
    id              SERIAL PRIMARY KEY,
    name_cn         VARCHAR(32) NOT NULL,
    name_en         VARCHAR(64) NOT NULL,
    start_year      INTEGER NOT NULL,
    end_year        INTEGER NOT NULL,
    canonical_epoch DOUBLE PRECISION NOT NULL,
    epoch_jd        DOUBLE PRECISION NOT NULL,
    description     TEXT,
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE dynasties IS '中国历史朝代信息，用于星表记录的时间归属';
COMMENT ON COLUMN dynasties.canonical_epoch IS '朝代标准历元（儒略年，用于岁差计算）';
COMMENT ON COLUMN dynasties.epoch_jd IS '标准历元对应的儒略日';

CREATE INDEX IF NOT EXISTS idx_dynasties_time_range ON dynasties USING gist (
    int4range(start_year, end_year, '[]')
);

-- ============================================================
-- 2. 二十八宿表 (28 Lunar Mansions / Xiù)
-- ============================================================
CREATE TABLE IF NOT EXISTS lunar_mansions (
    id              SERIAL PRIMARY KEY,
    mansion_order   INTEGER NOT NULL UNIQUE,
    name_cn         VARCHAR(8) NOT NULL UNIQUE,
    name_pinyin     VARCHAR(32) NOT NULL,
    animal          VARCHAR(16),
    azimuth         VARCHAR(8),
    standard_ra_deg DOUBLE PRECISION NOT NULL,
    extent_deg      DOUBLE PRECISION NOT NULL,
    description     TEXT
);

COMMENT ON TABLE lunar_mansions IS '二十八宿基本参数，用于入宿度转换';
COMMENT ON COLUMN lunar_mansions.standard_ra_deg IS '该宿距星在J2000.0历元下的标准赤经（度）';
COMMENT ON COLUMN lunar_mansions.extent_deg IS '该宿的跨度（古度，约等于现代度）';

-- ============================================================
-- 3. 恒星记录表 (Ancient Star Records)
-- ============================================================
CREATE TABLE IF NOT EXISTS ancient_stars (
    id              BIGSERIAL PRIMARY KEY,
    star_name_cn    VARCHAR(64) NOT NULL,
    star_name_alt   VARCHAR(128),
    constellation   VARCHAR(32),
    mansion_id      INTEGER REFERENCES lunar_mansions(id),
    dynasty_id      INTEGER NOT NULL REFERENCES dynasties(id),
    source_book     VARCHAR(64) NOT NULL,
    source_chapter  VARCHAR(64),

    ruxiu_du        DOUBLE PRECISION NOT NULL,
    quji_du         DOUBLE PRECISION NOT NULL,
    ruxiu_du_raw    VARCHAR(32),
    quji_du_raw     VARCHAR(32),

    magnitude_ancient VARCHAR(16),
    magnitude_num   DOUBLE PRECISION,
    color_desc      VARCHAR(32),
    color_class     VARCHAR(16),

    ra_j2000        DOUBLE PRECISION,
    dec_j2000       DOUBLE PRECISION,
    ra_ancient_conv DOUBLE PRECISION,
    dec_ancient_conv DOUBLE PRECISION,
    proper_motion_ra  DOUBLE PRECISION,
    proper_motion_dec DOUBLE PRECISION,
    parallax        DOUBLE PRECISION,
    hipparcos_id    INTEGER,
    henry_draper_id INTEGER,

    geom_sphere     GEOMETRY(Point, 4326),
    quality_flag    INTEGER DEFAULT 0,
    notes           TEXT,

    created_at      TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE ancient_stars IS '古代恒星记录主表';
COMMENT ON COLUMN ancient_stars.ruxiu_du IS '入宿度（度，含小数，如3.5）';
COMMENT ON COLUMN ancient_stars.quji_du IS '去极度（度）';
COMMENT ON COLUMN ancient_stars.ra_j2000 IS '现代证认的J2000.0赤经（度）';
COMMENT ON COLUMN ancient_stars.dec_j2000 IS '现代证认的J2000.0赤纬（度）';
COMMENT ON COLUMN ancient_stars.ra_ancient_conv IS '由古代坐标转换得到的J2000.0赤经（度）';
COMMENT ON COLUMN ancient_stars.dec_ancient_conv IS '由古代坐标转换得到的J2000.0赤纬（度）';
COMMENT ON COLUMN ancient_stars.proper_motion_ra IS '自行（赤经方向，mas/yr）';
COMMENT ON COLUMN ancient_stars.proper_motion_dec IS '自行（赤纬方向，mas/yr）';
COMMENT ON COLUMN ancient_stars.geom_sphere IS '球面坐标点（ra=lon, dec=lat）';
COMMENT ON COLUMN ancient_stars.quality_flag IS '数据质量标志：0未知，1低，2中，3高';

CREATE INDEX IF NOT EXISTS idx_ancient_stars_mansion ON ancient_stars(mansion_id);
CREATE INDEX IF NOT EXISTS idx_ancient_stars_dynasty ON ancient_stars(dynasty_id);
CREATE INDEX IF NOT EXISTS idx_ancient_stars_spatial ON ancient_stars USING gist (geom_sphere);
CREATE INDEX IF NOT EXISTS idx_ancient_stars_ra_dec ON ancient_stars(ra_j2000, dec_j2000);
CREATE INDEX IF NOT EXISTS idx_ancient_stars_magnitude ON ancient_stars(magnitude_num);
CREATE INDEX IF NOT EXISTS idx_ancient_stars_source ON ancient_stars(source_book);

-- ============================================================
-- 4. 彗星记录表 (Comet Records)
-- ============================================================
CREATE TABLE IF NOT EXISTS ancient_comets (
    id              BIGSERIAL PRIMARY KEY,
    comet_name      VARCHAR(64),
    appearance_id   VARCHAR(32) UNIQUE,
    dynasty_id      INTEGER REFERENCES dynasties(id),
    source_book     VARCHAR(64),

    start_date_text VARCHAR(64),
    end_date_text   VARCHAR(64),
    start_jd        DOUBLE PRECISION,
    end_jd          DOUBLE PRECISION,
    duration_days   INTEGER,

    mansion_id      INTEGER REFERENCES lunar_mansions(id),
    ruxiu_du        DOUBLE PRECISION,
    quji_du         DOUBLE PRECISION,
    position_desc   TEXT,

    brightness_desc VARCHAR(128),
    estimated_mag   DOUBLE PRECISION,
    tail_length     VARCHAR(64),
    tail_direction  VARCHAR(32),

    ra_apparent     DOUBLE PRECISION,
    dec_apparent    DOUBLE PRECISION,
    geom_sphere     GEOMETRY(Point, 4326),

    notes           TEXT,
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE ancient_comets IS '古代彗星观测记录';
CREATE INDEX IF NOT EXISTS idx_comets_dynasty ON ancient_comets(dynasty_id);
CREATE INDEX IF NOT EXISTS idx_comets_spatial ON ancient_comets USING gist (geom_sphere);
CREATE INDEX IF NOT EXISTS idx_comets_time ON ancient_comets(start_jd, end_jd);

-- ============================================================
-- 5. 客星/超新星记录表 (Guest Star / Supernova Records)
-- ============================================================
CREATE TABLE IF NOT EXISTS guest_stars (
    id              BIGSERIAL PRIMARY KEY,
    guest_name      VARCHAR(64),
    guest_id_code   VARCHAR(32) UNIQUE,
    dynasty_id      INTEGER REFERENCES dynasties(id),
    source_book     VARCHAR(64),

    appearance_date VARCHAR(64),
    disappearance_date VARCHAR(64),
    start_jd        DOUBLE PRECISION,
    end_jd          DOUBLE PRECISION,
    visibility_days INTEGER,

    mansion_id      INTEGER REFERENCES lunar_mansions(id),
    ruxiu_du        DOUBLE PRECISION,
    quji_du         DOUBLE PRECISION,
    position_desc   TEXT,

    peak_mag        DOUBLE PRECISION,
    peak_mag_err    DOUBLE PRECISION,
    light_curve_desc TEXT,
    color_at_peak   VARCHAR(32),

    ra_est          DOUBLE PRECISION,
    dec_est         DOUBLE PRECISION,
    ra_err          DOUBLE PRECISION,
    dec_err         DOUBLE PRECISION,
    geom_sphere     GEOMETRY(Point, 4326),

    remnant_candidate BIGINT,
    sn_type_hint    VARCHAR(16),
    notes           TEXT,
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE guest_stars IS '古代客星（超新星候选）记录';
CREATE INDEX IF NOT EXISTS idx_guest_stars_dynasty ON guest_stars(dynasty_id);
CREATE INDEX IF NOT EXISTS idx_guest_stars_spatial ON guest_stars USING gist (geom_sphere);
CREATE INDEX IF NOT EXISTS idx_guest_stars_time ON guest_stars(start_jd);

-- ============================================================
-- 6. 超新星遗迹表 (Supernova Remnants Catalog)
-- ============================================================
CREATE TABLE IF NOT EXISTS supernova_remnants (
    id              BIGSERIAL PRIMARY KEY,
    remnant_name    VARCHAR(64) UNIQUE,
    alias_names     TEXT,
    sn_type         VARCHAR(16),

    ra_deg          DOUBLE PRECISION NOT NULL,
    dec_deg         DOUBLE PRECISION NOT NULL,
    ra_err          DOUBLE PRECISION,
    dec_err         DOUBLE PRECISION,
    geom_sphere     GEOMETRY(Point, 4326),

    age_yr          DOUBLE PRECISION,
    age_err         DOUBLE PRECISION,
    explosion_jd    DOUBLE PRECISION,
    explosion_year_est DOUBLE PRECISION,

    distance_kpc    DOUBLE PRECISION,
    distance_err    DOUBLE PRECISION,
    diameter_pc     DOUBLE PRECISION,

    radio_flux_ghz  DOUBLE PRECISION,
    xray_luminosity DOUBLE PRECISION,
    gamma_detected  BOOLEAN DEFAULT FALSE,

    expansion_vel   DOUBLE PRECISION,
    spectral_index  DOUBLE PRECISION,
    notes           TEXT,
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE supernova_remnants IS '现代超新星遗迹观测目录（模拟数据）';
CREATE INDEX IF NOT EXISTS idx_snr_spatial ON supernova_remnants USING gist (geom_sphere);
CREATE INDEX IF NOT EXISTS idx_snr_type ON supernova_remnants(sn_type);
CREATE INDEX IF NOT EXISTS idx_snr_age ON supernova_remnants(age_yr);

-- ============================================================
-- 7. 贝叶斯匹配结果表 (Matching Results)
-- ============================================================
CREATE TABLE IF NOT EXISTS guest_star_matches (
    id              BIGSERIAL PRIMARY KEY,
    guest_star_id   BIGINT NOT NULL REFERENCES guest_stars(id) ON DELETE CASCADE,
    remnant_id      BIGINT NOT NULL REFERENCES supernova_remnants(id) ON DELETE CASCADE,

    spatial_score   DOUBLE PRECISION,
    temporal_score  DOUBLE PRECISION,
    magnitude_score DOUBLE PRECISION,
    total_log_posterior DOUBLE PRECISION,
    match_probability DOUBLE PRECISION,
    rank_within_guest INTEGER,

    angular_sep_arcmin DOUBLE PRECISION,
    time_delta_yr   DOUBLE PRECISION,
    bayes_factor    DOUBLE PRECISION,

    method_version  VARCHAR(32),
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(guest_star_id, remnant_id)
);

COMMENT ON TABLE guest_star_matches IS '客星与超新星遗迹的贝叶斯匹配结果';
CREATE INDEX IF NOT EXISTS idx_matches_guest ON guest_star_matches(guest_star_id);
CREATE INDEX IF NOT EXISTS idx_matches_remnant ON guest_star_matches(remnant_id);
CREATE INDEX IF NOT EXISTS idx_matches_prob ON guest_star_matches(match_probability DESC);

-- ============================================================
-- 8. 触发器：自动更新球面坐标
-- ============================================================
CREATE OR REPLACE FUNCTION update_geom_sphere_star()
RETURNS TRIGGER AS $$
BEGIN
    IF (NEW.ra_j2000 IS NOT NULL AND NEW.dec_j2000 IS NOT NULL) THEN
        NEW.geom_sphere := ST_SetSRID(ST_MakePoint(NEW.ra_j2000, NEW.dec_j2000), 4326);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_stars_geom ON ancient_stars;
CREATE TRIGGER trg_stars_geom
    BEFORE INSERT OR UPDATE OF ra_j2000, dec_j2000 ON ancient_stars
    FOR EACH ROW EXECUTE FUNCTION update_geom_sphere_star();

CREATE OR REPLACE FUNCTION update_geom_sphere_guest()
RETURNS TRIGGER AS $$
BEGIN
    IF (NEW.ra_est IS NOT NULL AND NEW.dec_est IS NOT NULL) THEN
        NEW.geom_sphere := ST_SetSRID(ST_MakePoint(NEW.ra_est, NEW.dec_est), 4326);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_guest_geom ON guest_stars;
CREATE TRIGGER trg_guest_geom
    BEFORE INSERT OR UPDATE OF ra_est, dec_est ON guest_stars
    FOR EACH ROW EXECUTE FUNCTION update_geom_sphere_guest();

CREATE OR REPLACE FUNCTION update_geom_sphere_snr()
RETURNS TRIGGER AS $$
BEGIN
    IF (NEW.ra_deg IS NOT NULL AND NEW.dec_deg IS NOT NULL) THEN
        NEW.geom_sphere := ST_SetSRID(ST_MakePoint(NEW.ra_deg, NEW.dec_deg), 4326);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_snr_geom ON supernova_remnants;
CREATE TRIGGER trg_snr_geom
    BEFORE INSERT OR UPDATE OF ra_deg, dec_deg ON supernova_remnants
    FOR EACH ROW EXECUTE FUNCTION update_geom_sphere_snr();

CREATE OR REPLACE FUNCTION update_geom_sphere_comet()
RETURNS TRIGGER AS $$
BEGIN
    IF (NEW.ra_apparent IS NOT NULL AND NEW.dec_apparent IS NOT NULL) THEN
        NEW.geom_sphere := ST_SetSRID(ST_MakePoint(NEW.ra_apparent, NEW.dec_apparent), 4326);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_comet_geom ON ancient_comets;
CREATE TRIGGER trg_comet_geom
    BEFORE INSERT OR UPDATE OF ra_apparent, dec_apparent ON ancient_comets
    FOR EACH ROW EXECUTE FUNCTION update_geom_sphere_comet();

-- ============================================================
-- 9. 视图：跨朝代恒星对比视图
-- ============================================================
CREATE OR REPLACE VIEW v_star_cross_dynasty AS
SELECT
    s1.star_name_cn,
    s1.constellation,
    d1.name_cn AS dynasty_1,
    d2.name_cn AS dynasty_2,
    s1.ruxiu_du AS ruxiu_1,
    s1.quji_du AS quji_1,
    s2.ruxiu_du AS ruxiu_2,
    s2.quji_du AS quji_2,
    (s2.ruxiu_du - s1.ruxiu_du) AS delta_ruxiu,
    (s2.quji_du - s1.quji_du) AS delta_quji,
    s1.ra_ancient_conv AS ra_conv_1,
    s1.dec_ancient_conv AS dec_conv_1,
    s2.ra_ancient_conv AS ra_conv_2,
    s2.dec_ancient_conv AS dec_conv_2,
    s1.magnitude_num AS mag_1,
    s2.magnitude_num AS mag_2,
    s1.color_desc AS color_1,
    s2.color_desc AS color_2
FROM ancient_stars s1
JOIN ancient_stars s2
    ON s1.star_name_cn = s2.star_name_cn
    AND s1.dynasty_id < s2.dynasty_id
    AND s1.id != s2.id
JOIN dynasties d1 ON s1.dynasty_id = d1.id
JOIN dynasties d2 ON s2.dynasty_id = d2.id;

COMMENT ON VIEW v_star_cross_dynasty IS '同一颗恒星在不同朝代记录的坐标对比视图';

-- ============================================================
-- 10. 辅助函数：天球角距离计算
-- ============================================================
CREATE OR REPLACE FUNCTION angular_distance_deg(
    ra1_deg DOUBLE PRECISION, dec1_deg DOUBLE PRECISION,
    ra2_deg DOUBLE PRECISION, dec2_deg DOUBLE PRECISION
) RETURNS DOUBLE PRECISION AS $$
DECLARE
    ra1 DOUBLE PRECISION := radians(ra1_deg);
    dec1 DOUBLE PRECISION := radians(dec1_deg);
    ra2 DOUBLE PRECISION := radians(ra2_deg);
    dec2 DOUBLE PRECISION := radians(dec2_deg);
    d_ra DOUBLE PRECISION;
    cos_d DOUBLE PRECISION;
BEGIN
    d_ra := ra2 - ra1;
    cos_d := sin(dec1) * sin(dec2) + cos(dec1) * cos(dec2) * cos(d_ra);
    cos_d := GREATEST(-1.0, LEAST(1.0, cos_d));
    RETURN degrees(acos(cos_d));
END;
$$ LANGUAGE plpgsql IMMUTABLE;

COMMENT ON FUNCTION angular_distance_deg IS '计算两点在天球上的角距离（Haversine-like，返回度）';
