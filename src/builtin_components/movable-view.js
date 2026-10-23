// <movable-view>（官方 component/movable-view）：在 <movable-area> 里可拖动的视图。
//
// - 只有一层节点：`virtualHost` + 引擎专用的 `__mergeHost`，模板根节点并入宿主（标签、页面写的 class / style / id，
//   parser/template/component.rs）—— 页面给 movable-view 写的宽高、背景、子项对齐直接作用在装内容的这一层。
//   默认绝对定位在 movable-area 左上角、10px 见方（标签默认样式，renderer components::native_placeholder 的
//   ua_declarations；页面 CSS 照样覆盖）。位置是根节点 style 里的 left / top（data.pos）。
// - direction：all / vertical / horizontal / none（默认 none，不能拖）；disabled 时也不能拖。
// - 可移动范围：视图比区域小时 [0, 区域 - 视图]，比区域大时 [区域 - 视图, 0]（官方语义）。
//   out-of-bounds 为真时可以拖出范围（阻力减半），松手回弹；inertia 为真时松手按速度滑行，friction 越大停得越快。
// - x / y（px，也认 "20rpx"）：使用方 setData 改了就移过去（animation 为真时 damping 决定时长）。
// - 事件：change {x, y, source}，source 是 touch / touch-out-of-bounds / out-of-bounds / friction / ''（x、y 被 setData）；
//   第一次移动是横向 / 纵向时各发一次 htouchmove / vtouchmove。
//
// 取舍（写明）：引擎触摸是单指，双指缩放（scale）不支持 —— scale-value 照样按 transform: scale 显示，bindscale 不触发。
// 横向拖动（direction=horizontal）用 bindtouchmove，竖着滑照样滚页面（手势仲裁按轴锁定）；其余方向 catchtouchmove。
Component({
  options: { virtualHost: true, __mergeHost: true },
  properties: {
    direction: { type: String, value: 'none' },
    inertia: { type: Boolean, value: false },
    outOfBounds: { type: Boolean, value: false },
    x: { type: null, value: 0 },
    y: { type: null, value: 0 },
    damping: { type: Number, value: 20 },
    friction: { type: Number, value: 2 },
    disabled: { type: Boolean, value: false },
    scale: { type: Boolean, value: false },
    scaleMin: { type: Number, value: 0.1 },
    scaleMax: { type: Number, value: 10 },
    scaleValue: { type: Number, value: 1 },
    animation: { type: Boolean, value: true }
  },
  data: { uid: '', mode: '', pos: 'left:0px;top:0px;' },
  observers: {
    'direction, disabled': function (d, dis) {
      var mode = dis || d === 'none' ? '' : (d === 'horizontal' ? 'bind' : 'catch');
      if (mode !== this.data.mode) this.setData({ mode: mode });
    },
    'x, y': function (x, y) {
      if (!this._ready) return;
      this._goto(this._px(x), this._px(y), this.properties.animation, '');
    },
    'scaleValue': function () { if (this._ready) this._place(this._x, this._y); }
  },
  lifetimes: {
    created: function () {
      globalThis.__mvSeq = (globalThis.__mvSeq || 0) + 1;
      this.data.uid = '__mv' + globalThis.__mvSeq;
      this._x = 0; this._y = 0;
    },
    attached: function () {
      this._ready = true;
      this._x = this._px(this.properties.x);
      this._y = this._px(this.properties.y);
      this._place(this._x, this._y);
      // 布局出来以后量一次范围，把初始 x / y 夹进去
      var self = this;
      setTimeout(function () { self._measure(function () { self._goto(self._x, self._y, false, null); }); }, 0);
    },
    detached: function () { this._stop(); this._ready = false; }
  },
  methods: {
    _px: function (v) {
      if (typeof v === 'number') return v;
      var s = String(v == null ? '' : v).trim(), n = parseFloat(s);
      if (isNaN(n)) return 0;
      if (/rpx$/.test(s)) return n * (wx.getWindowInfo().windowWidth || 375) / 750;
      return n;
    },
    _place: function (x, y) {
      this._x = x; this._y = y;
      var s = this.properties.scaleValue;
      var st = 'left:' + x + 'px;top:' + y + 'px;';
      if (s && s !== 1) st += 'transform:scale(' + Math.max(this.properties.scaleMin, Math.min(this.properties.scaleMax, s)) + ');';
      if (st !== this.data.pos) this.setData({ pos: st });
    },
    // 量自己与所在的 movable-area：自己的左上角减去当前偏移就是区域的原点，取原点对得上的那块区域
    _measure: function (done) {
      var self = this;
      wx.createSelectorQuery().select('.' + this.data.uid).boundingClientRect().selectAll('movable-area').boundingClientRect().exec(function (res) {
        var me = res && res[0], areas = (res && res[1]) || [];
        if (me) {
          var ox = me.left - self._x, oy = me.top - self._y, best = null, bd = Infinity;
          for (var i = 0; i < areas.length; i++) {
            var a = areas[i], d = Math.abs(a.left - ox) + Math.abs(a.top - oy);
            if (d < bd) { bd = d; best = a; }
          }
          var aw = best ? best.width : me.width, ah = best ? best.height : me.height;
          self._b = {
            minX: Math.min(0, aw - me.width), maxX: Math.max(0, aw - me.width),
            minY: Math.min(0, ah - me.height), maxY: Math.max(0, ah - me.height)
          };
        }
        if (done) done();
      });
    },
    _clampX: function (x) { var b = this._b; return b ? Math.max(b.minX, Math.min(b.maxX, x)) : x; },
    _clampY: function (y) { var b = this._b; return b ? Math.max(b.minY, Math.min(b.maxY, y)) : y; },
    _change: function (source) {
      this.triggerEvent('change', { x: this._x, y: this._y, source: source });
    },
    _stop: function () { if (this._timer) { clearTimeout(this._timer); this._timer = null; } },
    // 移到 (x, y)（夹进范围）；anim 为真时按 damping 定时长缓动。source 为 null 时不发 change
    _goto: function (x, y, anim, source) {
      this._stop();
      var tx = this._clampX(x), ty = this._clampY(y);
      if (!anim) {
        var moved = tx !== this._x || ty !== this._y;
        this._place(tx, ty);
        if (moved && source !== null) this._change(source);
        return;
      }
      var self = this, fx = this._x, fy = this._y, t0 = Date.now();
      var dur = Math.max(150, Math.min(600, 6000 / Math.max(1, this.properties.damping)));
      var step = function () {
        var k = Math.min(1, (Date.now() - t0) / dur), e = 1 - Math.pow(1 - k, 3);
        self._place(fx + (tx - fx) * e, fy + (ty - fy) * e);
        if (source !== null) self._change(source);
        self._timer = k < 1 ? setTimeout(step, 16) : null;
      };
      step();
    },
    _ts: function (e) {
      var t = e.touches && e.touches[0];
      if (!t) return;
      this._stop();
      this._drag = { x0: t.clientX, y0: t.clientY, sx: this._x, sy: this._y, lx: t.clientX, ly: t.clientY, lt: Date.now(), vx: 0, vy: 0, first: true };
      this._measure(null);
    },
    _tm: function (e) {
      var g = this._drag, t = e.touches && e.touches[0];
      if (!g || !t) return;
      var d = this.properties.direction;
      var dx = d === 'vertical' ? 0 : t.clientX - g.x0, dy = d === 'horizontal' ? 0 : t.clientY - g.y0;
      if (g.first && (Math.abs(t.clientX - g.x0) > 2 || Math.abs(t.clientY - g.y0) > 2)) {
        g.first = false;
        var horiz = Math.abs(t.clientX - g.x0) >= Math.abs(t.clientY - g.y0);
        this.triggerEvent(horiz ? 'htouchmove' : 'vtouchmove', { x: t.clientX, y: t.clientY });
      }
      var now = Date.now(), dt = Math.max(1, now - g.lt);
      g.vx = (t.clientX - g.lx) / dt; g.vy = (t.clientY - g.ly) / dt; g.lx = t.clientX; g.ly = t.clientY; g.lt = now;
      var x = g.sx + dx, y = g.sy + dy, cx = this._clampX(x), cy = this._clampY(y), out = false;
      if (this.properties.outOfBounds) {
        // 出界的部分阻力减半
        if (x !== cx) { x = cx + (x - cx) / 2; out = true; }
        if (y !== cy) { y = cy + (y - cy) / 2; out = true; }
      } else { x = cx; y = cy; }
      if (x === this._x && y === this._y) return;
      this._place(x, y);
      this._change(out ? 'touch-out-of-bounds' : 'touch');
    },
    _te: function () {
      var g = this._drag;
      if (!g) return;
      this._drag = null;
      var cx = this._clampX(this._x), cy = this._clampY(this._y);
      if (cx !== this._x || cy !== this._y) { this._goto(cx, cy, true, 'out-of-bounds'); return; }
      if (!this.properties.inertia || Date.now() - g.lt > 100) return;
      var d = this.properties.direction;
      var vx = d === 'vertical' ? 0 : g.vx, vy = d === 'horizontal' ? 0 : g.vy;
      // 惯性滑行：每 16ms 按 friction 衰减速度，碰到边界停下
      var self = this, decay = Math.pow(0.95, Math.max(0.1, this.properties.friction) / 2);
      var step = function () {
        vx *= decay; vy *= decay;
        var nx = self._clampX(self._x + vx * 16), ny = self._clampY(self._y + vy * 16);
        if (nx === self._x) vx = 0;
        if (ny === self._y) vy = 0;
        self._place(nx, ny);
        self._change('friction');
        self._timer = Math.abs(vx) + Math.abs(vy) > 0.02 ? setTimeout(step, 16) : null;
      };
      this._stop();
      this._timer = setTimeout(step, 16);
    }
  }
});
