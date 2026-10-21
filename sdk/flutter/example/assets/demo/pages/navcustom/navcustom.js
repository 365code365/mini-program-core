Page({
  data: {
    statusBar: 20,
    navHeight: 44,
    safeTop: 0,
    safeBottom: 0,
    windowHeight: 0,
    rows: [1, 2, 3, 4, 5, 6]
  },
  onLoad() {
    const i = wx.getSystemInfoSync();
    this.setData({
      statusBar: i.statusBarHeight,
      // 安卓上标题栏一般 48，iOS 44
      navHeight: i.platform === 'android' ? 48 : 44,
      safeTop: i.safeArea.top,
      safeBottom: i.safeArea.bottom,
      windowHeight: i.windowHeight
    });
  },
  back() {
    if (getCurrentPages().length > 1) wx.navigateBack({});
    else wx.reLaunch({ url: '/pages/index/index' });
  },
  toast() {
    wx.showToast({ title: '底部按钮栏', icon: 'none' });
  }
});
