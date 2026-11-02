App({
  globalData: {
    // 首页点险种分类后切到「保险」tab：switchTab 不能带参数，用这里传
    productCat: ''
  },
  onLaunch() {
    // 记一下首次打开时间，「我的」页显示「已守护 N 天」
    if (!wx.getStorageSync('ins_since')) wx.setStorageSync('ins_since', Date.now());
  }
});
