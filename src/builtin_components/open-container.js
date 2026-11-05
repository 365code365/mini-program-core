// <open-container>（官方 component/open-container，Skyline）：容器转场组件。源页面上是 closed 外观：
// closed-color（默认 white）背景、closed-border-radius 圆角（px）、closed-elevation 影深（Material 的 elevation，
// 换成 box-shadow：0 0.5e px e px rgba(0,0,0,.24)，近似，待验）。点它之后 wx.navigateTo 时官方会让下一页从它的
// 位置大小渐显放大（open-* 是放大到整页之后的外观、transition-* 是这段动画）。
// 取舍（写明）：引擎的 navigateTo 转场（host::route_anim）只有整页推入，没有「从某个节点放大」这一种 —— 这里只做
// closed 外观，open-* / middle-color / transition-* 收下但不生效；返回之后本来就回到 closed 外观。
Component({
  properties: {
    closedColor: { type: String, value: 'white' },
    closedElevation: { type: Number, value: 0 },
    closedBorderRadius: { type: Number, value: 0 },
    middleColor: { type: String, value: '' },
    openColor: { type: String, value: 'white' },
    openElevation: { type: Number, value: 0 },
    openBorderRadius: { type: Number, value: 0 },
    transitionDuration: { type: Number, value: 300 },
    transitionType: { type: String, value: 'fade' }
  },
  data: { shadow: '' },
  observers: {
    closedElevation: function (e) { this._shadow(e); }
  },
  lifetimes: {
    attached: function () { this._shadow(this.properties.closedElevation); }
  },
  methods: {
    _shadow: function (e) {
      var s = e > 0 ? 'box-shadow:0 ' + (e / 2) + 'px ' + e + 'px rgba(0,0,0,0.24);' : '';
      if (s !== this.data.shadow) this.setData({ shadow: s });
    }
  }
});
