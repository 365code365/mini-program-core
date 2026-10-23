// <draggable-sheet>（官方 component/draggable-sheet，Skyline）：半屏可拖拽面板，从容器底部往上占 size（0~1）。
// 拖动改 size（夹在 min-child-size ~ max-child-size），松手时 snap 开着就吸到最近的吸附点（min / max / snap-sizes）。
// DraggableSheetContext.scrollTo({size | pixels, animated, duration})：经 this.selectComponent(…).scrollTo 或
// SelectorQuery 的 node() 调。size 是**容器高**的比例：容器高在布局出来后与每次按下时量一次（量出来之前按窗口高）。
// 取舍：worklet:onsizeupdate 不触发（没有 worklet 线程）；改成发普通事件 bind:sizeupdate {size, pixels}。
Component({
  properties: {
    initialChildSize: { type: Number, value: 0.5 },
    minChildSize: { type: Number, value: 0.25 },
    maxChildSize: { type: Number, value: 1 },
    snap: { type: Boolean, value: false },
    snapSizes: { type: Array, value: [] }
  },
  data: { size: 0.5, anim: null, uid: '' },
  lifetimes: {
    created: function () {
      globalThis.__dsSeq = (globalThis.__dsSeq || 0) + 1;
      this.data.uid = '__ds' + globalThis.__dsSeq;
    },
    attached: function () {
      this._set(this.properties.initialChildSize);
      var self = this;
      setTimeout(function () { self._measure(); }, 0);
    }
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
    _set: function (s) {
      var size = this._clamp(s);
      if (size === this.data.size) return;
      this.setData({ size: size });
      this.triggerEvent('sizeupdate', { size: size, pixels: size * this._h() });
    },
    _ts: function (e) {
      var t = e.touches && e.touches[0];
      this._drag = t ? { y: t.clientY, size: this.data.size } : null;
      this._measure();
    },
    _tm: function (e) {
      var t = e.touches && e.touches[0];
      if (!this._drag || !t) return;
      this._set(this._drag.size + (this._drag.y - t.clientY) / this._h());
    },
    _te: function () {
      if (!this._drag) return;
      this._drag = null;
      if (!this.properties.snap) return;
      var p = this.properties, s = this.data.size;
      var points = [p.minChildSize, p.maxChildSize].concat(p.snapSizes || []);
      var best = points.reduce(function (a, b) { return Math.abs(b - s) < Math.abs(a - s) ? b : a; }, points[0]);
      this._set(best);
    },
    // DraggableSheetContext.scrollTo
    scrollTo: function (o) {
      o = o || {};
      var size = typeof o.size === 'number' ? o.size : (typeof o.pixels === 'number' ? o.pixels / this._h() : this.data.size);
      this._set(size);
    }
  }
});
