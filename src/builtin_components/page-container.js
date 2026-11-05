// <page-container>（官方 component/page-container）：页面里的「假页」弹层。
// show 变 true → beforeenter / enter / afterenter；变 false、点了返回（返回键、导航栏返回、wx.navigateBack、
// 左边缘右滑 —— host engine_edge_back.rs `edge_container_begin`）、close-on-slide-down 下滑 → beforeleave / leave / afterleave。
// 返回操作只关它、不离开页面（__backInterceptors）。
// 入场 / 离场用 wx.createAnimation（渲染层逐帧插值 transform / opacity）：bottom 从下往上、top 从上往下、
// right 从右往左、center 淡入 + 轻微放大。遮罩点了只发 clickoverlay（与微信一致：关不关由页面决定）。
// close-on-slide-down：面板跟着手指往下走（只往下），松手划过 1/5 窗口高就从当前位置离场，否则弹回原位（200ms）。
// 官方 tip「当前页面最多只有 1 个容器」：已有一个弹着时，别的 show 不出来（告警），等那个关掉。
if (!globalThis.__pageContainerOpen) {
  // 宿主问：当前页有没有弹着的容器（左边缘右滑关它，engine_edge_back.rs）
  globalThis.__pageContainerOpen = function () {
    var a = __currentPage && __currentPage.__pcActive;
    return !!(a && a.data.rendered && !a._leaving);
  };
}
Component({
  properties: {
    show: { type: Boolean, value: false },
    duration: { type: Number, value: 300 },
    zIndex: { type: Number, value: 100 },
    overlay: { type: Boolean, value: true },
    position: { type: String, value: 'bottom' },
    round: { type: Boolean, value: false },
    closeOnSlideDown: { type: Boolean, value: false },
    overlayStyle: { type: String, value: '' },
    customStyle: { type: String, value: '' }
  },
  data: { rendered: false, maskAnim: null, panelAnim: null },
  observers: {
    show: function (v) { if (v) this._enter(); else this._leave(); }
  },
  lifetimes: {
    attached: function () {
      var self = this;
      this._page = typeof __currentPage === 'object' ? __currentPage : null;
      this._back = function () {
        if (!self.data.rendered || self._leaving) return false;
        self._leave();
        return true;
      };
      __backInterceptors.push(this._back);
      if (this.properties.show && !this.data.rendered) this._enter();
    },
    detached: function () {
      var self = this;
      __backInterceptors = __backInterceptors.filter(function (f) { return f !== self._back; });
      clearTimeout(this._timer);
      if (this._page && this._page.__pcActive === this) this._page.__pcActive = null;
    }
  },
  methods: {
    _noop: function () {},
    _offset: function () {
      var w = wx.getWindowInfo();
      var p = this.properties.position;
      return p === 'top' ? { y: -w.windowHeight } : p === 'right' ? { x: w.windowWidth } : p === 'center' ? { s: 0.9 } : { y: w.windowHeight };
    },
    // fromDy：下滑关闭时从手指所在的位置离场
    _anim: function (toVisible, dur, fromDy) {
      var o = this._offset();
      var panel = wx.createAnimation({ duration: dur, timingFunction: 'ease' });
      var mask = wx.createAnimation({ duration: dur, timingFunction: 'ease' });
      if (toVisible) {
        // 先瞬移到起点（0ms 一步），再动到原位
        if (o.s) panel.scale(o.s).opacity(0).step({ duration: 0 }); else panel.translate(o.x || 0, o.y || 0).step({ duration: 0 });
        mask.opacity(0).step({ duration: 0 });
        if (o.s) panel.scale(1).opacity(1).step(); else panel.translate(0, 0).step();
        mask.opacity(1).step();
      } else {
        if (fromDy) panel.translate(0, fromDy).step({ duration: 0 });
        if (o.s) panel.scale(o.s).opacity(0).step(); else panel.translate(o.x || 0, o.y || (fromDy ? wx.getWindowInfo().windowHeight : 0)).step();
        mask.opacity(0).step();
      }
      return { panelAnim: panel.export(), maskAnim: mask.export() };
    },
    _enter: function () {
      if (this.data.rendered && !this._leaving) return;
      var page = this._page, cur = page && page.__pcActive;
      if (cur && cur !== this && cur.data.rendered) {
        console.warn('page-container：当前页面已有一个容器在显示，新的不显示（官方：一页最多 1 个）');
        return;
      }
      if (page) page.__pcActive = this;
      clearTimeout(this._timer);
      this._leaving = false;
      var dur = this.properties.duration;
      this.triggerEvent('beforeenter', {});
      var a = this._anim(true, dur);
      a.rendered = true;
      this.setData(a);
      this.triggerEvent('enter', {});
      var self = this;
      this._timer = setTimeout(function () { self.triggerEvent('afterenter', {}); }, dur);
    },
    _leave: function (fromDy) {
      if (!this.data.rendered || this._leaving) return;
      clearTimeout(this._timer);
      this._leaving = true;
      var dur = this.properties.duration;
      this.triggerEvent('beforeleave', {});
      // 容器里的 share-element 飞回页面上的那个（share-element.js）
      if (this._page && typeof this._page.__seLeave === 'function') this._page.__seLeave();
      this.setData(this._anim(false, dur, fromDy));
      this.triggerEvent('leave', {});
      var self = this;
      this._timer = setTimeout(function () {
        self._leaving = false;
        self.setData({ rendered: false, panelAnim: null, maskAnim: null });
        if (self._page && self._page.__pcActive === self) self._page.__pcActive = null;
        self.triggerEvent('afterleave', {});
      }, dur);
    },
    _onOverlay: function () { this.triggerEvent('clickoverlay', {}); },
    _ts: function (e) {
      var t = e.touches && e.touches[0];
      this._y0 = t ? t.clientY : null;
      this._dy = 0;
    },
    // 跟手：每一下只换面板的 animation（渲染层的 transform，只重画面板那块）
    _tm: function (e) {
      if (!this.properties.closeOnSlideDown || this._y0 === null || this._y0 === undefined || this._leaving) return;
      var t = e.touches && e.touches[0];
      if (!t) return;
      var dy = Math.max(0, t.clientY - this._y0);
      if (dy === this._dy) return;
      this._dy = dy;
      var a = wx.createAnimation({ duration: 0 });
      a.translate(0, dy).step({ duration: 0 });
      this.setData({ panelAnim: a.export() });
    },
    _te: function (e) {
      if (!this.properties.closeOnSlideDown || this._y0 === null || this._y0 === undefined) return;
      var t = (e.changedTouches && e.changedTouches[0]) || {};
      var dy = Math.max(0, (t.clientY === undefined ? this._y0 : t.clientY) - this._y0);
      this._y0 = null;
      if (e.type !== 'touchcancel' && dy > wx.getWindowInfo().windowHeight / 5) { this._leave(dy); return; }
      if (!this._dy) return;
      // 不够：弹回原位
      var a = wx.createAnimation({ duration: 200, timingFunction: 'ease' });
      a.translate(0, this._dy).step({ duration: 0 });
      a.translate(0, 0).step();
      this._dy = 0;
      this.setData({ panelAnim: a.export() });
    }
  }
});
