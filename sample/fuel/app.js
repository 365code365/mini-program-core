const store = require('./utils/store.js');

App({
  globalData: { net: { networkType: 'unknown', isConnected: true } },
  onLaunch() {
    store.init();
    wx.getNetworkType({
      success: (r) => { this.globalData.net = { networkType: r.networkType, isConnected: r.networkType !== 'none' }; this.notifyNet(); }
    });
    // 网络变化推给当前栈里所有页面（页面实现 onNetChange 即可）
    wx.onNetworkStatusChange((r) => {
      this.globalData.net = { networkType: r.networkType, isConnected: r.isConnected };
      this.notifyNet();
      if (!r.isConnected) wx.showToast({ title: '网络已断开，价格可能不是最新', icon: 'none' });
    });
  },
  notifyNet() {
    getCurrentPages().forEach((p) => { if (typeof p.onNetChange === 'function') p.onNetChange(this.globalData.net); });
  }
});
