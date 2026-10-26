// 小程序跳转中心：演示「小程序打开小程序」的三种写法，并展示自己被打开时收到的来源参数
App({
  // enterOptions：最近一次 onShow 收到的参数（热启动时与 getLaunchOptionsSync 不同）
  globalData: { enterOptions: null },
  onLaunch(options) { this.globalData.enterOptions = options || null; },
  onShow(options) { this.globalData.enterOptions = options || this.globalData.enterOptions; }
});
