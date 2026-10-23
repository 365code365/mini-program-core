// <navigation-bar>（官方 component/navigation-bar，只能放在 page-meta 里）：导航栏配置节点，自己不渲染。
// 属性变了就调 setNavigationBarTitle / setNavigationBarColor / show|hideNavigationBarLoading（宿主或引擎画的导航栏跟着变）
Component({
  options: { virtualHost: true },
  properties: {
    title: { type: String, value: '' },
    loading: { type: Boolean, value: false },
    frontColor: { type: String, value: '' },
    backgroundColor: { type: String, value: '' },
    colorAnimationDuration: { type: Number, value: 0 },
    colorAnimationTimingFunc: { type: String, value: 'linear' }
  },
  observers: {
    title: function (v) { if (v) wx.setNavigationBarTitle({ title: v }); },
    loading: function (v) { if (v) wx.showNavigationBarLoading(); else wx.hideNavigationBarLoading(); },
    'frontColor, backgroundColor': function (front, bg) {
      if (!front && !bg) return;
      var p = this.properties;
      wx.setNavigationBarColor({
        frontColor: front || '#000000',
        backgroundColor: bg || '#ffffff',
        animation: { duration: p.colorAnimationDuration, timingFunc: p.colorAnimationTimingFunc }
      });
    }
  }
});
