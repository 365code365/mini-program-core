const store = require('./utils/store.js');

App({
  globalData: { scenic: 'huangshan' },
  onLaunch() {
    store.ensureSeed();
    const last = wx.getStorageSync('scenic_last');
    if (last) this.globalData.scenic = last;
  },
  // 记住最近看的景区：导览 tab 默认讲它
  setScenic(id) {
    this.globalData.scenic = id;
    wx.setStorageSync('scenic_last', id);
  }
});
