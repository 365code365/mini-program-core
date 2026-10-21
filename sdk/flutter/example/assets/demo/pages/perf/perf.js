Page({
  data: {
    screen: '', seconds: 0, phone: '', confirmed: '', picked: '还没选',
    fruits: ['苹果', '香蕉', '橙子'], fruitIndex: 0, videoState: '未播放', items: []
  },
  onLoad() {
    const i = wx.getSystemInfoSync();
    const items = [];
    for (let n = 1; n <= 40; n++) items.push(n);
    this.setData({
      screen: i.windowWidth + '×' + i.windowHeight + ' @' + i.pixelRatio + 'x，状态栏 ' + i.statusBarHeight +
        '，安全区底 ' + i.safeArea.bottom + '（' + i.platform + '）',
      items
    });
    this.timer = setInterval(() => this.setData({ seconds: this.data.seconds + 1 }), 1000);
  },
  onUnload() { clearInterval(this.timer); },
  onInput(e) { this.setData({ phone: e.detail.value }); },
  onConfirm(e) { this.setData({ confirmed: '（已确认 ' + e.detail.value + '）' }); },
  onPick() {
    wx.chooseImage({
      count: 1,
      success: (r) => this.setData({ picked: '宿主返回：' + r.tempFilePaths.join(', ') }),
      fail: (e) => this.setData({ picked: '失败：' + e.errMsg })
    });
  },
  onPick2(e) { this.setData({ fruitIndex: e.detail.value }); },
  onEnded(e) { this.setData({ videoState: '宿主回传 ended，播放了 ' + e.detail.duration + ' 秒' }); },
  toDetail() { wx.navigateTo({ url: '/pages/detail/detail?from=perf' }); }
});
