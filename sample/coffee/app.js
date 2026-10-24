App({
  globalData: {
    storeId: 'st1'
  },
  onLaunch() {
    const s = wx.getStorageSync('cf_store');
    if (s) this.globalData.storeId = s;
  }
});
