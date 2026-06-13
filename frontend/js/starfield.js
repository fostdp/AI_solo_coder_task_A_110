/* ============================================================
 * 星空渲染引擎 (Three.js)
 *  天球视图 / 平面投影 两种模式
 *  恒星、彗星、客星、超新星遗迹、二十八宿边界
 *  自行箭头标注、对比模式下的双坐标标记
 * ============================================================ */

class StarField {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.renderer = null;
        this.scene = null;
        this.camera = null;
        this.controls = null;

        this.starsGroup = null;
        this.cometsGroup = null;
        this.guestsGroup = null;
        this.snrGroup = null;
        this.constellationGroup = null;
        this.mansionGroup = null;
        this.pmArrowGroup = null;
        this.selectRingGroup = null;
        this.globeGroup = null;

        // 模式: 'sphere' | 'plane' | 'compare'
        this.viewMode = 'sphere';

        // 当前数据
        this.stars = [];
        this.comets = [];
        this.guests = [];
        this.snr = [];
        this.dynasties = [];
        this.mansions = [];

        // 当前选中
        this.selectedStar = null;
        this.selectedGuest = null;
        this.currentDynastyId = null;
        this.compareDynastyId = null;
        this.compareMode = false;

        // 显示选项
        this.displayFilter = 'all';
        this.magThreshold = 6.5;
        this.styleMode = 'ancient';

        // 射线拾取
        this.raycaster = new THREE.Raycaster();
        this.mouse = new THREE.Vector2();
        this.pointCloud = null;  // 用于高效拾取的点云
        this.starDataMap = [];  // point 索引 -> star 对象

        this._init();
        this._animate();
    }

    _init() {
        const w = window.innerWidth;
        const h = window.innerHeight;

        // 渲染器
        this.renderer = new THREE.WebGLRenderer({
            canvas: this.canvas,
            antialias: true,
            alpha: true,
        });
        this.renderer.setPixelRatio(window.devicePixelRatio || 1);
        this.renderer.setSize(w, h);
        this.renderer.setClearColor(0x000000, 0);

        // 场景
        this.scene = new THREE.Scene();

        // 相机
        this.camera = new THREE.PerspectiveCamera(45, w / h, 0.1, 1000);
        this.camera.position.set(0, 0, 3.5);

        // 控制器
        this.controls = new THREE.OrbitControls(this.camera, this.canvas);
        this.controls.enableDamping = true;
        this.controls.dampingFactor = 0.05;
        this.controls.minDistance = 1.2;
        this.controls.maxDistance = 20;
        this.controls.enablePan = false;

        // 组
        this.starsGroup = new THREE.Group();
        this.cometsGroup = new THREE.Group();
        this.guestsGroup = new THREE.Group();
        this.snrGroup = new THREE.Group();
        this.mansionGroup = new THREE.Group();
        this.pmArrowGroup = new THREE.Group();
        this.selectRingGroup = new THREE.Group();
        this.globeGroup = new THREE.Group();

        this.scene.add(this.starsGroup);
        this.scene.add(this.cometsGroup);
        this.scene.add(this.guestsGroup);
        this.scene.add(this.snrGroup);
        this.scene.add(this.mansionGroup);
        this.scene.add(this.pmArrowGroup);
        this.scene.add(this.selectRingGroup);
        this.scene.add(this.globeGroup);

        // 天球背景网格
        this._createGlobe();

        // 事件监听
        window.addEventListener('resize', () => this._onResize());
        this.canvas.addEventListener('click', (e) => this._onClick(e));
        this.canvas.addEventListener('mousemove', (e) => this._onMouseMove(e));

        // 环境光 (微弱)
        const ambient = new THREE.AmbientLight(0x222233, 0.5);
        this.scene.add(ambient);
    }

    _createGlobe() {
        // 天球参考线
        const g = new THREE.Group();

        // 赤道
        const equatorGeom = new THREE.RingGeometry(0.998, 1.002, 128);
        const equatorMat = new THREE.MeshBasicMaterial({
            color: 0x406090, side: THREE.DoubleSide, transparent: true, opacity: 0.3
        });
        const equator = new THREE.Mesh(equatorGeom, equatorMat);
        equator.rotation.x = Math.PI / 2;
        g.add(equator);

        // 黄道
        const eclipticGeom = new THREE.RingGeometry(0.997, 1.003, 128);
        const eclipticMat = new THREE.MeshBasicMaterial({
            color: 0xffaa33, side: THREE.DoubleSide, transparent: true, opacity: 0.3
        });
        const ecliptic = new THREE.Mesh(eclipticGeom, eclipticMat);
        ecliptic.rotation.x = Math.PI / 2;
        ecliptic.rotation.z = (23.5 * Math.PI / 180);
        g.add(ecliptic);

        // 天球透明壳
        const sphereGeom = new THREE.SphereGeometry(0.98, 64, 32);
        const sphereMat = new THREE.MeshBasicMaterial({
            color: 0x000010, side: THREE.BackSide, transparent: true, opacity: 0.3
        });
        const sphere = new THREE.Mesh(sphereGeom, sphereMat);
        g.add(sphere);

        this.globeGroup.add(g);
    }

    _onResize() {
        const w = window.innerWidth;
        const h = window.innerHeight;
        this.camera.aspect = w / h;
        this.camera.updateProjectionMatrix();
        this.renderer.setSize(w, h);
    }

    _onClick(e) {
        const rect = this.canvas.getBoundingClientRect();
        this.mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
        this.mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;

        this.raycaster.setFromCamera(this.mouse, this.camera);

        // 先检测客星 (优先)
        if (this.guestsGroup.visible) {
            const guestHits = this.raycaster.intersectObjects(this.guestsGroup.children, true);
            if (guestHits.length > 0) {
                const obj = guestHits[0].object;
                if (obj.userData && obj.userData.guest) {
                    this._selectGuest(obj.userData.guest);
                    return;
                }
            }
        }

        // 检测恒星点云
        if (this.pointCloud && this.starsGroup.visible) {
            const hits = this.raycaster.intersectObject(this.pointCloud);
            if (hits.length > 0) {
                const idx = hits[0].index;
                if (idx >= 0 && idx < this.starDataMap.length) {
                    const star = this.starDataMap[idx];
                    this._selectStar(star);
                    return;
                }
            }
        }

        // 彗星
        if (this.cometsGroup.visible) {
            const cometHits = this.raycaster.intersectObjects(this.cometsGroup.children, true);
            if (cometHits.length > 0) {
                const obj = cometHits[0].object;
                if (obj.userData && obj.userData.comet) {
                    // 彗星也可以点击
                    return;
                }
            }
        }

        // 点击空白，取消选中
        this._deselectStar();
        this._deselectGuest();
    }

    _onMouseMove(e) {
        const rect = this.canvas.getBoundingClientRect();
        this.mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
        this.mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;

        // tooltip
        const tooltip = document.getElementById('tooltip');
        if (!tooltip) return;

        this.raycaster.setFromCamera(this.mouse, this.camera);

        let hovered = null;
        let type = null;

        if (this.pointCloud) {
            const hits = this.raycaster.intersectObject(this.pointCloud);
            if (hits.length > 0) {
                const idx = hits[0].index;
                if (idx >= 0 && idx < this.starDataMap.length) {
                    hovered = this.starDataMap[idx];
                    type = 'star';
                }
            }
        }

        if (!hovered && this.guestsGroup.visible) {
            const hits = this.raycaster.intersectObjects(this.guestsGroup.children, true);
            if (hits.length > 0 && hits[0].object.userData.guest) {
                hovered = hits[0].object.userData.guest;
                type = 'guest';
            }
        }

        if (hovered) {
            tooltip.style.display = 'block';
            tooltip.style.left = (e.clientX + 12) + 'px';
            tooltip.style.top = (e.clientY + 12) + 'px';
            if (type === 'star') {
                tooltip.innerHTML = `
                    <div class="name">${hovered.star_name_cn}</div>
                    <div>星等: ${hovered.magnitude_num?.toFixed?.(2) || '-'}</div>
                    <div>颜色: ${hovered.color_desc || '-'}</div>
                    <div>朝代: ${hovered.dynasty_name || '-'}</div>
                    <div>来源: ${hovered.source_book || '-'}</div>
                `;
            } else if (type === 'guest') {
                tooltip.innerHTML = `
                    <div class="name" style="color:#ffb74d;">${hovered.guest_name || '客星'}</div>
                    <div>朝代: ${hovered.dynasty_name || '-'}</div>
                    <div>峰值星等: ${hovered.peak_mag?.toFixed?.(1) || '-'}</div>
                    <div>可见: ${hovered.visibility_days || '-'} 天</div>
                `;
            }
            this.canvas.style.cursor = 'pointer';
        } else {
            tooltip.style.display = 'none';
            this.canvas.style.cursor = 'grab';
        }
    }

    // ============================================================
    // 数据加载
    // ============================================================

    setDynasties(dynasties) {
        this.dynasties = dynasties;
    }

    setMansions(mansions) {
        this.mansions = mansions;
        this._renderMansions();
    }

    setStars(stars) {
        this.stars = stars;
        this._renderStars();
    }

    setComets(comets) {
        this.comets = comets;
        this._renderComets();
    }

    setGuestStars(guests) {
        this.guests = guests;
        this._renderGuests();
    }

    setSnr(snr) {
        this.snr = snr;
        this._renderSnr();
    }

    // ============================================================
    // 渲染恒星 (使用 Points + BufferGeometry 高效渲染)
    // ============================================================

    _renderStars() {
        this._clearGroup(this.starsGroup);
        this.starDataMap = [];

        const positions = [];
        const colors = [];
        const sizes = [];

        const filtered = this.stars.filter(s => {
            if (this.magThreshold !== null && s.magnitude_num > this.magThreshold) return false;
            return true;
        });

        const radius = 1.0;

        filtered.forEach((star, i) => {
            const ra = star.ra_j2000 ?? star.ra_ancient_conv ?? 0;
            const dec = star.dec_j2000 ?? star.dec_ancient_conv ?? 0;
            if (ra == null || dec == null) return;

            const pos = Astro.sphereToCartesian(ra, dec, radius);
            positions.push(pos.x, pos.y, pos.z);

            const mag = star.magnitude_num ?? 5;
            const colorStr = Astro.getStarColor(star.color_desc || 'default', mag);
            const color = new THREE.Color(colorStr);
            colors.push(color.r, color.g, color.b);

            const size = Astro.magToSize(mag) * 0.8;
            sizes.push(size);

            this.starDataMap.push(star);
        });

        // 点云几何体
        const geom = new THREE.BufferGeometry();
        geom.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
        geom.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
        geom.setAttribute('size', new THREE.Float32BufferAttribute(sizes, 1));

        // 自定义 ShaderMaterial
        const mat = new THREE.ShaderMaterial({
            uniforms: {
                pixelRatio: { value: window.devicePixelRatio || 1 },
                scale: { value: 300.0 },
            },
            vertexShader: `
                attribute float size;
                attribute vec3 color;
                varying vec3 vColor;
                uniform float scale;
                uniform float pixelRatio;
                void main() {
                    vColor = color;
                    vec4 mvPos = modelViewMatrix * vec4(position, 1.0);
                    gl_Position = projectionMatrix * mvPos;
                    gl_PointSize = size * scale * pixelRatio / -mvPos.z;
                }
            `,
            fragmentShader: `
                varying vec3 vColor;
                void main() {
                    vec2 uv = gl_PointCoord - vec2(0.5);
                    float dist = length(uv);
                    if (dist > 0.5) discard;
                    float alpha = smoothstep(0.5, 0.0, dist);
                    alpha = pow(alpha, 1.5);
                    vec3 col = vColor;
                    // 中心更亮
                    col += vec3(pow(1.0 - dist * 2.0, 2.0) * 0.5);
                    gl_FragColor = vec4(col, alpha);
                }
            `,
            transparent: true,
            depthWrite: false,
            blending: THREE.AdditiveBlending,
        });

        this.pointCloud = new THREE.Points(geom, mat);
        this.starsGroup.add(this.pointCloud);

        // 更新数量统计
        const infoEl = document.getElementById('star-count-info');
        if (infoEl) {
            infoEl.textContent = `${filtered.length} 颗星`;
        }
    }

    // ============================================================
    // 渲染彗星 (菱形符号)
    // ============================================================

    _renderComets() {
        this._clearGroup(this.cometsGroup);
        const radius = 1.005;

        this.comets.forEach(comet => {
            const ra = comet.ra_apparent;
            const dec = comet.dec_apparent;
            if (ra == null || dec == null) return;

            const pos = Astro.sphereToCartesian(ra, dec, radius);

            // 彗星用旋转的菱形 + 拖尾
            const group = new THREE.Group();

            const coreGeom = new THREE.OctahedronGeometry(0.012, 0);
            const coreMat = new THREE.MeshBasicMaterial({ color: 0x00e5ff });
            const core = new THREE.Mesh(coreGeom, coreMat);
            group.add(core);

            // 光晕
            const glowGeom = new THREE.SphereGeometry(0.025, 16, 16);
            const glowMat = new THREE.MeshBasicMaterial({
                color: 0x00e5ff, transparent: true, opacity: 0.3
            });
            const glow = new THREE.Mesh(glowGeom, glowMat);
            group.add(glow);

            group.position.set(pos.x, pos.y, pos.z);
            group.lookAt(0, 0, 0);
            group.userData = { comet };

            this.cometsGroup.add(group);
        });
    }

    // ============================================================
    // 渲染客星 (脉动的红点 + 十字星芒)
    // ============================================================

    _renderGuests() {
        this._clearGroup(this.guestsGroup);
        const radius = 1.01;

        this.guests.forEach(guest => {
            const ra = guest.ra_est;
            const dec = guest.dec_est;
            if (ra == null || dec == null) return;

            const pos = Astro.sphereToCartesian(ra, dec, radius);
            const group = new THREE.Group();

            // 核心点
            const coreGeom = new THREE.SphereGeometry(0.018, 16, 16);
            const coreMat = new THREE.MeshBasicMaterial({ color: 0xff5722 });
            const core = new THREE.Mesh(coreGeom, coreMat);
            group.add(core);

            // 十字星芒
            const crossGroup = new THREE.Group();
            const lineMat = new THREE.LineBasicMaterial({
                color: 0xffb74d, transparent: true, opacity: 0.8
            });
            for (let i = 0; i < 4; i++) {
                const pts = [
                    new THREE.Vector3(0, 0, 0),
                    new THREE.Vector3(
                        Math.cos(i * Math.PI / 2) * 0.035,
                        Math.sin(i * Math.PI / 2) * 0.035,
                        0
                    )
                ];
                const geom = new THREE.BufferGeometry().setFromPoints(pts);
                const line = new THREE.Line(geom, lineMat);
                crossGroup.add(line);
            }
            group.add(crossGroup);

            // 外光晕
            const glowGeom = new THREE.SphereGeometry(0.04, 16, 16);
            const glowMat = new THREE.MeshBasicMaterial({
                color: 0xff5722, transparent: true, opacity: 0.2
            });
            const glow = new THREE.Mesh(glowGeom, glowMat);
            group.add(glow);

            group.position.set(pos.x, pos.y, pos.z);
            group.lookAt(0, 0, 0);
            group.userData = { guest, pulseTime: Math.random() * Math.PI * 2 };

            this.guestsGroup.add(group);
        });
    }

    // ============================================================
    // 渲染超新星遗迹 (空心圆)
    // ============================================================

    _renderSnr() {
        this._clearGroup(this.snrGroup);
        const radius = 0.995;

        this.snr.forEach(s => {
            const pos = Astro.sphereToCartesian(s.ra_deg, s.dec_deg, radius);
            const ringGeom = new THREE.RingGeometry(0.015, 0.02, 24);
            const ringMat = new THREE.MeshBasicMaterial({
                color: 0x9575cd, side: THREE.DoubleSide, transparent: true, opacity: 0.8
            });
            const ring = new THREE.Mesh(ringGeom, ringMat);
            ring.position.set(pos.x, pos.y, pos.z);
            ring.lookAt(0, 0, 0);
            ring.userData = { snr: s };
            this.snrGroup.add(ring);
        });
    }

    // ============================================================
    // 渲染二十八宿边界
    // ============================================================

    _renderMansions() {
        this._clearGroup(this.mansionGroup);
        const radius = 0.99;
        const lineMat = new THREE.LineBasicMaterial({
            color: 0x4466aa, transparent: true, opacity: 0.4
        });

        this.mansions.forEach((m, i) => {
            const nextM = this.mansions[(i + 1) % this.mansions.length];
            const ra1 = m.standard_ra_deg;
            const ra2 = nextM.standard_ra_deg;

            // 赤经圈 (从 -80° 到 +80° 赤纬的弧线)
            const points = [];
            for (let dec = -80; dec <= 80; dec += 5) {
                const p = Astro.sphereToCartesian(ra1, dec, radius);
                points.push(new THREE.Vector3(p.x, p.y, p.z));
            }
            const geom = new THREE.BufferGeometry().setFromPoints(points);
            const line = new THREE.Line(geom, lineMat);
            line.userData = { mansion: m };
            this.mansionGroup.add(line);
        });

        // 宿名标签 (简化为点标记)
        this.mansions.forEach(m => {
            const pos = Astro.sphereToCartesian(m.standard_ra_deg + 5, 0, radius * 0.97);
            const dotGeom = new THREE.SphereGeometry(0.005, 8, 8);
            const dotMat = new THREE.MeshBasicMaterial({ color: 0x6699cc });
            const dot = new THREE.Mesh(dotGeom, dotMat);
            dot.position.set(pos.x, pos.y, pos.z);
            dot.userData = { mansion: m };
            this.mansionGroup.add(dot);
        });
    }

    // ============================================================
    // 选中效果
    // ============================================================

    _selectStar(star) {
        this.selectedStar = star;
        this._renderSelectRing();
        this._renderProperMotionArrow(star);

        // 触发全局事件
        if (window.onStarSelected) {
            window.onStarSelected(star);
        }
    }

    _deselectStar() {
        this.selectedStar = null;
        this._clearGroup(this.selectRingGroup);
        this._clearGroup(this.pmArrowGroup);
    }

    _selectGuest(guest) {
        this.selectedGuest = guest;
        if (window.onGuestSelected) {
            window.onGuestSelected(guest);
        }
    }

    _deselectGuest() {
        this.selectedGuest = null;
    }

    _renderSelectRing() {
        this._clearGroup(this.selectRingGroup);
        if (!this.selectedStar) return;

        const ra = this.selectedStar.ra_j2000 ?? this.selectedStar.ra_ancient_conv;
        const dec = this.selectedStar.dec_j2000 ?? this.selectedStar.dec_ancient_conv;
        const pos = Astro.sphereToCartesian(ra, dec, 1.02);

        const ringGeom = new THREE.RingGeometry(0.025, 0.032, 32);
        const ringMat = new THREE.MeshBasicMaterial({
            color: 0x66ccff, side: THREE.DoubleSide, transparent: true, opacity: 0.9
        });
        const ring = new THREE.Mesh(ringGeom, ringMat);
        ring.position.set(pos.x, pos.y, pos.z);
        ring.lookAt(0, 0, 0);
        this.selectRingGroup.add(ring);

        // 外扩波纹
        const waveGeom = new THREE.RingGeometry(0.03, 0.035, 32);
        const waveMat = new THREE.MeshBasicMaterial({
            color: 0x66ccff, side: THREE.DoubleSide, transparent: true, opacity: 0.5
        });
        const wave = new THREE.Mesh(waveGeom, waveMat);
        wave.position.set(pos.x, pos.y, pos.z);
        wave.lookAt(0, 0, 0);
        wave.userData = { isWave: true, t: 0 };
        this.selectRingGroup.add(wave);
    }

    _renderProperMotionArrow(star) {
        this._clearGroup(this.pmArrowGroup);
        const pmRa = star.proper_motion_ra;
        const pmDec = star.proper_motion_dec;
        if (pmRa == null || pmDec == null) return;
        if (Math.abs(pmRa) < 1 && Math.abs(pmDec) < 1) return; // 太小不显示

        const ra0 = star.ra_j2000 ?? star.ra_ancient_conv;
        const dec0 = star.dec_j2000 ?? star.dec_ancient_conv;

        // 放大 500 年的位移，便于可视化
        const years = 500;
        const cosDec = Math.cos(dec0 * Astro.DEG2RAD) || 1e-9;
        const dRa = (pmRa / 3600000) * years / cosDec;
        const dDec = (pmDec / 3600000) * years;

        const ra1 = Astro.norm360(ra0 + dRa);
        const dec1 = Astro.clamp(dec0 + dDec, -89, 89);

        const start = Astro.sphereToCartesian(ra0, dec0, 1.03);
        const end   = Astro.sphereToCartesian(ra1, dec1, 1.03);

        // 画弧线 (用多点近似)
        const points = [];
        const n = 20;
        for (let i = 0; i <= n; i++) {
            const t = i / n;
            const ra = ra0 + (ra1 - ra0) * t;
            const dec = dec0 + (dec1 - dec0) * t;
            const p = Astro.sphereToCartesian(ra, dec, 1.03);
            points.push(new THREE.Vector3(p.x, p.y, p.z));
        }
        const lineGeom = new THREE.BufferGeometry().setFromPoints(points);
        const lineMat = new THREE.LineBasicMaterial({
            color: 0xffcc00, transparent: true, opacity: 0.9
        });
        const line = new THREE.Line(lineGeom, lineMat);
        this.pmArrowGroup.add(line);

        // 箭头 (用锥体)
        const coneGeom = new THREE.ConeGeometry(0.012, 0.03, 8);
        const coneMat = new THREE.MeshBasicMaterial({ color: 0xffcc00 });
        const cone = new THREE.Mesh(coneGeom, coneMat);
        cone.position.set(end.x, end.y, end.z);
        cone.lookAt(start.x, start.y, start.z);
        cone.rotateX(-Math.PI / 2);
        this.pmArrowGroup.add(cone);
    }

    // ============================================================
    // 对比模式: 同一颗星在两个朝代的位置都显示
    // ============================================================

    setCompareMode(enabled, dynastyId1, dynastyId2) {
        this.compareMode = enabled;
        this.compareDynastyId = dynastyId2;
    }

    // ============================================================
    // 显示控制
    // ============================================================

    setDisplayFilter(filter) {
        this.displayFilter = filter;
        const all = filter === 'all';
        this.starsGroup.visible = all || filter === 'stars';
        this.cometsGroup.visible = all || filter === 'comets';
        this.guestsGroup.visible = all || filter === 'guests';
        this.snrGroup.visible = all || filter === 'snr';
        this.mansionGroup.visible = true;
    }

    setMagThreshold(val) {
        this.magThreshold = val;
        this._renderStars();
    }

    setStyleMode(mode) {
        this.styleMode = mode;
        // 切换风格主要改变星颜色和背景
    }

    setViewMode(mode) {
        this.viewMode = mode;
        if (mode === 'sphere') {
            this.camera.position.set(0, 0, 3.5);
            this.globeGroup.visible = true;
        } else if (mode === 'plane') {
            // 平面投影 - 俯视南天
            this.camera.position.set(0, 3, 0);
            this.globeGroup.visible = true;
        } else if (mode === 'compare') {
            this.camera.position.set(0, 0, 3.5);
            this.globeGroup.visible = true;
        }
        this.controls.reset();
    }

    // ============================================================
    // 工具
    // ============================================================

    _clearGroup(group) {
        while (group.children.length > 0) {
            const obj = group.children[0];
            group.remove(obj);
            if (obj.geometry) obj.geometry.dispose?.();
            if (obj.material) {
                if (Array.isArray(obj.material)) {
                    obj.material.forEach(m => m.dispose?.());
                } else {
                    obj.material.dispose?.();
                }
            }
        }
    }

    // ============================================================
    // 动画循环
    // ============================================================

    _animate() {
        requestAnimationFrame(() => this._animate());

        const t = performance.now() * 0.001;

        // 客星脉动
        this.guestsGroup.children.forEach(g => {
            if (g.userData && g.userData.guest) {
                const pulse = 1 + Math.sin(t * 2 + (g.userData.pulseTime || 0)) * 0.15;
                g.scale.set(pulse, pulse, pulse);
            }
        });

        // 选中波纹
        this.selectRingGroup.children.forEach(w => {
            if (w.userData && w.userData.isWave) {
                w.userData.t += 0.01;
                const s = 1 + (w.userData.t % 1);
                w.scale.set(s, s, s);
                w.material.opacity = (1 - (w.userData.t % 1)) * 0.5;
            }
        });

        this.controls.update();
        this.renderer.render(this.scene, this.camera);
    }

    // 相机定位到某星
    flyTo(ra, dec, distance = 2.5) {
        const pos = Astro.sphereToCartesian(ra, dec, distance);
        // 平滑过渡
        const startPos = this.camera.position.clone();
        const endPos = new THREE.Vector3(pos.x, pos.y, pos.z);
        let progress = 0;
        const duration = 800;
        const startTime = performance.now();

        const animate = () => {
            const now = performance.now();
            progress = Math.min(1, (now - startTime) / duration);
            const eased = 1 - Math.pow(1 - progress, 3);
            this.camera.position.lerpVectors(startPos, endPos, eased);
            this.controls.update();
            if (progress < 1) {
                requestAnimationFrame(animate);
            }
        };
        animate();
    }
}

window.StarField = StarField;
