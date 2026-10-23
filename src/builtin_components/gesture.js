// <__KIND__>（官方 Skyline 手势系统 component/…-gesture-handler）：包住子节点，按触摸算手势状态，
// 回调使用方 `on-gesture-event="方法名"`（页面或外层组件上的方法；微信里是 worklet，这里在逻辑层同步调）。
//
// 事件对象（官方字段）：state（0 POSSIBLE / 1 BEGIN / 2 ACTIVE / 3 END / 4 CANCELLED）、absoluteX / absoluteY、
// deltaX / deltaY（相对上一次）、velocityX / velocityY（px/s）、translationX / translationY（相对起点）、
// focalX / focalY、scale（scale 手势）、pointerCount。
// `should-response-on-move="方法名"` 返回 false 时这一下 move 不回调；`should-accept-gesture="方法名"` 返回 false 时整串不认。
//
// 取舍（写明）：引擎的触摸是单指 —— scale-gesture-handler 只在单指拖动时按 pan 报 focal 点、scale 恒为 1；
// force-press（iPhone 重按）不会触发；simultaneous-handlers / native-view 不参与仲裁（手势照常并行）。
Component({
  options: { virtualHost: true },
  properties: {
    onGestureEvent: { type: String, value: '' },
    shouldResponseOnMove: { type: String, value: '' },
    shouldAcceptGesture: { type: String, value: '' },
    tag: { type: String, value: '' },
    simultaneousHandlers: { type: Array, value: [] },
    nativeView: { type: String, value: '' }
  },
  methods: {
    _owner: function () { return this.selectOwnerComponent() || (typeof __currentPage === 'object' ? __currentPage : null); },
    _call: function (name, ev) {
      var o = this._owner();
      var fn = name && o ? o[name] : null;
      if (typeof fn !== 'function') return undefined;
      try { return fn.call(o, ev); } catch (e) { console.error(e); }
    },
    _emit: function (state, t) {
      var g = this._g || {};
      var now = Date.now(), dt = Math.max(1, now - (g.lastT || now));
      var x = t ? t.clientX : g.x, y = t ? t.clientY : g.y;
      var dx = x - (g.x === undefined ? x : g.x), dy = y - (g.y === undefined ? y : g.y);
      var ev = {
        state: state, absoluteX: x, absoluteY: y, deltaX: dx, deltaY: dy,
        translationX: x - (g.x0 || x), translationY: y - (g.y0 || y),
        velocityX: dx * 1000 / dt, velocityY: dy * 1000 / dt,
        focalX: x, focalY: y, scale: 1, pointerCount: state === 3 || state === 4 ? 0 : 1
      };
      g.x = x; g.y = y; g.lastT = now;
      this._call(this.properties.onGestureEvent, ev);
    },
    _ts: function (e) {
      var t = e.touches && e.touches[0];
      if (!t) return;
      if (this.properties.shouldAcceptGesture && this._call(this.properties.shouldAcceptGesture, { state: 0 }) === false) { this._g = null; return; }
      var now = Date.now();
      this._g = { x0: t.clientX, y0: t.clientY, x: t.clientX, y: t.clientY, t0: now, lastT: now, active: false, moved: false };
      var kind = '__KIND__', self = this;
      if (kind === 'long-press-gesture-handler') {
        this._lp = setTimeout(function () { if (self._g && !self._g.moved) { self._g.active = true; self._emit(1, null); } }, 500);
      }
    },
    _tm: function (e) {
      var g = this._g, t = e.touches && e.touches[0];
      if (!g || !t) return;
      var tx = t.clientX - g.x0, ty = t.clientY - g.y0;
      if (Math.abs(tx) > 4 || Math.abs(ty) > 4) g.moved = true;
      var kind = '__KIND__';
      if (kind === 'tap-gesture-handler' || kind === 'double-tap-gesture-handler' || kind === 'force-press-gesture-handler') return;
      if (kind === 'long-press-gesture-handler') {
        // 认出长按之后的移动是 ACTIVE；之前移动了就不算长按
        if (g.active) this._emit(2, t); else if (g.moved) clearTimeout(this._lp);
        return;
      }
      if (!g.active) {
        if (!g.moved) return;
        var horiz = Math.abs(tx) > Math.abs(ty);
        if (kind === 'horizontal-drag-gesture-handler' && !horiz) { this._g = null; return; }
        if (kind === 'vertical-drag-gesture-handler' && horiz) { this._g = null; return; }
        g.active = true;
        this._emit(1, t);
        return;
      }
      if (this.properties.shouldResponseOnMove && this._call(this.properties.shouldResponseOnMove, { absoluteX: t.clientX, absoluteY: t.clientY }) === false) return;
      this._emit(2, t);
    },
    _te: function (e) {
      clearTimeout(this._lp);
      var g = this._g, t = (e.changedTouches && e.changedTouches[0]) || null;
      if (!g) return;
      var kind = '__KIND__';
      if (kind === 'tap-gesture-handler') {
        if (!g.moved && Date.now() - g.t0 < 500) this._emit(3, t);
      } else if (kind === 'double-tap-gesture-handler') {
        var now = Date.now();
        if (!g.moved) {
          if (this._lastTap && now - this._lastTap < 300) { this._lastTap = 0; this._emit(3, t); }
          else this._lastTap = now;
        }
      } else if (g.active) {
        this._emit(3, t);
      }
      this._g = null;
    },
    _tc: function () {
      clearTimeout(this._lp);
      if (this._g && this._g.active) this._emit(4, null);
      this._g = null;
    }
  }
});
