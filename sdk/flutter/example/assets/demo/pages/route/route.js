// 路由与页面生命周期：每个回调往 globalData.routeLog 记一条，onShow 时把流水显示出来。
// 同一个页面可以被 navigateTo 多次，用自增序号区分是哪一个实例。
let seq = 0;

function stamp() {
  const d = new Date();
  const p = (n) => (n < 10 ? '0' + n : '' + n);
  return p(d.getMinutes()) + ':' + p(d.getSeconds());
}

Page({
  data: {
    route: '',
    query: '',
    depth: 0,
    stack: [],
    log: []
  },
  record(what) {
    const g = getApp().globalData;
    g.routeLog.unshift(stamp() + '  #' + this.no + ' ' + what);
    if (g.routeLog.length > 14) g.routeLog.length = 14;
  },
  refresh() {
    const pages = getCurrentPages();
    this.setData({
      depth: pages.length,
      stack: pages.map((p) => p.route),
      log: getApp().globalData.routeLog.slice()
    });
  },
  onLoad(query) {
    this.no = ++seq;
    this.setData({ route: this.route, query: JSON.stringify(query) });
    this.record('onLoad ' + JSON.stringify(query));
  },
  onShow() {
    this.record('onShow');
    this.refresh();
  },
  onReady() {
    this.record('onReady');
    this.refresh();
  },
  onHide() {
    this.record('onHide');
  },
  onUnload() {
    this.record('onUnload');
  },

  navTo() { wx.navigateTo({ url: '/pages/detail/detail?from=route&n=' + this.no }); },
  navSelf() { wx.navigateTo({ url: '/pages/route/route?n=' + (this.no + 1) }); },
  redirect() { wx.redirectTo({ url: '/pages/detail/detail?from=redirect' }); },
  back() { wx.navigateBack({}); },
  back2() { wx.navigateBack({ delta: 2 }); },
  switchTab() { wx.switchTab({ url: '/pages/apis/apis' }); },
  relaunch() { wx.reLaunch({ url: '/pages/index/index' }); },
  clearLog() {
    getApp().globalData.routeLog = [];
    this.refresh();
  }
});
