// 律安心：所有解答仅供参考，不构成法律意见。
App({
  globalData: { disclaimer: '平台内容仅供参考，不构成法律意见；具体案件请以执业律师书面意见为准。' },
  onLaunch() {
    const info = wx.getSystemInfoSync();
    this.globalData.windowHeight = info.windowHeight;
  }
});
