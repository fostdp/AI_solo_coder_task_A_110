# 古代星表数据数字化与现代天体物理验证系统

> Ancient Star Catalog Digitization & Modern Astrophysical Verification System

## 项目概述

本系统将中国古代天文观测记录（从汉代至清代共 1200 条）进行数字化处理，并通过现代天体物理模型进行坐标转换和验证。核心功能包括：

- **古代坐标转换**：基于 IAU 2006 岁差模型、章动模型和恒星自行模型，将古代入宿度/去极度转换为现代 J2000.0 赤经赤纬
- **客星证认**：基于贝叶斯推断的时空交叉匹配，将古代客星记录与现代超新星遗迹目录进行概率匹配
- **星空可视化**：Three.js 渲染的天球视图，支持朝代切换、恒星详情查看、自行轨迹标注
- **跨朝代对比**：同一星区在不同朝代的坐标变化对比分析

## 系统架构

```
┌─────────────────────────────────────────────────┐
│                   Frontend                       │
│  Three.js + Canvas (天球视图, 时间轴, 面板)       │
└───────────────────┬─────────────────────────────┘
                    │ REST API
┌───────────────────▼─────────────────────────────┐
│              Rust Backend (Actix-Web)             │
│  ┌──────────────┐  ┌──────────────────────────┐  │
│  │ 坐标转换引擎  │  │ 贝叶斯匹配引擎           │  │
│  │ (岁差/章动/  │  │ (空间/时间/星等似然 +    │  │
│  │  自行模型)   │  │  Student-t 长尾分布)     │  │
│  └──────────────┘  └──────────────────────────┘  │
└───────────────────┬─────────────────────────────┘
                    │ SQL
┌───────────────────▼─────────────────────────────┐
│           PostgreSQL + PostGIS                    │
│  (恒星/彗星/客星/遗迹/匹配结果, 空间索引)        │
└─────────────────────────────────────────────────┘
```

## 目录结构

```
.
├── backend/                    # Rust 后端
│   ├── Cargo.toml
│   ├── .env                    # 环境变量配置
│   ├── src/
│   │   ├── main.rs             # API 入口 (Actix-Web 路由)
│   │   ├── models.rs           # 数据模型 + API 请求/响应
│   │   ├── db.rs               # 数据库访问层
│   │   ├── astronomy/
│   │   │   ├── mod.rs          # 天文计算入口 + 自行轨迹
│   │   │   ├── constants.rs    # 天文常量与数学工具
│   │   │   └── precession.rs   # 岁差/章动/自行模型
│   │   └── matching/
│   │       ├── mod.rs
│   │       └── bayes.rs        # 贝叶斯匹配引擎
│   └── static/                 # 前端静态文件
│
├── frontend/                   # 前端源码
│   ├── index.html
│   ├── css/style.css
│   └── js/
│       ├── api.js              # API 客户端封装
│       ├── astro.js            # 天文计算工具 (前端精简版)
│       ├── starfield.js        # Three.js 星空渲染引擎
│       ├── ui.js               # UI 控制模块
│       └── app.js              # 应用主入口
│
├── database/
│   ├── scripts/
│   │   ├── 01_init_schema.sql  # PostgreSQL+PostGIS 初始化脚本
│   │   └── seed_data.py        # 模拟数据生成与导入脚本
│   └── data/
```

## 快速开始

### 前置条件

- **Rust** (1.75+): https://rustup.rs
- **PostgreSQL** (14+) + **PostGIS** (3.3+)
- **Python** (3.9+) + `psycopg2-binary` + `numpy`
- **Node.js** (16+, 可选, 仅用于开发服务器)

### 1. 初始化数据库

```bash
# 创建数据库
psql -U postgres -c "CREATE DATABASE ancient_star_catalog;"

# 执行 schema 初始化
psql -U postgres -d ancient_star_catalog -f database/scripts/01_init_schema.sql

# 安装 Python 依赖
pip install psycopg2-binary numpy

# 生成并导入模拟数据 (1200 条恒星 + 彗星 + 客星 + SNR)
python database/scripts/seed_data.py
```

### 2. 启动后端

```bash
cd backend

# 配置环境变量 (编辑 .env 文件修改数据库连接)
# 默认: localhost:5432/ancient_star_catalog user=postgres

# 启动
cargo run --release
# 或开发模式
cargo run
```

后端 API 启动在 `http://localhost:8080`，同时自动托管前端静态文件。

### 3. 访问前端

打开浏览器访问: `http://localhost:8080`

或使用开发服务器:
```bash
cd frontend
npx http-server -p 3000 --cors
```
然后访问 `http://localhost:3000`（需要后端同时运行在 8080 端口）。

## API 接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 健康检查 |
| GET | `/api/dynasties` | 朝代列表 |
| GET | `/api/mansions` | 二十八宿列表 |
| GET | `/api/stars?dynasty_id=&limit=` | 恒星查询 (支持多种筛选) |
| GET | `/api/stars/{id}` | 恒星详情 |
| GET | `/api/stars/{id}/cross-dynasty` | 跨朝代坐标对比 |
| POST | `/api/convert/ruxiu-to-j2000` | 入宿度/去极度 → J2000 坐标转换 |
| POST | `/api/trajectory` | 自行轨迹采样 |
| GET | `/api/comets` | 彗星列表 |
| GET | `/api/guest-stars` | 客星列表 |
| GET | `/api/guest-stars/{id}` | 客星详情 |
| GET | `/api/snr` | 超新星遗迹目录 |
| POST | `/api/match/{guest_id}` | 运行贝叶斯匹配 |
| GET | `/api/match/{guest_id}` | 获取匹配结果 |

### 坐标转换示例

```json
POST /api/convert/ruxiu-to-j2000
{
    "ruxiu_du": 3.5,
    "quji_du": 68.0,
    "mansion_order": 1,
    "epoch_yr": 1054.0,
    "pm_ra_mas": -15.0,
    "pm_dec_mas": 5.0
}
```

### 贝叶斯匹配示例

```
POST /api/match/2?top_k=10
```

返回客星 #2 与所有 SNR 的贝叶斯匹配概率排序。

## 核心算法

### 古代坐标转换模型

```
入宿度/去极度 ──→ 古代赤经/赤纬 ──→ 减章动 ──→ 岁差旋转至 J2000 ──→ 加自行修正
                    (δ = 90° - 去极度)     (IAU 2000B)   (IAU 2006)    (mas/yr × Δt)
```

- **岁差**: IAU 2006 模型 (Vondrak 近似), Z-X-Z Euler 角旋转
- **章动**: IAU 2000B 截断级数 (5 主要项), 对千年尺度数据精度足够
- **自行**: 线性外推 μ_α × Δt / cos(δ), μ_δ × Δt

### 客星证认贝叶斯模型

```
后验 P(M|D) ∝ P(D|M) × P(M)

似然分解:
  P(D|M) = P_spatial × P_temporal × P_magnitude × P_lightcurve

  - P_spatial:  2D 高斯 + Cauchy 长尾混合 (90/10%)
  - P_temporal: Student-t (ν=4), 容许年龄估计偏差
  - P_magnitude: 对数正态, 基于绝对星等-距离模数-消光模型
  - P_lightcurve: Student-t (ν=4), 基于 SN 类型的可见期分布

归一化: softmax → 概率排序
贝叶斯因子: K = P(D|M₁) / P(D|M₂)
```

## 数据说明

### 模拟数据包含

| 类型 | 数量 | 说明 |
|------|------|------|
| 恒星记录 | ~1200 条 | 跨 12 个朝代，来自《甘石星经》等 16 部典籍 |
| 彗星记录 | 28 条 | 汉代至清代各朝代的彗星观测 |
| 客星记录 | 20 条 | 含 SN 1054 (蟹状星云)、SN 1006 等历史超新星 |
| 超新星遗迹 | 50 条 | 含 5 个已知历史 SNR + 45 个模拟 SNR |

### 古代颜色映射

| 古代描述 | 光谱型 | 前端颜色 |
|---------|--------|---------|
| 白 | B/V | #f5f7ff |
| 青 | O/B | #a0c8ff |
| 赤 | M/K | #ffa070 |
| 黄 | G/K | #fff0c0 |
| 苍 | B/A | #c0d8ff |

## 环境变量

| 变量 | 默认值 | 说明 |
|------|-------|------|
| `DB_HOST` | localhost | PostgreSQL 主机 |
| `DB_PORT` | 5432 | PostgreSQL 端口 |
| `DB_NAME` | ancient_star_catalog | 数据库名 |
| `DB_USER` | postgres | 数据库用户 |
| `DB_PASSWORD` | postgres | 数据库密码 |
| `API_HOST` | 127.0.0.1 | API 监听地址 |
| `API_PORT` | 8080 | API 监听端口 |
| `MAX_DB_CONN` | 16 | 最大数据库连接数 |

## 技术栈

- **后端**: Rust + Actix-Web + Tokio + deadpool-postgres
- **数据库**: PostgreSQL 14+ + PostGIS 3.3+
- **前端**: Three.js + Canvas + 原生 JavaScript
- **天文计算**: IAU 2006 岁差 + IAU 2000B 章动 + 贝叶斯推断

## License

MIT
