App({
  globalData: {},
  onLaunch() {
    // 上次没结束的骑行：首页会显示「骑行中」横幅，点了回到骑行页继续计时
    const ride = wx.getStorageSync('bike_ride');
    this.globalData.hasRide = !!ride;
  }
});
