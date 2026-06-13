/* ============================================================
 * 应用主入口
 * ============================================================ */

class Application {
    constructor() {
        this.starField = null;
        this.ui = null;
        this.allStars = [];
        this.dynastyStars = {};
        this.guests = [];
        this.snr = [];
        this._init();
    }

    async _init() {
        this._showLoading(true);

        this.starField = new StarField('star-canvas');
        window.starField = this.starField;

        this.ui = new UI(this.starField);
        window.ui = this.ui;

        this._bindGlobalEvents();

        try {
            await Promise.all([this._loadDynasties(), this._loadMansions()]);
            await Promise.all([
                this._loadComets(), this._loadGuestStars(), this._loadSnr()
            ]);
            await this._loadAllStars();
        } catch (e) {
            console.error('Init failed:', e);
            this._showError('初始化失败: ' + e.message);
        }

        this._showLoading(false);
    }

    _bindGlobalEvents() {
        window.onStarSelected = (star) => { this.ui._showStarDetail(star); };
        window.onGuestSelected = (guest) => { this._openMatchPanel(guest); };
        window.onDynastyChange = (dynasty) => { this._filterStarsByDynasty(dynasty); };
        window.onCompareChange = (cmp) => { console.log('对比:', cmp); };
    }

    async _loadDynasties() {
        const list = await window.api.getDynasties();
        this.dynasties = list || [];
        this.starField.setDynasties(this.dynasties);
        this.ui.setDynasties(this.dynasties);
    }
    async _loadMansions() {
        const list = await window.api.getMansions();
        this.mansions = list || [];
        this.starField.setMansions(this.mansions);
        this.ui.setMansions(this.mansions);
    }
    async _loadAllStars() {
        try {
            const data = await window.api.getStars({ limit: 2000 });
            this.allStars = Array.isArray(data) ? data : [];
            this.dynastyStars = {};
            this.allStars.forEach(s => {
                if (!this.dynastyStars[s.dynasty_id]) this.dynastyStars[s.dynasty_id] = [];
                this.dynastyStars[s.dynasty_id].push(s);
            });
            this.starField.setStars(this.allStars);
        } catch (e) {
            console.warn('加载恒星失败:', e);
            this.allStars = [];
        }
    }
    async _loadComets() {
        try {
            const list = await window.api.getComets();
            this.starField.setComets(list || []);
        } catch (e) { console.warn('彗星加载失败:', e); }
    }
    async _loadGuestStars() {
        try {
            const list = await window.api.getGuestStars();
            this.guests = list || [];
            this.starField.setGuestStars(this.guests);
        } catch (e) { console.warn('客星加载失败:', e); }
    }
    async _loadSnr() {
        try {
            const list = await window.api.getSnr();
            this.snr = list || [];
            this.starField.setSnr(this.snr);
        } catch (e) { console.warn('SNR加载失败:', e); }
    }

    _filterStarsByDynasty(dynasty) {
        if (!dynasty || this.ui.compareMode) {
            if (this.ui.compareMode && this.ui.currentDynasty && this.ui.compareDynasty) {
                const id1 = this.ui.currentDynasty.id;
                const id2 = this.ui.compareDynasty.id;
                this.starField.setStars(this.allStars.filter(s =>
                    s.dynasty_id === id1 || s.dynasty_id === id2));
                return;
            }
            this.starField.setStars(this.allStars);
            return;
        }
        const list = this.dynastyStars[dynasty.id] || [];
        this.starField.setStars(list);
    }

    async _openMatchPanel(guest) {
        const panel = document.getElementById('match-panel');
        const header = document.getElementById('match-guest-name');
        const list = document.getElementById('matches-list');
        if (!panel || !header || !list) return;
        panel.style.display = 'flex';
        header.textContent = guest.guest_name || guest.guest_id_code || '客星';

        document.getElementById('match-guest-dynasty').textContent = guest.dynasty_name || '-';
        document.getElementById('match-guest-mag').textContent =
            guest.peak_mag != null ? 'm ' + guest.peak_mag.toFixed(1) : '-';
        document.getElementById('match-guest-days').textContent =
            guest.visibility_days ? guest.visibility_days + ' 天' : '-';
        const err = (guest.ra_err || guest.dec_err)
            ? ((guest.ra_err || 0) + (guest.dec_err || 0)) / 2
            : 0;
        document.getElementById('match-guest-err').textContent = err ? err.toFixed(2) + '°' : '-';

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
                    <br><small>请确认后端服务已启动且数据库包含 SNR 数据</small>
                </div>`;
            try {
                const saved = await window.api.getMatches(guest.id);
                if (saved && saved.length > 0) this._renderMatchesList(saved);
            } catch (_) {}
        }
    }

    _renderMatchResults(result, guest) {
        const candidates = (result && result.candidates) || result || [];
        this._renderMatchesList(candidates);
    }

    _renderMatchesList(candidates) {
        const list = document.getElementById('matches-list');
        if (!list) return;
        if (!candidates.length) {
            list.innerHTML = `<div style="padding:30px;text-align:center;color:#8090b0;">未找到时空匹配的超新星遗迹候选体</div>`;
            return;
        }
        list.innerHTML = '';
        candidates.forEach((m, idx) => {
            const card = document.createElement('div');
            card.className = 'match-card' + (idx === 0 ? ' selected' : '');
            const probClass = m.match_probability > 0.5 ? ''
                : m.match_probability > 0.1 ? 'mid' : 'low';
            const pct = (m.match_probability * 100).toFixed(1);

            const spLevel = this._scoreLevel(m.angular_sep_arcmin / 60, 3, 1, 0.25);
            const tmLevel = this._scoreLevel(Math.abs(m.time_delta_yr), 1000, 400, 100);
            const mgLevel = 'warn';

            const bfLog = Math.log10(Math.max(1, m.bayes_factor || 1));
            let bfText = '';
            if (bfLog > 2) bfText = '强证据 K > 100';
            else if (bfLog > 1) bfText = '中等证据 K > 10';
            else if (bfLog > 0.5) bfText = '弱证据 K > 3';
            else bfText = '不确定';

            // 显示先验来源 (银河分布模型)
            const priorMag = m.log_prior != null ? m.log_prior.toFixed(2) : '-';

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
                    先验 ln P: <span style="color:#a0c8ff;">${priorMag}</span>
                </div>
                <div class="match-scores">
                    ${this._scoreBar('空间', spLevel)}
                    ${this._scoreBar('时间', tmLevel)}
                    ${this._scoreBar('星等', mgLevel)}
                    ${this._scoreBar('后验', m.match_probability > 0.7 ? 'good' : m.match_probability > 0.2 ? 'warn' : 'danger',
                        Math.round(m.match_probability * 100) + '%')}
                </div>
                ${m.bayes_factor ? `<div class="bayes-badge">K = ${(m.bayes_factor).toExponential(2)} · ${bfText}</div>` : ''}
            `;

            card.addEventListener('click', () => {
                document.querySelectorAll('.match-card').forEach(c => c.classList.remove('selected'));
                card.classList.add('selected');
                const snr = (window.app?.snr || []).find(s =>
                    s.remnant_name === m.remnant_name || s.id === m.remnant_id);
                if (snr) this.starField.flyTo(snr.ra_deg, snr.dec_deg, 2.8);
            });

            list.appendChild(card);
        });
    }

    _scoreBar(label, level, customValue) {
        let fill = 'danger', width = '30%', val = level;
        if (level === 'good') { fill = ''; width = '85%'; }
        else if (level === 'warn') { fill = 'warn'; width = '55%'; }

        if (customValue) {
            val = customValue;
            if (level === 'good') width = '85%';
            else if (level === 'warn') width = '55%';
        }
        return `
            <div class="score-bar">
                <span class="label">${label}</span>
                <div class="bar"><div class="bar-fill ${fill}" style="width:${width};"></div></div>
                <span class="value">${typeof val === 'string' ? val : (level === 'good' ? '优' : level === 'warn' ? '良' : '差')}</span>
            </div>`;
    }

    _scoreLevel(value, t1, t2, t3) {
        if (value < t3) return 'good';
        if (value < t2) return 'warn';
        return 'danger';
    }

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

window.addEventListener('DOMContentLoaded', () => { window.app = new Application(); });
