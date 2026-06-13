/* ============================================================
 * 应用主入口
 *  初始化星空渲染、UI、数据加载管线
 *  处理全局事件 (恒星选中、客星选中、朝代切换等)
 * ============================================================ */

class Application {
    constructor() {
        this.starField = null;
        this.ui = null;

        // 缓存
        this.allStars = [];
        this.dynastyStars = {};  // dynasty_id -> stars[]

        this._init();
    }

    async _init() {
        this._showLoading(true);

        // 初始化渲染器
        this.starField = new StarField('star-canvas');
        window.starField = this.starField;

        // 初始化 UI
        this.ui = new UI(this.starField);
        window.ui = this.ui;

        // 绑定全局事件
        this._bindGlobalEvents();

        // 并行加载基础数据
        try {
            await Promise.all([
                this._loadDynasties(),
                this._loadMansions(),
            ]);

            // 加载彗星和客星
            await Promise.all([
                this._loadComets(),
                this._loadGuestStars(),
                this._loadSnr(),
            ]);

            // 初次加载全部恒星 (如果太多就按朝代分页)
            await this._loadAllStars();

        } catch (e) {
            console.error('Initialization failed:', e);
            this._showError('初始化失败: ' + e.message);
        }

        this._showLoading(false);
    }

    _bindGlobalEvents() {
        // 恒星被选中
        window.onStarSelected = (star) => {
            this.ui._showStarDetail(star);
        };

        // 客星被选中
        window.onGuestSelected = (guest) => {
            this._openMatchPanel(guest);
        };

        // 朝代切换
        window.onDynastyChange = (dynasty) => {
            this._filterStarsByDynasty(dynasty);
        };

        // 对比模式朝代切换
        window.onCompareChange = (compareDynasty) => {
            // 更新对比视图
            if (compareDynasty) {
                console.log('对比:', compareDynasty);
            }
        };
    }

    // ============================================================
    // 数据加载
    // ============================================================

    async _loadDynasties() {
        const list = await window.api.getDynasties();
        this.dynasties = list || [];
        this.starField.setDynasties(this.dynasties);
        this.ui.setDynasties(this.dynasties);
        console.log(`[App] 加载 ${this.dynasties.length} 个朝代`);
    }

    async _loadMansions() {
        const list = await window.api.getMansions();
        this.mansions = list || [];
        this.starField.setMansions(this.mansions);
        this.ui.setMansions(this.mansions);
        console.log(`[App] 加载 ${this.mansions.length} 个星宿`);
    }

    async _loadAllStars() {
        try {
            // 一次加载全部恒星 (1200 条不算多)
            const data = await window.api.getStars({ limit: 2000 });
            this.allStars = Array.isArray(data) ? data : [];
            console.log(`[App] 加载 ${this.allStars.length} 条恒星记录`);

            // 缓存按朝代分组
            this.dynastyStars = {};
            this.allStars.forEach(s => {
                if (!this.dynastyStars[s.dynasty_id]) {
                    this.dynastyStars[s.dynasty_id] = [];
                }
                this.dynastyStars[s.dynasty_id].push(s);
            });

            // 初始渲染全部
            this.starField.setStars(this.allStars);
        } catch (e) {
            console.warn('加载全部恒星失败:', e);
            this.allStars = [];
        }
    }

    async _loadComets() {
        try {
            const list = await window.api.getComets();
            this.starField.setComets(list || []);
            console.log(`[App] 加载 ${(list || []).length} 条彗星记录`);
        } catch (e) {
            console.warn('加载彗星失败:', e);
        }
    }

    async _loadGuestStars() {
        try {
            const list = await window.api.getGuestStars();
            this.guests = list || [];
            this.starField.setGuestStars(this.guests);
            console.log(`[App] 加载 ${this.guests.length} 条客星记录`);
        } catch (e) {
            console.warn('加载客星失败:', e);
        }
    }

    async _loadSnr() {
        try {
            const list = await window.api.getSnr();
            this.snr = list || [];
            this.starField.setSnr(this.snr);
            console.log(`[App] 加载 ${this.snr.length} 条超新星遗迹`);
        } catch (e) {
            console.warn('加载 SNR 失败:', e);
        }
    }

    // ============================================================
    // 视图更新
    // ============================================================

    _filterStarsByDynasty(dynasty) {
        if (!dynasty || this.ui.compareMode) {
            // 对比模式或无朝代：显示全部 (或叠加两朝)
            if (this.ui.compareMode && this.ui.currentDynasty && this.ui.compareDynasty) {
                const id1 = this.ui.currentDynasty.id;
                const id2 = this.ui.compareDynasty.id;
                const filtered = this.allStars.filter(s =>
                    s.dynasty_id === id1 || s.dynasty_id === id2
                );
                this.starField.setStars(filtered);
                // 用颜色区分两朝: 这里简单处理，实际可扩展
                return;
            }
            this.starField.setStars(this.allStars);
            return;
        }

        const list = this.dynastyStars[dynasty.id] || [];
        this.starField.setStars(list);
    }

    // ============================================================
    // 客星匹配面板
    // ============================================================

    async _openMatchPanel(guest) {
        const panel = document.getElementById('match-panel');
        const header = document.getElementById('match-guest-name');
        const list = document.getElementById('matches-list');
        if (!panel || !header || !list) return;

        panel.style.display = 'flex';
        header.textContent = guest.guest_name || guest.guest_id_code || '客星';

        // 填充基本信息
        document.getElementById('match-guest-dynasty').textContent =
            guest.dynasty_name || '-';
        document.getElementById('match-guest-mag').textContent =
            guest.peak_mag != null ? 'm ' + guest.peak_mag.toFixed(1) : '-';
        document.getElementById('match-guest-days').textContent =
            guest.visibility_days ? guest.visibility_days + ' 天' : '-';
        const err = (guest.ra_err || guest.dec_err)
            ? ((guest.ra_err || 0) + (guest.dec_err || 0) / 2).toFixed(2) + '°'
            : '-';
        document.getElementById('match-guest-err').textContent = err;

        // 加载匹配结果 (先查缓存的，再运行)
        list.innerHTML = `
            <div style="text-align:center;padding:40px;">
                <div class="spinner" style="display:inline-block;"></div>
                <div style="margin-top:12px;color:#a0c8ff;">运行贝叶斯匹配中...</div>
            </div>
        `;

        try {
            const result = await window.api.runMatch(guest.id, 10);
            this._renderMatchResults(result, guest);
        } catch (e) {
            list.innerHTML = `
                <div style="padding:20px;text-align:center;color:#c08080;">
                    匹配失败: ${e.message}
                    <br>
                    <small>请确认后端服务已启动且数据库包含 SNR 数据</small>
                </div>
            `;
            // 尝试显示已有保存的结果
            try {
                const saved = await window.api.getMatches(guest.id);
                if (saved && saved.length > 0) {
                    this._renderMatchesList(saved);
                }
            } catch (_) { /* ignore */ }
        }
    }

    _renderMatchResults(result, guest) {
        const list = document.getElementById('matches-list');
        if (!list) return;

        const candidates = (result && result.candidates) || [];
        if (candidates.length === 0) {
            list.innerHTML = `
                <div style="padding:30px;text-align:center;color:#8090b0;">
                    未找到时空匹配的超新星遗迹候选体
                </div>
            `;
            return;
        }

        this._renderMatchesList(candidates);
    }

    _renderMatchesList(candidates) {
        const list = document.getElementById('matches-list');
        if (!list) return;

        list.innerHTML = '';
        candidates.forEach((m, idx) => {
            const card = document.createElement('div');
            card.className = 'match-card' + (idx === 0 ? ' selected' : '');

            const probClass = m.match_probability > 0.5 ? ''
                : m.match_probability > 0.1 ? 'mid' : 'low';
            const pct = (m.match_probability * 100).toFixed(1);

            // 各分项等级
            const spLevel = this._scoreLevel(m.angular_sep_arcmin / 60, 3, 1, 0.25);
            const tmLevel = this._scoreLevel(Math.abs(m.time_delta_yr), 1000, 400, 100);
            const mgLevel = this._scoreLevel(-m.log_p_magnitude, 8, 3, 1);

            // 贝叶斯因子强度判定
            const bfLog10 = Math.log10(Math.max(1, m.bayes_factor || 1));
            let bfText = '';
            if (bfLog10 > 2) bfText = '强证据 K > 100';
            else if (bfLog10 > 1) bfText = '中等证据 K > 10';
            else if (bfLog10 > 0.5) bfText = '弱证据 K > 3';
            else bfText = '不确定';

            card.innerHTML = `
                <div class="match-header">
                    <div style="display:flex;align-items:center;flex:1;">
                        <span class="match-rank">${m.rank_within_guest || (idx + 1)}</span>
                        <span class="match-name">${m.remnant_name || '未知遗迹'}</span>
                        <span class="match-type">${m.remnant_type || 'II'}</span>
                    </div>
                    <div class="match-prob ${probClass}">${pct}%</div>
                </div>

                <div style="font-size:11px;color:#8090b0;margin-top:2px;">
                    角距离: <span style="color:#a0c8ff;">${(m.angular_sep_arcmin || 0).toFixed(1)}'</span>
                    &nbsp;|&nbsp;
                    时间差: <span style="color:#a0c8ff;">${(m.time_delta_yr || 0).toFixed(0)} 年</span>
                    &nbsp;|&nbsp;
                    ΔlogL: <span style="color:#a0c8ff;">${(m.log_posterior || 0).toFixed(2)}</span>
                </div>

                <div class="match-scores">
                    ${this._scoreBar('空间', spLevel)}
                    ${this._scoreBar('时间', tmLevel)}
                    ${this._scoreBar('星等', mgLevel)}
                    ${this._scoreBar('后验', m.match_probability > 0.7 ? 'good' : m.match_probability > 0.2 ? 'warn' : 'danger',
                                     Math.round(m.match_probability * 100) + '%')}
                </div>

                ${m.bayes_factor ? `<div class="bayes-badge">K = ${(m.bayes_factor || 1).toExponential(2)} · ${bfText}</div>` : ''}
            `;

            card.addEventListener('click', () => {
                document.querySelectorAll('.match-card').forEach(c => c.classList.remove('selected'));
                card.classList.add('selected');
                // 定位到遗迹
                const snr = (window.app?.snr || []).find(s =>
                    s.remnant_name === m.remnant_name || s.id === m.remnant_id);
                if (snr) {
                    this.starField.flyTo(snr.ra_deg, snr.dec_deg, 2.8);
                }
            });

            list.appendChild(card);
        });
    }

    _scoreBar(label, level, customValue) {
        let fill = 'danger', width = '30%', val = level;
        if (level === 'good' || level < 0) { fill = ''; width = '85%'; }
        else if (level === 'warn') { fill = 'warn'; width = '55%'; }
        else { fill = 'danger'; width = '25%'; }

        if (customValue) {
            val = customValue;
            if (typeof level === 'string') {
                if (level === 'good') width = '85%';
                else if (level === 'warn') width = '55%';
                else width = '25%';
            }
        }

        return `
            <div class="score-bar">
                <span class="label">${label}</span>
                <div class="bar"><div class="bar-fill ${fill}" style="width:${width};"></div></div>
                <span class="value">${typeof val === 'string' ? val : (level === 'good' ? '优' : level === 'warn' ? '良' : '差')}</span>
            </div>
        `;
    }

    _scoreLevel(value, threshold1, threshold2, threshold3) {
        // 低分好
        if (value < threshold3) return 'good';
        if (value < threshold2) return 'warn';
        return 'danger';
    }

    // ============================================================
    // 状态提示
    // ============================================================

    _showLoading(show) {
        const el = document.getElementById('loading');
        if (el) el.style.display = show ? 'block' : 'none';
    }

    _showError(msg) {
        const el = document.getElementById('loading');
        if (el) {
            el.innerHTML = `<div style="color:#ff8080;">${msg}</div>`;
            setTimeout(() => (el.style.display = 'none'), 5000);
        }
    }
}

// 启动
window.addEventListener('DOMContentLoaded', () => {
    window.app = new Application();
});
