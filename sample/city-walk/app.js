App({
  globalData: { guideMessages: [] },
  onLaunch() {
    // 第一次打开：给一条示范路线（故宫 → 景山 → 北海）
    if (wx.getStorageSync('cw_route') === '') wx.setStorageSync('cw_route', [1, 4, 6]);
  }
});
