// <match-media>（官方 component/match-media）：窗口尺寸满足条件才渲染子节点。
// 属性不写 = 不限制（这里用 -1 表示没写）；窗口变了（转屏、分屏）重新判（this.createMediaQueryObserver，worker-mq.js）
Component({
  options: { virtualHost: true },
  properties: {
    minWidth: { type: Number, value: -1 },
    maxWidth: { type: Number, value: -1 },
    width: { type: Number, value: -1 },
    minHeight: { type: Number, value: -1 },
    maxHeight: { type: Number, value: -1 },
    height: { type: Number, value: -1 },
    orientation: { type: String, value: '' }
  },
  data: { matches: false },
  observers: {
    'minWidth, maxWidth, width, minHeight, maxHeight, height, orientation': function () { if (this._mq) this._observe(); }
  },
  lifetimes: {
    attached: function () { this._observe(); },
    detached: function () { if (this._mq) this._mq.disconnect(); }
  },
  methods: {
    _descriptor: function () {
      var p = this.properties, d = {};
      ['minWidth', 'maxWidth', 'width', 'minHeight', 'maxHeight', 'height'].forEach(function (k) { if (p[k] >= 0) d[k] = p[k]; });
      if (p.orientation === 'landscape' || p.orientation === 'portrait') d.orientation = p.orientation;
      return d;
    },
    _observe: function () {
      if (this._mq) this._mq.disconnect();
      var self = this, d = this._descriptor();
      // 首帧就要对：同步判一次（observer 的首个回调在下一轮事件循环）
      this.setData({ matches: __mqMatch(d) });
      this._mq = this.createMediaQueryObserver();
      this._mq.observe(d, function (r) { if (r.matches !== self.data.matches) self.setData({ matches: r.matches }); });
    }
  }
});
