/* ============================================================
 * API 封装模块
 * ============================================================ */

const API_BASE = (window.location.protocol + '//' + window.location.hostname + ':8080')
    || 'http://localhost:8080';

class ApiClient {
    constructor(baseUrl) {
        this.baseUrl = baseUrl || API_BASE;
    }

    async request(path, options = {}) {
        const url = this.baseUrl + path;
        const defaultOptions = {
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json',
            },
            mode: 'cors',
        };
        const opts = { ...defaultOptions, ...options };
        if (options.body && typeof options.body !== 'string') {
            opts.body = JSON.stringify(options.body);
        }

        try {
            const resp = await fetch(url, opts);
            const data = await resp.json();
            if (data.success) {
                return data.data;
            } else {
                throw new Error(data.error || data.message || 'API Error');
            }
        } catch (e) {
            console.error('API request failed:', path, e);
            throw e;
        }
    }

    async health() {
        return this.request('/api/health');
    }

    async getDynasties() {
        return this.request('/api/dynasties');
    }

    async getMansions() {
        return this.request('/api/mansions');
    }

    async getStars(params = {}) {
        const q = new URLSearchParams();
        Object.entries(params).forEach(([k, v]) => {
            if (v !== undefined && v !== null && v !== '') {
                q.append(k, v);
            }
        });
        const path = '/api/stars' + (q.toString() ? '?' + q.toString() : '');
        return this.request(path);
    }

    async getStar(id) {
        return this.request('/api/stars/' + id);
    }

    async getComets(dynastyId) {
        const q = dynastyId ? '?dynasty_id=' + dynastyId : '';
        return this.request('/api/comets' + q);
    }

    async getGuestStars(dynastyId) {
        const q = dynastyId ? '?dynasty_id=' + dynastyId : '';
        return this.request('/api/guest-stars' + q);
    }

    async getGuestStar(id) {
        return this.request('/api/guest-stars/' + id);
    }

    async getSnr() {
        return this.request('/api/snr');
    }

    async convertRuxiuToJ2000(payload) {
        return this.request('/api/convert/ruxiu-to-j2000', {
            method: 'POST',
            body: payload,
        });
    }

    async getTrajectory(payload) {
        return this.request('/api/trajectory', {
            method: 'POST',
            body: payload,
        });
    }

    async getCrossDynasty(starId) {
        return this.request('/api/stars/' + starId + '/cross-dynasty');
    }

    async runMatch(guestId, topK = 20) {
        return this.request('/api/match/' + guestId + '?top_k=' + topK, {
            method: 'POST',
        });
    }

    async getMatches(guestId) {
        return this.request('/api/match/' + guestId);
    }
}

// 全局单例
window.api = new ApiClient();
