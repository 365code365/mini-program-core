// 攻略：打包在小程序里的离线网页（/h5/guide.html），由宿主的系统 WebView 打开。
// 网页里点「收藏本地点」「读完了」会 wx.miniProgram.postMessage —— 和微信一样，消息在返回时一次派给 bindmessage，
// 这里放进 globalData，地点详情页 onShow 时读
Page({
  data: { src: '' },
  onLoad(q) {
    this.id = Number(q.id || 1);
    this.setData({ src: '/h5/guide.html?id=' + this.id });
  },
  onMessage(e) {
    const list = e.detail.data || [];
    getApp().globalData.guideMessages = getApp().globalData.guideMessages.concat(list);
  },
  onLoaded() {},
  onError(e) {
    wx.showModal({ title: '攻略打不开', content: (e.detail && e.detail.errMsg) || '网页加载失败', showCancel: false, success: () => wx.navigateBack() });
  }
});
