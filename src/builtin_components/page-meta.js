// <page-meta>（官方 component/page-meta）：页面属性配置节点，自己不渲染。属性变了就调对应的 wx 接口：
// background-text-style → setBackgroundTextStyle；background-color(-top / -bottom)、root-background-color → setBackgroundColor；
// scroll-top（带单位：px / rpx）+ scroll-duration → pageScrollTo。
// 取舍：page-style / page-font-size / root-font-size / page-orientation 不生效（要渲染层改页面根样式，写明）；
// bindscroll / bindscrolldone / bindresize 不触发（页面滚动用 onPageScroll、窗口变化用 onResize）。
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
      wx.pageScrollTo({ scrollTop: n, duration: this.properties.scrollDuration });
    }
  }
});
