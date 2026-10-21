// 全局：globalData 给各页共享状态；生命周期记一条流水，宿主页（tab「宿主」）展示出来，
// 切到后台再回来就能看到 App.onHide / App.onShow 被宿主转发进来。
App({
  globalData: {
    launchAt: 0,
    showCount: 0,
    hideCount: 0,
    lifecycle: [],
    // 路由页把自己的生命周期也记在这里：页面卸载后回到上一页还能看到 onUnload
    routeLog: []
  },
  onLaunch() {
    const info = wx.getSystemInfoSync();
    this.globalData.launchAt = Date.now();
    this.record('App.onLaunch scene=' + wx.getLaunchOptionsSync().scene);
    console.log('onLaunch', info.platform, info.windowWidth + 'x' + info.windowHeight, 'dpr', info.pixelRatio,
      'safeArea.bottom', info.safeArea.bottom);
  },
  onShow() {
    this.globalData.showCount++;
    this.record('App.onShow');
  },
  onHide() {
    this.globalData.hideCount++;
    this.record('App.onHide');
  },
  onError(msg) {
    console.error('App.onError', msg);
  },
  record(text) {
    const d = new Date();
    const pad = (n) => (n < 10 ? '0' + n : '' + n);
    const line = pad(d.getHours()) + ':' + pad(d.getMinutes()) + ':' + pad(d.getSeconds()) + '  ' + text;
    const list = this.globalData.lifecycle;
    list.push(line);
    if (list.length > 12) list.shift();
  }
});
