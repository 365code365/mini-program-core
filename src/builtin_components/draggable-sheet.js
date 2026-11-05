// <draggable-sheet>（官方 component/draggable-sheet，Skyline）：半屏可拖拽面板，从容器底部往上占 size（0~1）。
// 拖动改 size（夹在 min-child-size ~ max-child-size），松手时 snap 开着就吸到最近的吸附点（min / max / snap-sizes）。
// DraggableSheetContext.scrollTo({size | pixels, animated = true, duration = 300, easingFunction = 'ease'})
// （官方 api/skyline/DraggableSheetContext.scrollTo：size 与 pixels 都给时只认 size）：经 this.selectComponent(…).scrollTo
// 或 SelectorQuery 的 node() 调（不带 id 也行，宿主按组件实例键找，query.js `__builtinInstance`）。
// size 是**容器高**的比例：容器高在布局出来后与每次按下时量一次（量出来之前按窗口高）。
//
// 尺寸变化回调 `worklet:onsizeupdate="方法名"`（官方 event = {pixels, size}）：引擎没有 worklet 线程，在逻辑层同步调；
// 另外照旧发普通事件 bind:sizeupdate（同一个 detail）。
//
// 动画（吸附、scrollTo animated）走 `animation` 属性（renderer js_animation：渲染层按帧插值、只重画面板那块，
// 不是每帧 setData 重排）：高度先取起止里大的那个，translateY 从 (大-起)·H 走到 (大-止)·H，走完再把高度落到止点。
// 吸附动画的时长与缓动官方没写，用 scrollTo 的默认值（300ms、ease，待验）。
//
// 面板里的 scroll-view（宿主仲裁，host::gesture `SheetHit`）：面板没到最大时上推 → 拖面板；列表在顶时下拉 → 拖面板；
// 其余列表滚。取舍：一次拖动里不换手（面板拖到最大之后继续上推，列表要等下一次拖动才滚）。
Component({
  properties: {
    initialChildSize: { type: Number, value: 0.5 },
    minChildSize: { type: Number, value: 0.25 },
    maxChildSize: { type: Number, value: 1 },
    snap: { type: Boolean, value: false },
    snapSizes: { type: Array, value: [] },
    onsizeupdate: { type: String, value: '' }
  },
  data: { size: 0.5, anim: null, uid: '', atMax: false },
  lifetimes: {
    created: function () {
      globalThis.__dsSeq = (globalThis.__dsSeq || 0) + 1;
      this.data.uid = '__ds' + globalThis.__dsSeq;
    },
    attached: function () {
      this._set(this.properties.initialChildSize);
      var self = this;
      setTimeout(function () { self._measure(); }, 0);
    },
    detached: function () { clearTimeout(this._at); }
  },
  methods: {
    _clamp: function (s) {
      var p = this.properties;
      return Math.max(p.minChildSize, Math.min(p.maxChildSize, s));
    },
    // 容器（.ds-wrap 撑满组件）的高
    _measure: function () {
      var self = this;
      wx.createSelectorQuery().select('.' + this.data.uid + '-w').boundingClientRect(function (r) {
        if (r && r.height > 0) self._ch = r.height;
      }).exec();
    },
    _h: function () { return this._ch || wx.getWindowInfo().windowHeight || 1; },
    _report: function (size) {
      var d = { size: size, pixels: size * this._h() };
      var name = this.properties.onsizeupdate;
      var o = name ? (this.selectOwnerComponent() || (typeof __currentPage === 'object' ? __currentPage : null)) : null;
      if (o && typeof o[name] === 'function') { try { o[name](d); } catch (e) { console.error(e); } }
      this.triggerEvent('sizeupdate', d);
    },
    _atMax: function (s) { return s >= this.properties.maxChildSize - 1e-4; },
    _set: function (s) {
      var size = this._clamp(s);
      this._stopAnim();
      if (size === this.data.size) return;
      this.setData({ size: size, atMax: this._atMax(size) });
      this._report(size);
    },
    _stopAnim: function () {
      if (!this._anim) return;
      clearTimeout(this._at);
      var to = this._anim.to;
      this._anim = null;
      this.setData({ anim: null, size: to, atMax: this._atMax(to) });
    },
    // 从当前 size 动画到 to（animation 通道，见文件头）
    _animateTo: function (to, duration, easing) {
      to = this._clamp(to);
      this._stopAnim();
      var from = this.data.size;
      if (to === from) return;
      if (!(duration > 0)) { this._set(to); return; }
      var H = this._h(), hi = Math.max(from, to), self = this;
      var step = function (y, ms, ease) {
        return { animates: [{ type: 'translateY', args: [y] }], option: { transition: { duration: ms, delay: 0, timingFunction: ease } } };
      };
      this._anim = { to: to };
      this.setData({ size: hi, atMax: this._atMax(to), anim: { actions: [step((hi - from) * H, 0, 'linear'), step((hi - to) * H, duration, easing || 'ease')] } });
      this._at = setTimeout(function () {
        if (!self._anim) return;
        self._anim = null;
        self.setData({ anim: null, size: to });
        self._report(to);
      }, duration);
    },
    _ts: function (e) {
      var t = e.touches && e.touches[0];
      this._stopAnim();
      this._drag = t ? { y: t.clientY, size: this.data.size, moves: 0 } : null;
      this._measure();
    },
    _tm: function (e) {
      var t = e.touches && e.touches[0];
      if (!this._drag || !t) return;
      this._drag.moves++;
      this._set(this._drag.size + (this._drag.y - t.clientY) / this._h());
    },
    _te: function () {
      if (!this._drag) return;
      this._drag = null;
      if (!this.properties.snap) return;
      var p = this.properties, s = this.data.size;
      var points = [p.minChildSize, p.maxChildSize].concat(p.snapSizes || []);
      var best = points.reduce(function (a, b) { return Math.abs(b - s) < Math.abs(a - s) ? b : a; }, points[0]);
      this._animateTo(best, 300, 'ease');
    },
    // 列表接管了这串触摸（宿主补的 touchcancel）：定方向那一下面板已经跟着动了，退回去
    _tc: function () {
      var d = this._drag;
      if (!d) return;
      if (d.moves <= 1) { this._drag = null; this._set(d.size); return; }
      this._te();
    },
    // DraggableSheetContext.scrollTo
    scrollTo: function (o) {
      o = o || {};
      var size = typeof o.size === 'number' ? o.size : (typeof o.pixels === 'number' ? o.pixels / this._h() : this.data.size);
      if (o.animated === false) { this._set(size); return; }
      this._animateTo(size, o.duration === undefined ? 300 : Number(o.duration), o.easingFunction || 'ease');
    }
  }
});
