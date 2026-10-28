// 界面工具包：每个视图（面板）= 一块离屏 2D 画布 + 一个贴图平面，叠在 3D 上画（正交相机，逻辑像素，y 向下）。
//
// 为什么这样分：本引擎的 2D 画布是 CPU 光栅化（一加 PLY110 上整屏渐变 9~30ms），所以
// - 视图只在 dirty 时重画（数字变了、开关切了），平时每帧只是一次 draw call；
// - 列表滚动不重画：内容整张画在一块高画布上，滚动只改纹理的 offset / repeat（UV 窗口），带惯性与回弹；
// - 按下的反馈是盖一块半透明的圆角块（与按钮同一个圆角 h/2.4），不重画视图；入场 / 退场动画只动平面的位置、透明度、缩放。
// 一块画布改了尺寸就换一张新纹理（three.js 在 WebGL2 上 texStorage2D 一次定死纹理尺寸，之后按原尺寸传会失败）。
function make(THREE, renderer, W, H, dpr) {
  var scene = new THREE.Scene(), cam = new THREE.OrthographicCamera(0, W, 0, -H, -50, 50);
  var plane = new THREE.PlaneGeometry(1, 1), views = [], byId = {}, SD = Math.min(2, dpr);
  function mat(opts) { var m = new THREE.MeshBasicMaterial(opts); m.transparent = true; m.depthTest = false; m.depthWrite = false; return m; }
  function quad(color, alpha) { var q = new THREE.Mesh(plane, mat({ color: color, opacity: alpha })); q.visible = false; scene.add(q); return q; }
  function place(m, x, y, w, h) { m.scale.set(Math.max(0.001, w), Math.max(0.001, h), 1); m.position.set(x + w / 2, -(y + h / 2), m.position.z); }
  var press = quad(0x000000, 0.18);
  // 按下的遮罩做成圆角：方块盖在圆角按钮上，四个角会多出一圈方的暗边（像渲染错了）。
  // 圆角半径与 draw.js 的 button 一致（h / 2.4，再不超过半宽）；遮罩按「宽×高」缓存，按下时才画一次（几十像素的小画布）
  var pressTex = {};
  function pressMask(w, h) {
    var k = Math.round(w) + 'x' + Math.round(h);
    if (pressTex[k]) return pressTex[k];
    var cv = wx.createCanvas(), sd = SD;
    cv.width = Math.max(1, Math.ceil(w * sd)); cv.height = Math.max(1, Math.ceil(h * sd));
    var c = cv.getContext('2d');
    c.setTransform(sd, 0, 0, sd, 0, 0); c.fillStyle = '#fff';
    rr(c, 0, 0, w, h, Math.min(h / 2.4, w / 2)); c.fill();
    var t = newTex(cv);
    pressTex[k] = t;
    return t;
  }

  function newTex(cv) {
    var t = new THREE.CanvasTexture(cv);
    t.minFilter = THREE.LinearFilter; t.magFilter = THREE.LinearFilter; t.generateMipmaps = false;
    if (THREE.SRGBColorSpace) t.colorSpace = THREE.SRGBColorSpace;
    return t;
  }
  // ───────── 视图 ─────────
  // o: { x, y, w, h, z, draw(c, v), onTap(id, v, lx, ly), contentH（滚动视图：内容高度）, modal（带暗底、挡住下面）,
  //      anim: 'up' | 'pop' | 'fade' | 'right'，onBackdrop() }
  function view(id, o) {
    var v = { id: id, x: o.x, y: o.y, w: o.w, h: o.h, z: o.z || 0, draw: o.draw, onTap: o.onTap, modal: !!o.modal, anim: o.anim || 'fade',
      onBackdrop: o.onBackdrop, partial: o.partial || null, follow: o.follow || null, contentH: o.contentH || o.h, scrollY: 0, vy: 0, hits: [], dirty: true, shown: false, t: 0, target: 0, data: o.data || {} };
    v.cv = wx.createCanvas();
    v.mesh = new THREE.Mesh(plane, mat({ color: 0xffffff }));
    v.mesh.visible = false;
    v.dim = v.modal ? quad(0x0b1020, 0.7) : null;
    scene.add(v.mesh);
    v.markDirty = function () { v.dirty = true; };
    v.hit = function (hid, x, y, w, h, extra) { v.hits.push({ id: hid, x: x, y: y, w: w, h: h, extra: extra }); };
    v.setContent = function (ch) { if (Math.abs(ch - v.contentH) > 0.5) { v.contentH = Math.max(v.h, ch); v.dirty = true; } };
    v.scrollTo = function (y) { v.scrollY = Math.max(0, Math.min(v.contentH - v.h, y)); v.vy = 0; };
    v.show = function () { if (!v.shown) { v.shown = true; v.target = 1; v.dirty = true; } };
    v.hide = function () { v.shown = false; v.target = 0; };
    views.push(v); byId[id] = v;
    views.sort(function (a, b) { return a.z - b.z; });
    return v;
  }
  // 倒计时小条：盖在父视图上的一小块画布，每秒只重画自己（几百像素）。
  // 父面板（任务 / 商店 700×2000 的画布）每秒整块重画 + 整张贴图重传，真机上一次 60~110ms，就是 v4 每秒顿一下的来源。
  // (x, y) 是父视图内容坐标，跟着父视图滚；when() 为假时不显示；不收触摸（passive）。
  var tickers = [];
  function ticker(parent, id, x, y, w, h, draw, when) {
    var t = view(id, { x: parent.x + x, y: parent.y + y, w: w, h: h, z: parent.z + 0.5, follow: parent, draw: draw });
    t.passive = true; t.scrollWith = true; t.when = when || null; t.sec = -1;
    tickers.push(t);
    return t;
  }
  // draw 里可以 setContent 改内容高度：改了就按新高度再画一遍（否则画布还是旧高度，显示时被拉伸成「放大」的样子）
  function redraw(v) {
    var before = v.contentH;
    redrawOnce(v);
    if (Math.abs(v.contentH - before) > 0.5) redrawOnce(v);
  }
  function redrawOnce(v) {
    var ch = Math.max(v.h, v.contentH), sd = Math.min(SD, 4096 / ch);
    var pw = Math.ceil(v.w * sd), ph = Math.ceil(ch * sd);
    var keep = !!v.tex && v.cv.width === pw && v.cv.height === ph;
    if (!keep) {
      v.cv.width = pw; v.cv.height = ph;
      if (v.tex) v.tex.dispose();
      v.tex = newTex(v.cv); v.mesh.material.map = v.tex; v.mesh.material.needsUpdate = true;
    }
    var c = v.cv.getContext('2d');
    // partial(c, v)：画布尺寸没变时只擦 / 画变了的那几块（hud-top），hits 留着上次整画的；返回 false 就照常整块重画
    if (v.partial && v.drawn && keep) {
      c.setTransform(sd, 0, 0, sd, 0, 0);
      if (v.partial(c, v)) { v.tex.needsUpdate = true; v.dirty = false; return; }
    }
    c.setTransform(1, 0, 0, 1, 0, 0); c.clearRect(0, 0, pw, ph);
    c.setTransform(sd, 0, 0, sd, 0, 0);
    v.hits = [];
    v.draw(c, v);
    v.tex.needsUpdate = true;
    v.dirty = false; v.drawn = true;
  }
  function ease(t) { var c1 = 1.4, c3 = c1 + 1; return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2); }   // easeOutBack
  // 视图在屏上的位置（含动画）
  function frameOf(v) {
    // 跟随视图（follow）：嵌在父视图里的滚动列表，位置 / 缩放 / 透明度全跟父视图走（一起滑入，不会错位）
    if (v.follow) { var p = v.follow, pf = frameOf(p); return { x: pf.x + (v.x - p.x) * pf.s, y: pf.y + (v.y - p.y - (v.scrollWith ? p.scrollY : 0)) * pf.s, w: v.w * pf.s, h: v.h * pf.s, s: pf.s, a: pf.a }; }
    var k = v.t, e = v.target ? ease(k) : k, dx = 0, dy = 0, s = 1;
    if (v.anim === 'up') dy = (1 - e) * (H - v.y);
    else if (v.anim === 'right') dx = (1 - e) * (W - v.x);
    else if (v.anim === 'pop') s = 0.6 + 0.4 * e;
    var w = v.w * s, h = v.h * s;
    return { x: v.x + dx + (v.w - w) / 2, y: v.y + dy + (v.h - h) / 2, w: w, h: h, s: s, a: Math.min(1, k * 1.5) };
  }

  // ───────── 触摸：最上面的、点在里面的视图收；开着 modal 时下面的都收不到 ─────────
  var active = null;
  function topAt(x, y) {
    for (var i = views.length - 1; i >= 0; i--) {
      var v = views[i]; if (v.passive) continue;
      if (!v.shown || v.t < 0.6) { if (v.shown && v.modal) return { v: v, inside: false }; continue; }
      var f = frameOf(v);
      if (x >= f.x && x <= f.x + f.w && y >= f.y && y <= f.y + f.h) return { v: v, inside: true, f: f };
      if (v.modal) return { v: v, inside: false };
    }
    return null;
  }
  function hitAt(v, f, x, y) {
    var lx = (x - f.x) / f.s, ly = (y - f.y) / f.s + v.scrollY;
    for (var i = v.hits.length - 1; i >= 0; i--) { var h = v.hits[i]; if (lx >= h.x && lx <= h.x + h.w && ly >= h.y && ly <= h.y + h.h) return { hit: h, lx: lx, ly: ly }; }
    return { hit: null, lx: lx, ly: ly };
  }
  function down(x, y) {
    var t = topAt(x, y);
    if (!t) return false;
    if (!t.inside) { active = { backdrop: t.v }; return true; }
    var r = hitAt(t.v, t.f, x, y);
    active = { v: t.v, f: t.f, x0: x, y0: y, s0: t.v.scrollY, hit: r.hit, lx: r.lx, ly: r.ly, moved: false, hist: [], t0: Date.now() };
    return true;
  }
  function move(x, y) {
    if (!active || !active.v) return !!active;
    var a = active, v = a.v, dy = y - a.y0;
    if (!a.moved && Math.abs(dy) + Math.abs(x - a.x0) > 9) a.moved = true;
    if (a.moved && v.contentH > v.h + 1) {
      var maxS = v.contentH - v.h, s = a.s0 - dy;
      if (s < 0) s = s * 0.35; else if (s > maxS) s = maxS + (s - maxS) * 0.35;     // 拉过头：阻尼回弹
      v.scrollY = s;
      a.hist.push({ y: y, t: Date.now() }); a.hist = a.hist.filter(function (h) { return Date.now() - h.t < 90; });
    }
    return true;
  }
  function up(x, y) {
    var a = active; active = null;
    if (!a) return false;
    if (a.backdrop) { if (a.backdrop.onBackdrop) a.backdrop.onBackdrop(); return true; }
    if (a.moved && a.hist.length > 1) { var h0 = a.hist[0], span = Math.max(16, Date.now() - h0.t); a.v.vy = -(y - h0.y) / span * 1000; }
    if (!a.moved && a.hit && a.v.onTap) a.v.onTap(a.hit.id, a.v, a.lx - a.hit.x, a.ly - a.hit.y, a.hit.extra);
    return true;
  }

  // ───────── 提示条（游戏内的，系统 wx.showToast 留给支付等系统流程）与「飞到 HUD」的图标 ─────────
  var toasts = [], flies = [];
  var toastView = view('__toast', { x: W * 0.12, y: H * 0.3, w: W * 0.76, h: 46, z: 90, anim: 'pop', draw: function (c, v) {
    var t = v.data.text || '', ic = v.data.icon;
    c.fillStyle = 'rgba(20,24,40,0.85)'; rr(c, 0, 0, v.w, v.h, 23); c.fill();
    c.font = 'bold 16px sans-serif'; c.textBaseline = 'middle'; c.textAlign = 'center'; c.fillStyle = '#fff';
    var tw = c.measureText(t).width + (ic ? 30 : 0), x0 = (v.w - tw) / 2;
    if (ic && api.icons) api.icons.draw(c, ic, x0, 9, 28);
    c.textAlign = 'left'; c.fillText(t, x0 + (ic ? 32 : 0), v.h / 2 + 1);
  } });
  var toastT = 0;
  function toast(text, icon) { toastView.data = { text: text, icon: icon }; toastView.dirty = true; toastView.show(); toastT = 1.8; }
  function fly(icon, fx, fy, tx, ty, n) {
    if (!api.icons) return;
    for (var i = 0; i < (n || 6); i++) {
      var m = new THREE.Mesh(plane, mat({ map: api.icons.texture(icon) }));
      m.position.z = 40; scene.add(m);
      flies.push({ m: m, t: -i * 0.05, fx: fx + (Math.random() - 0.5) * 50, fy: fy + (Math.random() - 0.5) * 30, tx: tx, ty: ty });
    }
  }
  function rr(c, x, y, w, h, r) { r = Math.min(r, w / 2, h / 2); c.beginPath(); c.moveTo(x + r, y); c.arcTo(x + w, y, x + w, y + h, r); c.arcTo(x + w, y + h, x, y + h, r); c.arcTo(x, y + h, x, y, r); c.arcTo(x, y, x + w, y, r); c.closePath(); }

  var api = {
    prof: {},   // 本帧各视图重画耗时（game.js 的 prof=1 打出来后清空）
    view: view, ticker: ticker, get: function (id) { return byId[id]; }, down: down, move: move, up: up, toast: toast, fly: fly, icons: null,
    anyModal: function () { return views.some(function (v) { return v.shown && v.modal; }); },
    hideAll: function (except) { views.forEach(function (v) { if (v.modal && v.id !== except) v.hide(); }); },
    step: function (dt) {
      if (toastT > 0) { toastT -= dt; if (toastT <= 0) toastView.hide(); }
      var sec = Math.floor(Date.now() / 1000);
      tickers.forEach(function (t) { if (t.follow.shown && t.sec !== sec) { t.sec = sec; t.dirty = true; } });
      views.forEach(function (v) {
        if (v.follow) { if (v.follow.shown && !v.shown) v.dirty = true; v.shown = v.follow.shown; v.target = v.follow.target; v.t = v.follow.t; }   // 父视图 z 更小，先更新过了
        // 刚打开、还没画过：这一帧先不动，等 render 把面板画好（大面板 60~110ms），下一帧再开始滑入。
        // 否则长帧正好落在入场动画第一帧，面板一下子跳出半截 —— 点了之后慢 0.1 秒没人察觉，动画跳一下一眼就看出来
        else if (v.target && v.t === 0 && v.dirty) { /* 等首帧画好 */ }
        else v.t += (v.target - v.t) * Math.min(1, dt * (v.target ? 9 : 14));
        if (!v.target && v.t < 0.02) v.t = 0;
        // 惯性滚动 + 回弹
        if (v.contentH > v.h && (!active || active.v !== v)) {
          var maxS = v.contentH - v.h;
          if (v.scrollY < 0) { v.scrollY += (0 - v.scrollY) * Math.min(1, dt * 12); v.vy = 0; }
          else if (v.scrollY > maxS) { v.scrollY += (maxS - v.scrollY) * Math.min(1, dt * 12); v.vy = 0; }
          else if (Math.abs(v.vy) > 5) { v.scrollY += v.vy * dt; v.vy *= Math.exp(-dt * 3.5); } else v.vy = 0;
        }
      });
      flies = flies.filter(function (f) {
        f.t += dt * 1.6;
        if (f.t < 0) { f.m.visible = false; return true; }
        var k = Math.min(1, f.t), e = k * k, x = f.fx + (f.tx - f.fx) * e, y = f.fy + (f.ty - f.fy) * e - Math.sin(k * Math.PI) * 60;
        f.m.visible = true; place(f.m, x - 14, y - 14, 28, 28);
        if (k >= 1) { scene.remove(f.m); f.m.material.dispose(); if (api.onFlyDone) api.onFlyDone(); return false; }
        return true;
      });
    },
    render: function () {
      var z = 0;
      views.forEach(function (v) {
        // 贴图也在这一帧传上去（initTexture）：不然要等第一次可见才上传，长帧又落回动画第一帧
        if (v.shown && v.t === 0 && v.dirty && !v.passive) { var t1 = Date.now(); redraw(v); if (renderer.initTexture) renderer.initTexture(v.tex); api.prof[v.id] = (api.prof[v.id] || 0) + Date.now() - t1; }
        var vis = v.t > 0.01 && (!v.when || v.when()), f;
        // 跟着父视图滚的小条：滚出父视图的可见窗口就不画（没有裁剪，半截露在面板外面更难看）
        if (vis && v.scrollWith) { f = frameOf(v); var pf = frameOf(v.follow); vis = f.y >= pf.y - 0.5 && f.y + f.h <= pf.y + pf.h + 0.5; }
        v.mesh.visible = vis; if (v.dim) v.dim.visible = vis;
        if (!vis) return;
        if (v.dirty) { var t0 = Date.now(); redraw(v); api.prof[v.id] = (api.prof[v.id] || 0) + Date.now() - t0; }
        f = frameOf(v);
        if (v.dim) { v.dim.material.opacity = 0.7 * Math.min(1, v.t * 1.4); v.dim.position.z = z++ * 0.01; place(v.dim, 0, 0, W, H); }
        v.mesh.position.z = z++ * 0.01;
        place(v.mesh, f.x, f.y, f.w, f.h);
        v.mesh.material.opacity = f.a;
        var ch = Math.max(v.h, v.contentH);
        v.tex.repeat.set(1, v.h / ch); v.tex.offset.set(0, 1 - (Math.max(0, Math.min(ch - v.h, v.scrollY)) + v.h) / ch);
      });
      press.visible = false;
      if (active && active.v && active.hit && !active.moved) {
        var a = active, f = frameOf(a.v), h = a.hit;
        var y0 = f.y + (h.y - a.v.scrollY) * f.s, y1 = y0 + h.h * f.s;
        y0 = Math.max(f.y, y0); y1 = Math.min(f.y + f.h, y1);
        if (y1 > y0) {
          var m = pressMask(h.w, h.h), full = h.h * f.s, top = (y0 - (f.y + (h.y - a.v.scrollY) * f.s)) / full;
          if (press.material.map !== m) { press.material.map = m; press.material.needsUpdate = true; }
          // 滚动视图里按钮被视图上下边裁掉一截时，遮罩也只取露出来的那段（UV 窗口），圆角不被压扁
          m.repeat.set(1, (y1 - y0) / full); m.offset.set(0, 1 - top - (y1 - y0) / full);
          press.visible = true; press.position.z = z++ * 0.01; place(press, f.x + h.x * f.s, y0, h.w * f.s, y1 - y0);
        }
      }
      var ac = renderer.autoClear;
      renderer.autoClear = false; renderer.clearDepth();
      renderer.render(scene, cam);
      renderer.autoClear = ac;
    }
  };
  return api;
}

module.exports = { make: make };
