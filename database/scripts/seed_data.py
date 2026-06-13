"""
古代星表数据导入脚本 (Seed Data Generator & Importer)
=======================================================
功能:
  1. 插入朝代基础数据 (Han -> Qing)
  2. 插入二十八宿基础数据
  3. 生成 1200 条古代恒星记录 (跨多个朝代)
  4. 生成彗星记录
  5. 生成客星(超新星候选)记录
  6. 生成现代超新星遗迹目录 (模拟数据)

用法:
    pip install psycopg2-binary numpy
    python seed_data.py

环境变量 (可选):
    DB_HOST, DB_PORT, DB_NAME, DB_USER, DB_PASSWORD
"""

import os
import json
import random
import math
from dataclasses import dataclass, asdict
from typing import List, Tuple

import numpy as np

try:
    import psycopg2
    from psycopg2.extras import execute_values
except ImportError:
    raise SystemExit("请先安装: pip install psycopg2-binary numpy")

random.seed(42)
np.random.seed(42)

DB_CFG = {
    "host": os.getenv("DB_HOST", "localhost"),
    "port": int(os.getenv("DB_PORT", "5432")),
    "dbname": os.getenv("DB_NAME", "ancient_star_catalog"),
    "user": os.getenv("DB_USER", "postgres"),
    "password": os.getenv("DB_PASSWORD", "postgres"),
}

# ======================================================================
# 1. 中国主要朝代基础数据
# ======================================================================
DYNASTIES = [
    # (name_cn, name_en, start, end, canonical_epoch)
    ("西汉", "Western Han",   -206,   8, -100.0),
    ("东汉", "Eastern Han",      9, 220,  100.0),
    ("三国", "Three Kingdoms", 220, 280,  250.0),
    ("晋",   "Jin Dynasty",    265, 420,  350.0),
    ("南北朝","Northern & Southern Dynasties", 420, 589, 500.0),
    ("隋",   "Sui Dynasty",    581, 618,  600.0),
    ("唐",   "Tang Dynasty",   618, 907,  750.0),
    ("五代", "Five Dynasties", 907, 960,  930.0),
    ("宋",   "Song Dynasty",   960, 1279, 1100.0),
    ("元",   "Yuan Dynasty",  1271, 1368, 1320.0),
    ("明",   "Ming Dynasty",  1368, 1644, 1500.0),
    ("清",   "Qing Dynasty",  1644, 1912, 1780.0),
]

# 儒略日转 J2000.0 (JD 2451545.0 = 2000-01-01 12:00 TT)
def julian_year_to_jd(year: float) -> float:
    return 2451545.0 + (year - 2000.0) * 365.25

# ======================================================================
# 2. 二十八宿基础数据
#    standard_ra_deg: 距星在 J2000.0 下的赤经 (度)
#    extent_deg:       该宿距度 (古度)
# ======================================================================
LUNAR_MANSIONS = [
    # (order, cn, pinyin, animal,  azimuth, ra,  extent)
    ( 1, "角", "Jiao",   "蛟",   "东",  187.75, 12.0),
    ( 2, "亢", "Kang",   "龙",   "东",  198.20,  9.0),
    ( 3, "氐", "Di",     "貉",   "东",  209.45, 15.0),
    ( 4, "房", "Fang",   "兔",   "东",  221.35,  5.0),
    ( 5, "心", "Xin",    "狐",   "东",  231.55,  5.0),
    ( 6, "尾", "Wei",    "虎",   "东",  240.73, 18.0),
    ( 7, "箕", "Ji",     "豹",   "东",  251.10, 11.0),
    ( 8, "斗", "Dou",    "獬",   "北",  262.52, 26.0),
    ( 9, "牛", "Niu",    "牛",   "北",  280.26,  8.0),
    (10, "女", "Nv",     "蝠",   "北",  291.10, 12.0),
    (11, "虚", "Xu",     "鼠",   "北",  303.80, 10.0),
    (12, "危", "Wei",    "燕",   "北",  315.77, 17.0),
    (13, "室", "Shi",    "猪",   "北",  334.25, 16.0),
    (14, "壁", "Bi",     "狳",   "北",  349.00,  9.0),
    (15, "奎", "Kui",    "狼",   "西",    0.25, 16.0),
    (16, "娄", "Lou",    "狗",   "西",   16.45, 12.0),
    (17, "胃", "Wei",    "雉",   "西",   30.05, 14.0),
    (18, "昴", "Mao",    "鸡",   "西",   44.35, 11.0),
    (19, "毕", "Bi",     "乌",   "西",   57.25, 16.0),
    (20, "觜", "Zi",     "猴",   "西",   73.60,  3.0),
    (21, "参", "Shen",   "猿",   "西",   78.75, 10.0),
    (22, "井", "Jing",   "犴",   "南",   90.15, 33.0),
    (23, "鬼", "Gui",    "羊",   "南",  117.50,  4.0),
    (24, "柳", "Liu",    "獐",   "南",  124.80, 15.0),
    (25, "星", "Xing",   "马",   "南",  138.30,  7.0),
    (26, "张", "Zhang",  "鹿",   "南",  147.10, 18.0),
    (27, "翼", "Yi",     "蛇",   "南",  161.60, 18.0),
    (28, "轸", "Zhen",   "蚓",   "南",  175.00, 17.0),
]

# ======================================================================
# 3. 典籍列表
# ======================================================================
SOURCE_BOOKS = {
    "甘石星经":    "Warring States",
    "史记·天官书": "Western Han",
    "汉书·天文志": "Eastern Han",
    "后汉书·天文志":"Eastern Han",
    "晋书·天文志": "Jin",
    "隋书·天文志": "Sui",
    "开元占经":    "Tang",
    "新唐书·天文志":"Song",
    "宋史·天文志": "Song",
    "辽史·历象志": "Yuan",
    "元史·天文志": "Yuan",
    "观象玩占":    "Ming",
    "明实录":      "Ming",
    "仪象考成":    "Qing",
    "历象考成":    "Qing",
    "清实录":      "Qing",
}

# 按朝代给出可用典籍
DYNASTY_BOOKS = {
    "西汉": ["史记·天官书", "汉书·天文志"],
    "东汉": ["汉书·天文志", "后汉书·天文志", "甘石星经"],
    "三国": ["后汉书·天文志"],
    "晋":   ["晋书·天文志", "甘石星经"],
    "南北朝":["晋书·天文志"],
    "隋":   ["隋书·天文志", "晋书·天文志"],
    "唐":   ["开元占经", "隋书·天文志", "新唐书·天文志"],
    "五代": ["新唐书·天文志"],
    "宋":   ["新唐书·天文志", "宋史·天文志"],
    "元":   ["辽史·历象志", "元史·天文志", "宋史·天文志"],
    "明":   ["观象玩占", "明实录", "元史·天文志"],
    "清":   ["仪象考成", "历象考成", "清实录", "明实录"],
}

# ======================================================================
# 4. 古代星等和颜色描述映射
# ======================================================================
MAGNITUDE_DESCS = [
    ("大星",    0.5),  ("明大星",  0.0),
    ("中大星",  2.0),  ("星明",    2.5),
    ("小星",    4.0),  ("星微明",  4.5),
    ("甚小星",  5.5),  ("不见",    6.5),
]

COLOR_DESCS = [
    ("白",    "B/V",  "#f5f7ff"),  ("青",    "O/B", "#a0c8ff"),
    ("赤",    "M/K",  "#ffa070"),  ("黄",    "G/K", "#fff0c0"),
    ("苍",    "B/A",  "#c0d8ff"),  ("黑",    "未证认", "#808080"),
    ("白赤",  "F/G",  "#ffe0d0"),
]

# 传统星官/星座名
CHINESE_CONSTELLATIONS = [
    "紫微垣", "太微垣", "天市垣",
    "角宿", "亢宿", "氐宿", "房宿", "心宿", "尾宿", "箕宿",
    "斗宿", "牛宿", "女宿", "虚宿", "危宿", "室宿", "壁宿",
    "奎宿", "娄宿", "胃宿", "昴宿", "毕宿", "觜宿", "参宿",
    "井宿", "鬼宿", "柳宿", "星宿", "张宿", "翼宿", "轸宿",
    "北斗", "南斗", "文昌", "三台", "四辅", "华盖", "杠星",
    "贯索", "天牢", "天纪", "少微", "长垣", "灵台", "明堂",
    "五帝座", "太子", "从官", "幸臣", "郎将", "郎位", "常陈",
    "轩辕", "御女", "女史", "柱史", "天柱", "尚书", "阴德",
    "天枪", "天棓", "玄戈", "天枢", "天璇", "天玑", "天权",
    "玉衡", "开阳", "摇光", "天乙", "太乙", "紫微左垣", "紫微右垣",
]

# 客星光变描述
LIGHT_CURVES = [
    "初见大如桃，色白，月余渐微，凡见二百三十日乃消",
    "初出如盏，明烛地，昼或见之，百五日而没",
    "初现明如月，七日而衰，百日不见",
    "昼见有芒，色赤，凡见三十一日，夜见七十一日",
    "其出也，夕见西方，晨见东方，历月方没",
    "初昏见于东南，大如盘，色红，三阅月而渐隐",
]

# ======================================================================
# 5. 生成基础"真实"恒星骨架 (用于跨朝代多记录关联)
#    每颗"真实"恒星有 J2000.0 坐标和自行
# ======================================================================
@dataclass
class BaseStar:
    base_name: str
    constellation: str
    ra_j2000: float
    dec_j2000: float
    pm_ra: float
    pm_dec: float
    mag: float
    color_class: str
    color_desc: str

def generate_base_stars(n: int = 350) -> List[BaseStar]:
    """生成基础恒星，均匀分布在北天和天赤道附近"""
    stars = []
    for i in range(n):
        # 偏向北天和可见天区
        u = np.random.uniform()
        dec = math.degrees(math.asin(0.25 + 0.75 * u))   # 15° ~ 90°
        if np.random.random() < 0.25:
            dec = np.random.uniform(-30, 15)
        ra = np.random.uniform(0, 360)

        # 星等分布: 亮星少，暗星多
        r = np.random.random()
        if r < 0.05:    mag = np.random.uniform(-1.5, 1.5)
        elif r < 0.25:  mag = np.random.uniform(1.5, 3.0)
        elif r < 0.70:  mag = np.random.uniform(3.0, 5.0)
        else:           mag = np.random.uniform(5.0, 6.5)

        # 自行 mas/yr (典型值 0 ~ 300，少数大自行星)
        pm_ra  = np.random.normal(0, 25)
        pm_dec = np.random.normal(0, 20)
        if np.random.random() < 0.03:
            pm_ra  = np.random.choice([-1, 1]) * np.random.uniform(100, 400)
            pm_dec = np.random.choice([-1, 1]) * np.random.uniform(100, 300)

        constellation = random.choice(CHINESE_CONSTELLATIONS)
        idx_in_const = random.randint(1, 7)
        base_name = f"{constellation}{idx_in_const}"

        cd = random.choice(COLOR_DESCS)
        color_desc, color_class, _hex = cd

        stars.append(BaseStar(
            base_name=base_name,
            constellation=constellation,
            ra_j2000=ra,
            dec_j2000=dec,
            pm_ra=pm_ra,
            pm_dec=pm_dec,
            mag=mag,
            color_class=color_class,
            color_desc=color_desc,
        ))
    return stars

# ======================================================================
# 6. 岁差模型 (简化 IAU 2000A，精度足够用于古代数据)
# ======================================================================
def precession_rotation(ra_rad, dec_rad, from_julian_yr, to_julian_yr):
    """
    利用 Z-X-Z Euler 岁差角旋转 (简化 Vondrak 近似)
    返回 (to_ra_rad, to_dec_rad)
    """
    T = (from_julian_yr - to_julian_yr) / 100.0  # 反向岁差
    # 岁差速率 (度/世纪), 近似 J2000 值
    zeta_A  = (0.001397 * T + 0.3084778) * T + 2.650545
    z_A     = (-0.001147 * T + 1.0926022) * T
    theta_A = (0.004199 * T - 0.4294934) * T
    zeta_A  = math.radians(zeta_A * T)
    z_A     = math.radians(z_A * T)
    theta_A = math.radians(theta_A * T)

    # 先把球坐标变笛卡尔
    x = math.cos(dec_rad) * math.cos(ra_rad)
    y = math.cos(dec_rad) * math.sin(ra_rad)
    z = math.sin(dec_rad)

    # R_z(-zeta_A) * R_x(theta_A) * R_z(-z_A) 的转置 (反向)
    # 应用 R = Rz(z_A) * Rx(-th) * Rz(zeta_A)
    def rot_z(v, a):
        c, s = math.cos(a), math.sin(a)
        return (c*v[0] - s*v[1], s*v[0] + c*v[1], v[2])
    def rot_x(v, a):
        c, s = math.cos(a), math.sin(a)
        return (v[0], c*v[1] - s*v[2], s*v[1] + c*v[2])

    v = (x, y, z)
    v = rot_z(v, zeta_A)
    v = rot_x(v, -theta_A)
    v = rot_z(v, z_A)
    nx, ny, nz = v
    nra = math.atan2(ny, nx) % (2 * math.pi)
    ndec = math.asin(max(-1, min(1, nz)))
    return nra, ndec

# ======================================================================
# 7. 将 J2000.0 赤经赤纬 反推为古代某朝代的入宿度/去极度
# ======================================================================
def j2000_to_ancient_coords(ra_j2000: float, dec_j2000: float,
                            epoch_yr: float,
                            pm_ra_mas: float = 0, pm_dec_mas: float = 0):
    # J2000.0 -> 古代视位置 (自行 + 岁差)
    dt = epoch_yr - 2000.0
    ra_old = ra_j2000 + (pm_ra_mas / 3600e3) * dt / math.cos(math.radians(dec_j2000))
    dec_old = dec_j2000 + (pm_dec_mas / 3600e3) * dt
    ra_old = ra_old % 360

    ra_r = math.radians(ra_old)
    dec_r = math.radians(dec_old)
    ra_ancient_r, dec_ancient_r = precession_rotation(
        ra_r, dec_r, 2000.0, epoch_yr)
    ra_ancient = math.degrees(ra_ancient_r) % 360
    dec_ancient = math.degrees(dec_ancient_r)

    # 找最近的二十八宿距星
    mansion_idx = None
    min_ra_diff = 1e9
    for idx, (_o, _n, _py, _a, _az, m_ra, _ext) in enumerate(LUNAR_MANSIONS):
        # 古代距星赤经 = 距星 J2000 赤经 再岁差回古代
        mra_r = math.radians(m_ra)
        mdec_r = math.radians(dec_j2000 * 0 + 5.0)  # 近似
        mra_old_r, _ = precession_rotation(mra_r, mdec_r, 2000.0, epoch_yr)
        mra_old = math.degrees(mra_old_r) % 360
        diff = (ra_ancient - mra_old + 360) % 360
        if diff >= 0 and diff < LUNAR_MANSIONS[idx][6] * 1.3:
            if diff < min_ra_diff:
                min_ra_diff = diff
                mansion_idx = idx
    if mansion_idx is None:
        diffs = []
        for idx, (_o, _n, _py, _a, _az, m_ra, _ext) in enumerate(LUNAR_MANSIONS):
            mra_r = math.radians(m_ra)
            mra_old_r, _ = precession_rotation(mra_r, 0, 2000.0, epoch_yr)
            mra_old = math.degrees(mra_old_r) % 360
            d = (ra_ancient - mra_old + 720) % 360
            diffs.append((d, idx))
        diffs.sort()
        mansion_idx = diffs[0][1]
        min_ra_diff = diffs[0][0]

    ruxiu = min_ra_diff
    quji  = 90.0 - dec_ancient  # 去极度 = 90° - 赤纬
    return mansion_idx, ruxiu, quji, ra_ancient, dec_ancient

# ======================================================================
# 8. 数据生成主体
# ======================================================================
def seed_dynasties(cur):
    rows = []
    for name_cn, name_en, s, e, ep in DYNASTIES:
        rows.append((name_cn, name_en, s, e, ep, julian_year_to_jd(ep),
                     f"{name_cn} (公元前{-s}年-公元{e}年)" if s < 0 else f"{name_cn} ({s}年-{e}年)"))
    execute_values(cur, """
        INSERT INTO dynasties (name_cn, name_en, start_year, end_year, canonical_epoch, epoch_jd, description)
        VALUES %s RETURNING id, name_cn""", rows)
    res = cur.fetchall()
    return {name_cn: id for id, name_cn in res}

def seed_mansions(cur):
    rows = [m + (f"{m[1]}宿，四象{m[4]}方",) for m in LUNAR_MANSIONS]
    execute_values(cur, """
        INSERT INTO lunar_mansions
            (mansion_order, name_cn, name_pinyin, animal, azimuth,
             standard_ra_deg, extent_deg, description)
        VALUES %s RETURNING id, mansion_order""", rows)
    res = cur.fetchall()
    return {order: id for id, order in res}

def generate_and_insert_stars(cur, dynasty_ids, mansion_order_ids, base_stars: List[BaseStar]):
    total = 0
    star_records = []
    # 为每颗基础恒星在每个朝代中按概率生成记录 (越早的朝代覆盖越少)
    dynasty_prob = {
        "西汉": 0.35, "东汉": 0.5,  "三国": 0.4,  "晋": 0.55,
        "南北朝":0.45, "隋": 0.55,  "唐": 0.85,  "五代": 0.5,
        "宋":   0.9,  "元": 0.88,  "明": 0.92,  "清": 0.95,
    }
    for bs in base_stars:
        for name_cn, ep_y in [(d[0], d[4]) for d in DYNASTIES]:
            if random.random() > dynasty_prob.get(name_cn, 0.5):
                continue
            did = dynasty_ids[name_cn]
            ep = ep_y
            midx, ruxiu, quji, ra_ancient_old, dec_ancient_old = \
                j2000_to_ancient_coords(bs.ra_j2000, bs.dec_j2000, ep, bs.pm_ra, bs.pm_dec)
            mansion_id = mansion_order_ids[midx + 1]

            # 添加古代测量误差 (入宿度 +- 0.5°, 去极度 +- 0.4°)
            ruxiu += np.random.normal(0, 0.45)
            quji  += np.random.normal(0, 0.40)
            ruxiu = max(0, ruxiu)
            quji  = max(0, min(170, quji))

            books = DYNASTY_BOOKS.get(name_cn, ["甘石星经"])
            src_book = random.choice(books)
            src_chapter = f"卷{random.randint(1, 30)}"

            # 古代星等描述
            mag_choices = sorted(MAGNITUDE_DESCS, key=lambda x: abs(x[1] - bs.mag))
            chosen_mag_desc, chosen_mag_num = mag_choices[0]
            if random.random() < 0.2:
                chosen_mag_desc, chosen_mag_num = random.choice(mag_choices[:min(3, len(mag_choices))])

            # 古代原始记录字符串
            ruxiu_whole = int(ruxiu)
            ruxiu_frac  = ruxiu - ruxiu_whole
            ruxiu_str = f"{ruxiu_whole}度"
            if abs(ruxiu_frac - 0.5) < 0.25:
                ruxiu_str = f"{ruxiu_whole}度半"
            elif abs(ruxiu_frac - 0.25) < 0.12:
                ruxiu_str = f"{ruxiu_whole}度少"
            elif abs(ruxiu_frac - 0.75) < 0.12:
                ruxiu_str = f"{ruxiu_whole}度太"
            quji_str = f"{int(round(quji))}度"

            # 质量标志：明清高，汉代中低
            if name_cn in ("清", "明", "元", "宋"):
                qf = 3
            elif name_cn in ("唐", "隋", "晋", "南北朝"):
                qf = 2
            else:
                qf = 1
            if random.random() < 0.1:
                qf = max(1, qf - 1)

            star_records.append((
                bs.base_name, f"《{src_book}》|{bs.constellation}",
                bs.constellation, mansion_id, did, src_book, src_chapter,
                round(ruxiu, 3), round(quji, 3), ruxiu_str, quji_str,
                chosen_mag_desc, round(chosen_mag_num, 2),
                bs.color_desc, bs.color_class,
                round(bs.ra_j2000, 5), round(bs.dec_j2000, 5),
                round(ra_ancient_old, 5), round(dec_ancient_old, 5),
                round(bs.pm_ra, 3), round(bs.pm_dec, 3),
                round(max(0.01, np.random.exponential(20)), 4),
                int(1000 + total), int(10000 + total),
                qf,
                f"base_seed={id(bs)}; epoch={ep:.1f}"
            ))
            total += 1
            if total >= 1200:
                break
        if total >= 1200:
            break

    execute_values(cur, """
        INSERT INTO ancient_stars (
            star_name_cn, star_name_alt, constellation,
            mansion_id, dynasty_id, source_book, source_chapter,
            ruxiu_du, quji_du, ruxiu_du_raw, quji_du_raw,
            magnitude_ancient, magnitude_num, color_desc, color_class,
            ra_j2000, dec_j2000,
            ra_ancient_conv, dec_ancient_conv,
            proper_motion_ra, proper_motion_dec,
            parallax, hipparcos_id, henry_draper_id,
            quality_flag, notes
        ) VALUES %s""", star_records, page_size=500)
    return total

def generate_and_insert_comets(cur, dynasty_ids, mansion_order_ids):
    comets = []
    N = 28
    for i in range(N):
        name_cn = random.choice(list(dynasty_ids.keys()))
        did = dynasty_ids[name_cn]
        dynasty_epoch = [d[4] for d in DYNASTIES if d[0] == name_cn][0]
        start_year = dynasty_epoch + np.random.uniform(-80, 80)
        start_jd = julian_year_to_jd(start_year) + np.random.uniform(-180, 180)
        duration = int(np.random.uniform(10, 180))
        end_jd = start_jd + duration

        # 随机位置 (黄道附近)
        ra = np.random.uniform(0, 360)
        dec = np.random.normal(0, 15)
        midx, ruxiu, quji, _, _ = j2000_to_ancient_coords(ra, dec, start_year)
        mansion_id = mansion_order_ids[midx + 1]

        est_mag = round(np.random.uniform(-1, 5), 1)
        tail_len = random.choice([None, "长三尺", "长丈余", "长数尺", "竟天", "长二丈", "长一尺"])
        tail_dir = random.choice([None, "指东", "指西", "指南", "指北", "东南", "西北"])

        comets.append((
            f"彗星#{i+1:03d}", f"C-{int(start_year)}-N{i:02d}",
            did, random.choice(list(SOURCE_BOOKS.keys())),
            f"{int(start_year)}年{random.randint(1,12)}月",
            f"{int(start_year + (duration/365))}年{random.randint(1,12)}月",
            start_jd, end_jd, duration,
            mansion_id, round(ruxiu, 2), round(quji, 2),
            random.choice(["见于" + m[1] + "宿之西", "入于" + m[1] + "宿", "经" + m[1] + "宿而行"]),
            random.choice(["昼见", "夜见", "明烛地", "有芒", "大如桃", "色苍白"]),
            est_mag, tail_len, tail_dir,
            round(ra, 4), round(dec, 4)
        ))
    execute_values(cur, """
        INSERT INTO ancient_comets (
            comet_name, appearance_id, dynasty_id, source_book,
            start_date_text, end_date_text, start_jd, end_jd, duration_days,
            mansion_id, ruxiu_du, quji_du, position_desc,
            brightness_desc, estimated_mag, tail_length, tail_direction,
            ra_apparent, dec_apparent
        ) VALUES %s""", comets)
    return len(comets)

def generate_and_insert_guest_stars(cur, dynasty_ids, mansion_order_ids):
    """生成客星记录（含历史上著名客星 SN 1054, SN 1006, SN 1181, SN 1572, SN 1604 等）"""
    known = [
        # (name, code, dynasty, year, ra, dec, peak_mag, desc)
        ("周伯星", "SN-1006",  "宋",  1006, 225.0, -41.9, -9.5, "初见于氐宿，昼见如半月，有芒角"),
        ("天关客星", "SN-1054","宋",  1054,  83.6,  22.0, -6.0, "见于天关东南，可数寸，凡见二十三日昼见"),
        ("传舍客星", "SN-1181","宋",  1181,  23.0,  64.0,  0.0, "见于传舍，凡见一百八十五日"),
        ("阁道客星", "SN-1572","明",  1572,   6.3,  64.1, -4.0, "见于阁道旁，昼见，凡见二十三"),
        ("尾分客星", "SN-1604","明",  1604, 255.7, -21.3, -2.5, "见于尾分，明如金星，渐微")
    ]
    guests = []
    for i, (name, code, dyn_name, year, ra, dec, peak_mag, desc) in enumerate(known):
        did = dynasty_ids[dyn_name]
        midx, ruxiu, quji, _, _ = j2000_to_ancient_coords(ra, dec, float(year))
        mansion_id = mansion_order_ids[midx + 1]
        start_jd = julian_year_to_jd(year + np.random.uniform(0, 0.6))
        vis_days = int(np.random.uniform(60, 600))
        end_jd = start_jd + vis_days
        guests.append((
            name, code, did, random.choice(DYNASTY_BOOKS[dyn_name]),
            f"{year}年{random.randint(1,12)}月", f"{year + vis_days // 365}年",
            start_jd, end_jd, vis_days,
            mansion_id, round(ruxiu, 2), round(quji, 2), desc,
            peak_mag, 0.5,
            random.choice(LIGHT_CURVES),
            random.choice(["赤", "白", "黄", "青"]),
            round(ra, 4), round(dec, 4), 0.5, 0.5
        ))
    # 再生成一些未证认的客星
    for i in range(15):
        name_cn = random.choice(list(dynasty_ids.keys()))
        did = dynasty_ids[name_cn]
        ep = [d[4] for d in DYNASTIES if d[0] == name_cn][0]
        year = ep + np.random.uniform(-60, 60)
        ra = np.random.uniform(0, 360)
        dec = np.random.uniform(-40, 80)
        midx, ruxiu, quji, _, _ = j2000_to_ancient_coords(ra, dec, year)
        mansion_id = mansion_order_ids[midx + 1]
        start_jd = julian_year_to_jd(year)
        vis_days = int(np.random.uniform(30, 400))
        guests.append((
            f"{name_cn}客星#{i+1:02d}", f"GX-{int(year)}-{i:02d}",
            did, random.choice(list(SOURCE_BOOKS.keys())),
            f"{int(year)}年", f"{int(year + vis_days // 365)}年",
            start_jd, start_jd + vis_days, vis_days,
            mansion_id, round(ruxiu, 2), round(quji, 2),
            random.choice(LIGHT_CURVES),
            round(np.random.uniform(-4, 2), 1), 0.8,
            random.choice(LIGHT_CURVES),
            random.choice(["赤", "白", "黄"]),
            round(ra, 4), round(dec, 4), 1.0, 1.0
        ))
    execute_values(cur, """
        INSERT INTO guest_stars (
            guest_name, guest_id_code, dynasty_id, source_book,
            appearance_date, disappearance_date, start_jd, end_jd, visibility_days,
            mansion_id, ruxiu_du, quji_du, position_desc,
            peak_mag, peak_mag_err, light_curve_desc, color_at_peak,
            ra_est, dec_est, ra_err, dec_err
        ) VALUES %s""", guests)
    return len(guests)

def generate_and_insert_snr(cur):
    """生成模拟超新星遗迹目录"""
    known_remnants = [
        ("蟹状星云", "Crab Nebula, M1, SN 1054", "II",
         83.6331, 22.0145, 0.5, 0.5, 970, 30, 2.0, 0.3, 3.4, 200, 4.0, 10e36, True, 1400, 0.0),
        ("SN 1006 遗迹", "SNR G327.6+14.6, PKS 1459-41", "Ia",
         225.0, -41.9, 2.0, 2.0, 1020, 40, 2.2, 0.3, 30, 170, 2.0, 1e36, False, 2900, 0.5),
        ("3C 58", "3C 58, G130.7+3.1", "II",
         23.2, 64.15, 0.5, 0.5, 840, 50, 3.2, 0.4, 12, 120, 1.3, 5e36, True, 700, 0.05),
        ("第谷超新星遗迹","SN 1572, G120.1+1.4, 3C 10", "Ia",
         6.35, 64.13, 0.2, 0.2, 450, 10, 2.5, 0.2, 20, 110, 1.6, 2e37, True, 5000, 0.6),
        ("开普勒超新星遗迹","SN 1604, G4.5+6.8", "Ia",
         255.6, -21.4, 0.3, 0.3, 420, 10, 6.0, 0.4, 6, 160, 1.2, 3e37, True, 7500, 0.7),
    ]
    remnants = []
    for row in known_remnants:
        (name, alias, sntype, ra, dec, ra_err, dec_err,
         age, age_err, dist, dist_err, dpc, rad_flux, xray, gam, exp_vel, sp_idx) = row
        remnants.append((
            name, alias, sntype,
            ra, dec, ra_err, dec_err,
            age, age_err, julian_year_to_jd(2000 - age/1), 2000 - age,
            dist, dist_err, dpc,
            rad_flux, xray, gam, exp_vel, sp_idx,
            "已知历史超新星对应遗迹"
        ))
    # 生成一批未知遗迹
    names = [f"G{x:05.1f}{y:+04.1f}" for x in np.random.uniform(0, 360, 50)
             for y in [np.random.uniform(-3, 3)]]
    for i, nm in enumerate(names[:45]):
        ra = np.random.uniform(0, 360)
        dec = np.random.uniform(-40, 80)
        age = np.random.exponential(800) + 100
        age_err = age * 0.2
        dist = round(np.random.uniform(1, 15), 2)
        sntype = random.choice(["II", "Ib", "Ic", "Ia", "IIn", "未分类"])
        remnants.append((
            nm, "Simulated SNR", sntype,
            round(ra, 5), round(dec, 5),
            round(np.random.uniform(0.2, 3), 2), round(np.random.uniform(0.2, 3), 2),
            round(age, 0), round(age_err, 0),
            julian_year_to_jd(2000 - age), round(2000 - age, 0),
            dist, round(dist * 0.15, 2),
            round(np.random.uniform(5, 40), 2),
            round(np.random.exponential(30), 3), round(np.random.uniform(1, 999), 1),
            np.random.random() < 0.2,
            round(np.random.uniform(200, 12000), 0),
            round(np.random.uniform(0.2, 0.8), 2),
            "模拟数据"
        ))
    execute_values(cur, """
        INSERT INTO supernova_remnants (
            remnant_name, alias_names, sn_type,
            ra_deg, dec_deg, ra_err, dec_err,
            age_yr, age_err, explosion_jd, explosion_year_est,
            distance_kpc, distance_err, diameter_pc,
            radio_flux_ghz, xray_luminosity, gamma_detected,
            expansion_vel, spectral_index, notes
        ) VALUES %s""", remnants)
    return len(remnants)

# ======================================================================
# 9. 主函数
# ======================================================================
def main():
    print("=" * 60)
    print("古代星表数据生成与导入脚本")
    print("=" * 60)

    print("\n[1/4] 生成基础恒星骨架...")
    base_stars = generate_base_stars(350)
    print(f"      已生成 {len(base_stars)} 颗基础恒星")

    print("\n[2/4] 连接数据库...")
    print(f"      {DB_CFG['host']}:{DB_CFG['port']}/{DB_CFG['dbname']} user={DB_CFG['user']}")
    conn = psycopg2.connect(**DB_CFG)
    conn.autocommit = False
    cur = conn.cursor()

    try:
        print("\n[3/4] 插入基础数据 (朝代 + 二十八宿)...")
        dynasty_ids = seed_dynasties(cur)
        mansion_order_ids = seed_mansions(cur)
        conn.commit()
        print(f"      朝代: {len(dynasty_ids)} | 二十八宿: {len(mansion_order_ids)}")

        print("\n[4/4] 生成并插入星表数据...")
        n_stars = generate_and_insert_stars(cur, dynasty_ids, mansion_order_ids, base_stars)
        conn.commit()
        print(f"      恒星记录: {n_stars}")

        n_comets = generate_and_insert_comets(cur, dynasty_ids, mansion_order_ids)
        conn.commit()
        print(f"      彗星记录: {n_comets}")

        n_guests = generate_and_insert_guest_stars(cur, dynasty_ids, mansion_order_ids)
        conn.commit()
        print(f"      客星记录: {n_guests}")

        n_snr = generate_and_insert_snr(cur)
        conn.commit()
        print(f"      超新星遗迹: {n_snr}")

        print("\n" + "=" * 60)
        print("全部数据导入成功!")
        print(f"总计: {n_stars + n_comets + n_guests + n_snr} 条记录")
        print("=" * 60)
    except Exception as e:
        conn.rollback()
        print(f"\n[ERROR] 数据导入失败: {e}")
        import traceback; traceback.print_exc()
        raise
    finally:
        cur.close()
        conn.close()

if __name__ == "__main__":
    main()
