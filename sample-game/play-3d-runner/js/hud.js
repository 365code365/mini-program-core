// three.js 游戏的 2D 界面层（三个 play-3d-* 共用，各拷一份）：正交相机 + 一个个平面，叠在 3D 画面上画。
// 文字：每个 key 一块小离屏 2D 画布 → CanvasTexture，内容（字 / 字号 / 颜色）不变就不重画、不重传；
// 纯色块（遮罩、按钮底）是普通平面。坐标是逻辑像素、左上角原点，与 Canvas 2D 一致。
//
// 每帧用法：hud.begin() → hud.text(...) / hud.rect(...) / hud.button(...) → hud.render()。
// 这一帧没调到的 key 自动隐藏（不用自己管显隐）。
function make(THREE, renderer, W, H, dpr) {
  var scene = new THREE.Scene();
  var cam = new THREE.OrthographicCamera(0, W, 0, -H, -10, 10);      // y 向下：摆的时候 y 取负
  var plane = new THREE.PlaneGeometry(1, 1);
  var items = {}, frame = 0, order = 0;

  function get(key, make) {
    var it = items[key];
    if (!it) { it = items[key] = make(); scene.add(it.mesh); }
    it.used = frame;
    it.mesh.visible = true;
    it.mesh.renderOrder = order++;                                     // 先调的先画（后调的盖在上面）
    return it;
  }
  function place(mesh, x, y, w, h) {
    mesh.scale.set(w, h, 1);
    mesh.position.set(x + w / 2, -(y + h / 2), 0);
  }
  function basic(opts) {
    var m = new THREE.MeshBasicMaterial(opts);
    m.transparent = true; m.depthTest = false; m.depthWrite = false;
    return m;
  }

  function rect(key, x, y, w, h, color, alpha) {
    var it = get('r:' + key, function () { return { mesh: new THREE.Mesh(plane, basic({ color: 0xffffff })) }; });
    it.mesh.material.color.set(color);
    it.mesh.material.opacity = alpha === undefined ? 1 : alpha;
    place(it.mesh, x, y, w, h);
  }
  // align 'left' | 'center' | 'right'；y 是文字中线。opts：{ alpha, outline: [颜色, 宽], bg: 胶囊底色, padX, padY }
  // 返回包围盒（按钮命中用）
  function text(key, s, x, y, size, color, align, weight, opts) {
    opts = opts || {};
    s = String(s);
    var sig = s + '|' + size + '|' + color + '|' + (weight || '') + '|' + (opts.outline || '') + '|' + (opts.bg || '') + '|' + (opts.padX || 0);
    var it = get('t:' + key, function () {
      var cv = wx.createCanvas(), tex = new THREE.CanvasTexture(cv);
      tex.minFilter = THREE.LinearFilter; tex.generateMipmaps = false;   // 尺寸不是 2 的幂：不用 mipmap
      if (THREE.SRGBColorSpace) tex.colorSpace = THREE.SRGBColorSpace;
      return { cv: cv, tex: tex, mesh: new THREE.Mesh(plane, basic({ map: tex })), sig: null, w: 1, h: 1 };
    });
    if (it.sig !== sig) {
      it.sig = sig;
      var c = it.cv.getContext('2d'), font = (weight || 'normal') + ' ' + Math.round(size * dpr) + 'px sans-serif';
      c.font = font;
      var ow = opts.outline ? opts.outline[1] : 0, padX = opts.padX || 0, padY = opts.padY || 0;
      it.w = Math.ceil(c.measureText(s).width / dpr + 4 + ow * 2 + padX * 2);
      it.h = Math.ceil(size * 1.35 + 4 + ow * 2 + padY * 2);
      var pw = Math.ceil(it.w * dpr), ph = Math.ceil(it.h * dpr);
      if (pw !== it.cv.width || ph !== it.cv.height) {
        it.cv.width = pw; it.cv.height = ph;
        // 尺寸变了就换一张新纹理：three.js 在 WebGL2 上用 texStorage2D 一次定死纹理大小，之后 needsUpdate 只按原尺寸
        // texSubImage2D 上传，画布变大的那次上传失败、屏上一直是旧字（分数停在 9 不动就是这个）
        it.tex.dispose();
        it.tex = new THREE.CanvasTexture(it.cv);
        it.tex.minFilter = THREE.LinearFilter; it.tex.generateMipmaps = false;
        if (THREE.SRGBColorSpace) it.tex.colorSpace = THREE.SRGBColorSpace;
        it.mesh.material.map = it.tex;
      }
      c = it.cv.getContext('2d');
      c.clearRect(0, 0, it.cv.width, it.cv.height);
      if (opts.bg) {
        var r = it.cv.height / 2, cw = it.cv.width;
        c.fillStyle = opts.bg;
        c.beginPath(); c.moveTo(r, 0); c.lineTo(cw - r, 0); c.arc(cw - r, r, r, -Math.PI / 2, Math.PI / 2);
        c.lineTo(r, it.cv.height); c.arc(r, r, r, Math.PI / 2, Math.PI * 1.5); c.closePath(); c.fill();
      }
      c.font = font; c.textAlign = 'center'; c.textBaseline = 'middle';
      var cx = it.cv.width / 2, cy = it.cv.height / 2 + dpr * 0.5;
      if (opts.outline) {
        // 描边：往 16 个方向各挪一点写一遍（本引擎的 strokeText 在真机上一次几百毫秒，fillText 便宜）
        c.fillStyle = opts.outline[0];
        for (var k = 0; k < 16; k++) { var a = k / 16 * Math.PI * 2; c.fillText(s, cx + Math.cos(a) * ow * dpr, cy + Math.sin(a) * ow * dpr); }
      }
      c.fillStyle = color; c.fillText(s, cx, cy);
      it.tex.needsUpdate = true;
    }
    it.mesh.material.opacity = opts.alpha === undefined ? 1 : opts.alpha;
    var lx = align === 'left' ? x - 2 : align === 'right' ? x - it.w + 2 : x - it.w / 2;
    place(it.mesh, lx, y - it.h / 2, it.w, it.h);
    return { x: lx, y: y - it.h / 2, w: it.w, h: it.h };
  }
  function button(key, s, cx, cy, w, h, bg, fg) {
    var size = Math.round(h * 0.42);
    var padX = Math.max(0, (w - size * String(s).length * 1.05) / 2), padY = Math.max(0, (h - size * 1.35 - 4) / 2);
    var b = text(key, s, cx, cy, size, fg || '#333333', 'center', 'bold', { bg: bg || '#ffffff', padX: padX, padY: padY });
    return b;
  }

  // 面板：draw(ctx2d, w, h) 在一块离屏 2D 画布上画一整块界面（逻辑像素坐标，已按 dpr 缩放），sig 不变就不重画、不重传。
  // 一块面板 = 一次 draw call：计分板这种几十个格子 / 字的，别拆成几十个 text（每个都是一个网格、一次 draw call）
  function panel(key, sig, x, y, w, h, draw, alpha) {
    var it = get('p:' + key, function () {
      var cv = wx.createCanvas(); cv.width = Math.ceil(w * dpr); cv.height = Math.ceil(h * dpr);
      var tex = new THREE.CanvasTexture(cv);
      tex.minFilter = THREE.LinearFilter; tex.generateMipmaps = false;
      if (THREE.SRGBColorSpace) tex.colorSpace = THREE.SRGBColorSpace;
      return { cv: cv, tex: tex, mesh: new THREE.Mesh(plane, basic({ map: tex })), sig: null };
    });
    if (it.sig !== sig) {
      it.sig = sig;
      var c = it.cv.getContext('2d');
      c.setTransform(1, 0, 0, 1, 0, 0); c.clearRect(0, 0, it.cv.width, it.cv.height);
      c.setTransform(dpr, 0, 0, dpr, 0, 0);
      draw(c, w, h);
      it.tex.needsUpdate = true;
    }
    it.mesh.material.opacity = alpha === undefined ? 1 : alpha;
    place(it.mesh, x, y, w, h);
  }

  // 整张图（封面）：tex 是现成的 THREE.Texture，裁切（repeat / offset）由调用方设好
  function image(key, tex, x, y, w, h, alpha) {
    var it = get('i:' + key, function () { return { mesh: new THREE.Mesh(plane, basic({ map: tex })) }; });
    it.mesh.material.opacity = alpha === undefined ? 1 : alpha;
    place(it.mesh, x, y, w, h);
  }

  return {
    begin: function () { frame++; order = 0; },
    rect: rect, text: text, button: button, panel: panel, image: image,
    // 叠在 3D 上画：不清颜色，只清深度
    render: function () {
      for (var k in items) if (items[k].used !== frame) items[k].mesh.visible = false;
      var ac = renderer.autoClear;
      renderer.autoClear = false;
      renderer.clearDepth();
      renderer.render(scene, cam);
      renderer.autoClear = ac;
    }
  };
}

module.exports = { make: make };
