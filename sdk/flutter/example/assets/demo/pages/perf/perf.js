const fmt = (t) => {
  const s = Math.max(0, Math.floor(t));
  return String(Math.floor(s / 60)).padStart(2, '0') + ':' + String(s % 60).padStart(2, '0');
};

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
  // 视频的组件事件（与微信同形）：loadedmetadata {width, height, duration}、timeupdate {currentTime, duration}、
  // ended / play / pause 的 detail 是空对象 —— 时长要自己从前两个里记下来
  onMeta(e) { this.duration = e.detail.duration; },
  onPlay() { this.sec = -1; this.setData({ videoState: '播放中' }); },
  onPause() { this.setData({ videoState: '已暂停 ' + fmt(this.at || 0) + ' / ' + fmt(this.duration || 0) }); },
  onTime(e) {
    this.at = e.detail.currentTime;
    this.duration = e.detail.duration || this.duration;
    // timeupdate 一秒四次：秒数变了才 setData
    const s = Math.floor(this.at);
    if (s === this.sec) return;
    this.sec = s;
    this.setData({ videoState: '播放中 ' + fmt(s) + ' / ' + fmt(this.duration) });
  },
  onEnded() { this.setData({ videoState: '播放结束，时长 ' + (this.duration || 0).toFixed(1) + ' 秒' }); },
  onFullscreen(e) {
    const d = e.detail.direction === 'horizontal' ? '横屏' : '竖屏';
    this.setData({ videoState: e.detail.fullScreen ? '全屏（' + d + '）' : '退出全屏' });
  },
  onControls(e) { console.log('controlstoggle show=' + e.detail.show); },
  fullLandscape() { wx.createVideoContext('v1').requestFullScreen({ direction: 90 }); },
  fullPortrait() { wx.createVideoContext('v1').requestFullScreen({ direction: 0 }); },
  toDetail() { wx.navigateTo({ url: '/pages/detail/detail?from=perf' }); }
});
