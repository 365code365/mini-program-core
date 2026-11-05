// <page-meta>（官方 component/page-meta）：页面属性配置节点，自己不渲染。属性变了就调对应的 wx 接口：
// background-text-style → setBackgroundTextStyle；background-color(-top / -bottom)、root-background-color → setBackgroundColor；
// scroll-top（带单位：px / rpx）+ scroll-duration → pageScrollTo。
// bind:resize {size: {windowWidth, windowHeight}}（wx.onWindowResize）；bind:scroll {scrollTop}：使用方绑了才接页面滚动 ——
// 页面没写 onPageScroll 时宿主根本不发滚动事件（host::scroll_view `page_wants_scroll_events`），所以只在绑了 scroll
// 时给页面包一层 onPageScroll（页面自己的照样调）；bind:scrolldone：scroll-top 引起的滚动按 scroll-duration 之后发一次。
// 取舍：page-style / page-font-size / root-font-size / page-orientation 不生效（要渲染层改页面根样式与 rem 基准，
// 引擎的 rem 固定 16px，写明）。
Component({
  options: { virtualHost: true },
  properties: {
    backgroundTextStyle: { type: String, value: '' },
    backgroundColor: { type: String, value: '' },
    backgroundColorTop: { type: String, value: '' },
    backgroundColorBottom: { type: String, value: '' },
    rootBackgroundColor: { type: String, value: '' },
    scrollTop: { type: String, value: '' },
    scrollDuration: { type: Number, value: 300 },
    pageStyle: { type: String, value: '' },
    pageFontSize: { type: String, value: '' },
    rootFontSize: { type: String, value: '' },
    pageOrientation: { type: String, value: '' }
  },
  observers: {
    backgroundTextStyle: function (v) { if (v) wx.setBackgroundTextStyle({ textStyle: v }); },
    'backgroundColor, backgroundColorTop, backgroundColorBottom, rootBackgroundColor': function (bg, top, bottom, root) {
      var o = {};
      if (bg || root) o.backgroundColor = bg || root;
      if (top) o.backgroundColorTop = top;
      if (bottom) o.backgroundColorBottom = bottom;
      if (Object.keys(o).length) wx.setBackgroundColor(o);
    },
    scrollTop: function (v) {
      if (v === '' || v === undefined || v === null) return;
      var s = String(v), n = parseFloat(s);
      if (!isFinite(n)) return;
      if (/rpx$/.test(s)) n = n * wx.getWindowInfo().windowWidth / 750;
      var self = this, dur = this.properties.scrollDuration;
      wx.pageScrollTo({ scrollTop: n, duration: dur });
      clearTimeout(this._done);
      this._done = setTimeout(function () { self.triggerEvent('scrolldone', {}); }, Math.max(0, dur));
    }
  },
  lifetimes: {
    attached: function () {
      var self = this;
      if (this._bound('resize')) {
        this._onResize = function (r) {
          var size = (r && r.size) || {};
          self.triggerEvent('resize', { size: { windowWidth: size.windowWidth, windowHeight: size.windowHeight } });
        };
        wx.onWindowResize(this._onResize);
      }
      var page = typeof __currentPage === 'object' ? __currentPage : null;
      if (page && this._bound('scroll')) {
        var own = page.onPageScroll;
        this._page = page;
        this._own = own;
        page.onPageScroll = function (e) {
          if (typeof own === 'function') own.call(page, e);
          self.triggerEvent('scroll', { scrollTop: e && e.scrollTop });
        };
      }
    },
    detached: function () {
      if (this._onResize) wx.offWindowResize(this._onResize);
      if (this._page) this._page.onPageScroll = this._own;
      clearTimeout(this._done);
    }
  },
  methods: {
    // 宿主节点（使用方写的 <page-meta>）上有没有这个事件的绑定（component.js 的冒泡路径第一帧）
    _bound: function (name) {
      var c = this.__chain, ev = c && c[0] && c[0].ev;
      return !!((ev && ev[name] && ev[name].length) || (this.__events && this.__events[name] && this.__events[name].length));
    }
  }
});
