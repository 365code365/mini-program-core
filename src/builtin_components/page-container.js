// <page-container>（官方 component/page-container）：页面里的「假页」弹层。
// show 变 true → beforeenter / enter / afterenter；变 false、点了返回（返回键、导航栏返回、wx.navigateBack）、
// close-on-slide-down 下滑 → beforeleave / leave / afterleave。返回操作只关它、不离开页面（__backInterceptors）。
// 入场 / 离场用 wx.createAnimation（渲染层逐帧插值 transform / opacity）：bottom 从下往上、top 从上往下、
// right 从右往左、center 淡入 + 轻微放大。遮罩点了只发 clickoverlay（与微信一致：关不关由页面决定）。
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
    }
  },
  methods: {
    _noop: function () {},
    _offset: function () {
      var w = wx.getWindowInfo();
      var p = this.properties.position;
      return p === 'top' ? { y: -w.windowHeight } : p === 'right' ? { x: w.windowWidth } : p === 'center' ? { s: 0.9 } : { y: w.windowHeight };
    },
    _anim: function (toVisible, dur) {
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
        if (o.s) panel.scale(o.s).opacity(0).step(); else panel.translate(o.x || 0, o.y || 0).step();
        mask.opacity(0).step();
      }
      return { panelAnim: panel.export(), maskAnim: mask.export() };
    },
    _enter: function () {
      if (this.data.rendered && !this._leaving) return;
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
    _leave: function () {
      if (!this.data.rendered || this._leaving) return;
      clearTimeout(this._timer);
      this._leaving = true;
      var dur = this.properties.duration;
      this.triggerEvent('beforeleave', {});
      this.setData(this._anim(false, dur));
      this.triggerEvent('leave', {});
      var self = this;
      this._timer = setTimeout(function () {
        self._leaving = false;
        self.setData({ rendered: false, panelAnim: null, maskAnim: null });
        self.triggerEvent('afterleave', {});
      }, dur);
    },
    _onOverlay: function () { this.triggerEvent('clickoverlay', {}); },
    // close-on-slide-down：在弹层上往下划过 1/5 窗口高就关
    _ts: function (e) { var t = e.touches && e.touches[0]; this._y0 = t ? t.clientY : null; },
    _tm: function () {},
    _te: function (e) {
      if (!this.properties.closeOnSlideDown || this._y0 === null || this._y0 === undefined) return;
      var t = (e.changedTouches && e.changedTouches[0]) || {};
      var dy = (t.clientY || 0) - this._y0;
      this._y0 = null;
      if (dy > wx.getWindowInfo().windowHeight / 5) this._leave();
    }
  }
});
