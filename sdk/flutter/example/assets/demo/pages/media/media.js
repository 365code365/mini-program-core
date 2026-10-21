Page({
  data: {
    modes: ['scaleToFill', 'aspectFit', 'aspectFill'],
    imgLog: '',
    gallery: ['/images/coffee.jpg', '/images/earbuds.jpg', '/images/phone.jpg', '/images/watch.jpg', '/images/sneakers.jpg', '/images/cake.jpg'],
    // https：Android 9+ 默认禁明文 HTTP、iOS 有 ATS，宿主播放器拿 http 地址会直接报错
    videoSrc: 'https://vjs.zencdn.net/v/oceans.mp4',
    videoLog: '点视频区域，宿主回传 play / timeupdate / pause / ended'
  },
  logs: [],
  addLog(line) {
    this.logs.unshift(line);
    this.logs = this.logs.slice(0, 4);
    this.setData({ imgLog: this.logs.join('\n') });
  },
  onLoadImg(e) {
    this.addLog('load ' + e.currentTarget.dataset.name + '：原始尺寸 ' + e.detail.width + '×' + e.detail.height);
  },
  onErrImg(e) {
    this.addLog('error ' + e.currentTarget.dataset.name + '：' + e.detail.errMsg);
  },
  preview(e) {
    const i = e.currentTarget.dataset.index;
    wx.previewImage({ urls: this.data.gallery, current: this.data.gallery[i] });
  },
  onVideo(e) {
    this.setData({ videoLog: 'bind' + (e.type || '?') + ' ' + JSON.stringify(e.detail || {}) });
  },
  onTime(e) {
    const d = e.detail;
    this.setData({ videoLog: 'bindtimeupdate ' + Number(d.currentTime).toFixed(1) + ' / ' + d.duration + ' 秒' });
  }
});
