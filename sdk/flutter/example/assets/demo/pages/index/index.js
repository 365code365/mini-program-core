const catalog = require('../../utils/catalog.js');

Page({
  data: {
    screen: '',
    platform: '',
    theme: '',
    pageCount: 0,
    groups: [],
    quick: ''
  },
  onLoad() {
    const i = wx.getSystemInfoSync();
    this.setData({
      screen: i.windowWidth + '×' + i.windowHeight + ' @' + i.pixelRatio + 'x，状态栏 ' + i.statusBarHeight +
        '，安全区 top ' + i.safeArea.top + ' / bottom ' + i.safeArea.bottom + '（' + i.platform + ' ' + i.system + '）',
      platform: i.platform,
      theme: i.theme || 'light',
      pageCount: catalog.count(),
      groups: catalog.pick(['lang', 'page'])
    });
  },
  go(e) {
    wx.navigateTo({ url: e.currentTarget.dataset.url });
  },
  toTab(e) {
    wx.switchTab({ url: e.currentTarget.dataset.url });
  },
  toast() {
    wx.showToast({ title: '来自引擎的 Toast', icon: 'success' });
  },
  sheet() {
    wx.showActionSheet({
      itemList: ['拍照', '从相册选择', '保存图片'],
      success: (r) => this.setData({ quick: 'ActionSheet 选了第 ' + r.tapIndex + ' 项（tapIndex 是 number）' }),
      fail: (e) => this.setData({ quick: e.errMsg })
    });
  },
  modal() {
    wx.showModal({
      title: '提示',
      content: '这个弹窗由引擎画在自己的表面上，宿主不用实现',
      success: (r) => this.setData({ quick: r.confirm ? '点了确定' : '点了取消' })
    });
  }
});
