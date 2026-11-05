// <__KIND__>（官方 Skyline 手势系统 framework/runtime/skyline/gesture）：**虚组件** —— 模板根节点不进树，
// 触摸绑定并到唯一的子节点上（parser/template/gesture_merge.rs），父节点的样式直接作用在子节点上。
// 绑定的处理函数名是 `__gh:<uid>:<方法>`，逻辑层按 uid 找到实例（page-components.js `__ghDispatch`）。
// 回调使用方 `on-gesture-event` / `worklet:ongesture="方法名"`（页面或外层组件上的方法；微信里是 worklet，
// 这里在逻辑层同步调，参数就是手势事件对象）。
//
// 事件对象（官方字段）：state（0 POSSIBLE / 1 BEGIN / 2 ACTIVE / 3 END / 4 CANCELLED）、absoluteX / absoluteY、
// deltaX / deltaY（相对上一次）、velocityX / velocityY（px/s，按最近 100ms 的位移算；抬手那一下就是离手速度）、
// translationX / translationY（相对按下点）、focalX / focalY / focalDeltaX / focalDeltaY、scale、pointerCount。
//
// 状态（官方「手势状态」一节）：连续手势（pan / 两种 drag / long-press）按下就报 POSSIBLE；认出来报 BEGIN，
// 之后 ACTIVE，抬手 END；没认出来（方向不对、被别的手势抢走、should-accept-gesture 返回 false）报 CANCELLED。
// tap 只报一次、state 恒为 1（官方原话）；double-tap 第二下抬手报一次，state 沿用 3（官方没写，待验）；
// scale 没有 POSSIBLE。
//
// 仲裁（官方「工作原理」「手势协商」）：同一串触摸里的手势组件进一个竞技场（globalThis.__ghArena）。
// 每一步从内到外推进；同一步里同时够条件的，专门的（drag / long-press）先于宽松的（pan / scale）——
// 官方：vertical-drag 包 pan 时纵向滑动归 vertical-drag。一个认出来之后，其余不能同时的（`simultaneous-handlers`
// 里没列对方的 `tag`，任一方列了就算）一律 CANCELLED，之后也认不出来。
// 判定距离：drag 4px（与宿主的方向锁定同一个数，host::gesture::DIRECTION_LOCK_SLOP）、pan / scale 8px。
//
// native-view="scroll-view|swiper"：代理容器自己的拖动 —— 不锁滚动，容器照常滚；宿主在这串触摸里不给
// 页面补 touchcancel（手势回调一直跟到松手，官方：触顶后 scroll 不再触发、手势回调照常）。
// should-accept-gesture 返回 false 时整串不滚，should-response-on-move 返回 false 时这一下不滚
// （globalThis.__ghNativeBlock，宿主每一下 move 读一次，只在这种触摸里读：host::gesture_native）。
//
// 取舍（写明）：引擎的触摸是单指 —— scale-gesture-handler 按 pan 报（scale 恒为 1、pointerCount 1）；
// force-press（iPhone 重按）不会触发；横向 drag 不锁页面的纵向滚动（只有 pan / vertical-drag 锁）。
if (!globalThis.__ghArena) {
  globalThis.__ghArena = {
    seq: '', list: [], winners: [], key: '',
    // 一串新的触摸从 touchstart 的时间戳认（同一个 touchstart 派给几层手势时时间戳相同）
    join: function (g, e) {
      var k = String(e.timeStamp);
      if (this.seq !== k) {
        this.seq = k; this.list = []; this.winners = []; this.key = '';
        globalThis.__ghNativeBlock = false; globalThis.__ghNativeReject = false;
      }
      if (this.list.indexOf(g) < 0) this.list.push(g);
    },
    simul: function (a, b) {
      var ta = a.properties.tag, tb = b.properties.tag;
      var la = a.properties.simultaneousHandlers || [], lb = b.properties.simultaneousHandlers || [];
      return !!((tb && la.indexOf(tb) >= 0) || (ta && lb.indexOf(ta) >= 0));
    },
    // 一步（move / end / cancel）只推进一次：同一个事件派给好几层时后面的直接返回
    step: function (phase, e) {
      var ts = phase === 'tm' ? e.touches : e.changedTouches;
      var t = (ts && ts[0]) || null;
      var k = phase + e.timeStamp + ':' + (t ? t.clientX + ',' + t.clientY : '');
      if (this.key === k) return;
      this.key = k;
      var list = this.list.slice(), cands = [];
      // 从内到外（进竞技场的顺序是外 → 内：捕获阶段先到外层）
      for (var i = list.length - 1; i >= 0; i--) {
        if (list[i]['_' + phase + 'Impl'](t, e)) cands.push(list[i]);
      }
      cands.sort(function (a, b) { return a._rank() - b._rank(); });
      for (var j = 0; j < cands.length; j++) this.accept(cands[j], t);
      if (phase !== 'tm') {
        for (var m = 0; m < list.length; m++) list[m]._reset();
        this.list = []; this.winners = []; this.seq = '';
      }
    },
    accept: function (g, t) {
      var self = this;
      if (!g._g || g._g.failed) return false;
      var lost = this.winners.some(function (w) { return w !== g && !self.simul(w, g); });
      if (lost || g._shouldAccept() === false) { g._fail(); return false; }
      this.winners.push(g);
      g._begin(t);
      this.list.forEach(function (o) {
        if (o !== g && o._g && !o._g.active && !self.simul(o, g)) o._fail();
      });
      return true;
    }
  };
}
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
  data: { uid: '' },
  lifetimes: {
    created: function () {
      globalThis.__ghUid = (globalThis.__ghUid || 0) + 1;
      this.data.uid = String(globalThis.__ghUid);
      __ghInstances[this.data.uid] = this;
    },
    detached: function () { delete __ghInstances[this.data.uid]; }
  },
  methods: {
    _owner: function () { return this.selectOwnerComponent() || (typeof __currentPage === 'object' ? __currentPage : null); },
    _call: function (name, ev) {
      var o = this._owner();
      var fn = name && o ? o[name] : null;
      if (typeof fn !== 'function') return undefined;
      try { return fn.call(o, ev); } catch (e) { console.error(e); }
    },
    // 同一步里够条件的先后：专门的在前
    _rank: function () { var k = '__KIND__'; return k === 'pan-gesture-handler' || k === 'scale-gesture-handler' ? 1 : 0; },
    // 最近 100ms 的速度（px/s）
    _velocity: function (now) {
      var s = this._g.samples;
      while (s.length > 1 && s[0].t < now - 100) s.shift();
      if (s.length < 2 || now - s[0].t > 100) return [0, 0];
      var a = s[0], b = s[s.length - 1], dt = b.t - a.t;
      return dt > 0 ? [(b.x - a.x) * 1000 / dt, (b.y - a.y) * 1000 / dt] : [0, 0];
    },
    _emit: function (state, now) {
      var g = this._g;
      var v = this._velocity(now === undefined ? g.lastT : now);
      var live = state !== 3 && state !== 4;
      this._call(this.properties.onGestureEvent, {
        state: state, absoluteX: g.x, absoluteY: g.y, deltaX: g.dx, deltaY: g.dy,
        translationX: g.x - g.x0, translationY: g.y - g.y0, velocityX: v[0], velocityY: v[1],
        focalX: g.x, focalY: g.y, focalDeltaX: g.dx, focalDeltaY: g.dy,
        scale: 1, horizontalScale: 1, verticalScale: 1, rotation: 0, pressure: 0, pointerCount: live ? 1 : 0
      });
    },
    _shouldAccept: function () {
      var p = this.properties;
      if (!p.shouldAcceptGesture) return true;
      var ok = this._call(p.shouldAcceptGesture, { state: 0, absoluteX: this._g.x, absoluteY: this._g.y });
      if (ok === false && p.nativeView) globalThis.__ghNativeReject = true;
      return ok;
    },
    _begin: function () {
      var g = this._g, k = '__KIND__';
      g.active = true;
      if (k === 'tap-gesture-handler') this._emit(1);
      else if (k === 'double-tap-gesture-handler') this._emit(3);
      else { this._emit(1); }
    },
    _fail: function () {
      var g = this._g;
      if (!g || g.failed) return;
      clearTimeout(this._lp);
      var announced = g.possible || g.active;
      g.failed = true; g.active = false;
      if (announced) this._emit(4);
    },
    _reset: function () { clearTimeout(this._lp); this._g = null; },
    _ts: function (e) {
      var t = e.touches && e.touches[0];
      if (!t) return;
      globalThis.__ghArena.join(this, e);
      var now = e.timeStamp, k = '__KIND__', self = this;
      this._g = { x0: t.clientX, y0: t.clientY, x: t.clientX, y: t.clientY, dx: 0, dy: 0, t0: now, lastT: now,
                  active: false, failed: false, moved: false, possible: false, samples: [{ t: now, x: t.clientX, y: t.clientY }] };
      if (k === 'pan-gesture-handler' || k === 'horizontal-drag-gesture-handler' || k === 'vertical-drag-gesture-handler' ||
          k === 'long-press-gesture-handler') {
        this._g.possible = true;
        this._emit(0);
      }
      if (k === 'long-press-gesture-handler') {
        var g0 = this._g;
        this._lp = setTimeout(function () {
          if (self._g === g0 && !g0.moved && !g0.failed) globalThis.__ghArena.accept(self, null);
        }, 500);
      }
    },
    _tm: function (e) { globalThis.__ghArena.step('tm', e); },
    _te: function (e) { globalThis.__ghArena.step('te', e); },
    _tc: function (e) { globalThis.__ghArena.step('tc', e); },
    _nv: function () {},
    // 竞技场推进的一步 move：返回「这一步够条件认出来」
    _tmImpl: function (t, e) {
      var g = this._g, p = this.properties, k = '__KIND__';
      if (!g || g.failed || !t) return false;
      if (p.shouldResponseOnMove) {
        var ok = this._call(p.shouldResponseOnMove, {
          identifier: t.identifier || 0, type: 'touchmove', deltaX: t.clientX - g.x, deltaY: t.clientY - g.y,
          clientX: t.clientX, clientY: t.clientY, radiusX: 0, radiusY: 0, rotationAngle: 0, force: t.force || 0,
          timeStamp: e.timeStamp
        }) !== false;
        if (p.nativeView) globalThis.__ghNativeBlock = !ok;
        if (!ok) return false;
      }
      g.dx = t.clientX - g.x; g.dy = t.clientY - g.y;
      g.x = t.clientX; g.y = t.clientY; g.lastT = e.timeStamp;
      g.samples.push({ t: e.timeStamp, x: g.x, y: g.y });
      var tx = g.x - g.x0, ty = g.y - g.y0, ax = Math.abs(tx), ay = Math.abs(ty);
      if (ax > 4 || ay > 4) g.moved = true;
      if (k === 'tap-gesture-handler' || k === 'double-tap-gesture-handler' || k === 'force-press-gesture-handler') {
        if (g.moved) this._fail();
        return false;
      }
      if (k === 'long-press-gesture-handler') {
        // 认出长按之后的移动是 ACTIVE；之前移动了就不算长按
        if (g.active) this._emit(2); else if (g.moved) this._fail();
        return false;
      }
      if (g.active) { this._emit(2); return false; }
      if (k === 'horizontal-drag-gesture-handler') {
        if (ax >= 4 && ax > ay) return true;
        if (ay >= 4) this._fail();
        return false;
      }
      if (k === 'vertical-drag-gesture-handler') {
        if (ay >= 4 && ay >= ax) return true;
        if (ax >= 4) this._fail();
        return false;
      }
      return Math.sqrt(tx * tx + ty * ty) >= 8;
    },
    _teImpl: function (t, e) {
      var g = this._g, k = '__KIND__';
      clearTimeout(this._lp);
      if (!g || g.failed) return false;
      if (t) { g.dx = t.clientX - g.x; g.dy = t.clientY - g.y; g.x = t.clientX; g.y = t.clientY; }
      if (k === 'tap-gesture-handler') return !g.moved;
      if (k === 'double-tap-gesture-handler') {
        if (g.moved) return false;
        if (this._lastTap && e.timeStamp - this._lastTap < 300) { this._lastTap = 0; return true; }
        this._lastTap = e.timeStamp;
        return false;
      }
      if (g.active) { this._emit(3, e.timeStamp); g.active = false; }
      else if (g.possible) this._fail();
      return false;
    },
    _tcImpl: function () {
      this._fail();
      return false;
    }
  }
});
