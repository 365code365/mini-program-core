// <share-element>（官方 component/share-element）：共享元素，与 page-container 配合 —— 页面上放一个、page-container
// 里放一个同 key 的；容器显示时，容器里那个（transform 为真）从页面上那个的位置大小飞到自己的位置大小，
// 容器退出时飞回去（官方：「当前页面容器退出时，会产生返回动画」）。duration 默认 300、easing-function 默认 ease-out。
//
// 做法：飞的是容器里那个节点自己（animation 通道：渲染层按帧插值 translate + scale，只重画它那块），
// 起止矩形用 SelectorQuery 量；飞的时候页面上那个隐藏（opacity 0），容器关掉之后再显示。
// 取舍（写明）：
// - 只做 page-container 这条（WebView 与 Skyline 都有）；Skyline 的跨页面路由飞跃（navigateTo 时同 key 元素飞到新页面）
//   要宿主转场层画「飞跃物」，引擎的路由转场（host::route_anim）没有这一层，不做；shuttle-on-push / shuttle-on-pop /
//   rect-tween-type / transition-on-gesture / worklet:onframe 收下不生效（插值是直线 + easing-function）；
// - 飞跃相对容器里的布局位置算：容器自己的入场动画（bottom 从下往上滑）叠在上面，起点会跟着面板偏一段 ——
//   要准就给 page-container 用 position="center"（只淡入、不位移）。
Component({
  properties: {
    key: { type: String, value: '' },
    transform: { type: Boolean, value: false },
    duration: { type: Number, value: 300 },
    easingFunction: { type: String, value: 'ease-out' },
    transitionOnGesture: { type: Boolean, value: false },
    shuttleOnPush: { type: String, value: 'to' },
    shuttleOnPop: { type: String, value: 'to' },
    rectTweenType: { type: String, value: 'materialRectArc' }
  },
  data: { uid: '', anim: null, hidden: false },
  lifetimes: {
    created: function () {
      globalThis.__seSeq = (globalThis.__seSeq || 0) + 1;
      this.data.uid = '__se' + globalThis.__seSeq;
    },
    attached: function () {
      var page = this._page = typeof __currentPage === 'object' ? __currentPage : null;
      var key = this.properties.key;
      if (!page || !key) return;
      var reg = page.__shareEls = page.__shareEls || {};
      var list = reg[key] = reg[key] || [];
      var pc = page.__pcActive;
      // 容器正在弹出时挂上来的、且页面上已经有同 key 的：它就是目标
      var src = list.filter(function (o) { return o._page === page; })[0];
      list.push(this);
      if (this.properties.transform && src && pc && pc.data.rendered && !pc._leaving) {
        this._src = src;
        var self = this;
        setTimeout(function () { self._fly(true); }, 0);
      }
      if (!page.__seLeave) {
        // page-container 离场时叫（page-container.js `_leave`）：所有正飞在外面的目标飞回去
        page.__seLeave = function () {
          var r = page.__shareEls || {};
          for (var k in r) if (r.hasOwnProperty(k)) r[k].forEach(function (o) { if (o._src) o._fly(false); });
        };
      }
    },
    detached: function () {
      var page = this._page, key = this.properties.key, self = this;
      if (this._src) this._src.setData({ hidden: false });
      if (page && page.__shareEls && page.__shareEls[key]) {
        page.__shareEls[key] = page.__shareEls[key].filter(function (o) { return o !== self; });
      }
    }
  },
  methods: {
    // forward：页面上那个 → 自己；否则自己 → 页面上那个
    _fly: function (forward) {
      var self = this, src = this._src;
      if (!src) return;
      wx.createSelectorQuery().select('.' + src.data.uid).boundingClientRect().select('.' + this.data.uid).boundingClientRect().exec(function (res) {
        var a = res && res[0], b = res && res[1];
        if (!a || !b || !b.width || !b.height) return;
        var dx = (a.left + a.width / 2) - (b.left + b.width / 2), dy = (a.top + a.height / 2) - (b.top + b.height / 2);
        var sx = a.width / b.width, sy = a.height / b.height;
        var p = self.properties;
        var an = wx.createAnimation({ duration: p.duration, timingFunction: p.easingFunction || 'ease-out' });
        if (forward) {
          an.translate(dx, dy).scale(sx, sy).step({ duration: 0 });
          an.translate(0, 0).scale(1, 1).step();
        } else {
          an.translate(dx, dy).scale(sx, sy).step();
        }
        src.setData({ hidden: true });
        self.setData({ anim: an.export() });
        if (!forward) {
          setTimeout(function () { src.setData({ hidden: false }); self._src = null; }, p.duration);
        }
      });
    }
  }
});
