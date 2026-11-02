const data = require('../../utils/spots.js');
const store = require('../../utils/store.js');
// 演示信号：火山引擎的公开 HLS 测试流（系统播放器都认 HLS）
const STREAM = 'https://sf1-cdn-tos.huoshanstatic.com/obj/media-fe/xgplayer_doc_video/hls/xgplayer-demo.m3u8';

Page({
  data: { spot: {}, src: STREAM, muted: true, playing: false, paused: false, status: '连接中…', size: '', shots: [], logs: [] },
  onLoad(q) {
    const s = data.byId(q.id) || data.SPOTS[0];
    this.setData({ spot: s });
    wx.setNavigationBarTitle({ title: s.name + ' · 实况' });
  },
  onReady() {
    this.ctx = wx.createLivePlayerContext('cam', this);
  },
  log(t) {
    const d = new Date();
    this.setData({ logs: [d.getHours() + ':' + ('0' + d.getMinutes()).slice(-2) + ':' + ('0' + d.getSeconds()).slice(-2) + '  ' + t].concat(this.data.logs).slice(0, 6) });
  },
  onState(e) {
    const c = e.detail.code;
    if (c === 2004) this.setData({ playing: true, status: '直播中' });
    if (c === 2007) this.setData({ status: '缓冲中…' });
    if (c === 2006 || c === -2301) this.setData({ playing: false, status: c === 2006 ? '直播已结束' : '信号中断' });
    this.log(c + ' ' + (e.detail.message || ''));
  },
  onNet(e) {
    const i = e.detail.info || {};
    if (i.videoWidth) this.setData({ size: i.videoWidth + '×' + i.videoHeight });
  },
  onFull(e) {
    this.log(e.detail.fullScreen ? '进入全屏' : '退出全屏');
  },
  call(m, opt) {
    if (!this.ctx) return;
    this.ctx[m](Object.assign({}, opt || {}, { fail: (r) => wx.showToast({ title: r.errMsg, icon: 'none' }) }));
  },
  toggle() {
    this.call(this.data.paused ? 'resume' : 'pause');
    this.setData({ paused: !this.data.paused, playing: this.data.paused });
  },
  mute() {
    this.setData({ muted: !this.data.muted });
  },
  full() {
    this.call('requestFullScreen', { direction: 90 });
  },
  snap() {
    if (!this.ctx) return;
    this.ctx.snapshot({
      quality: 'compressed',
      success: (r) => {
        // 临时文件存一份到本地，「我的 · 实况截图」里重开还在
        wx.saveFile({
          tempFilePath: r.tempImagePath,
          success: (f) => this.keep(f.savedFilePath),
          fail: () => this.keep(r.tempImagePath)
        });
      },
      fail: (r) => wx.showToast({ title: '还没有画面', icon: 'none' })
    });
  },
  keep(path) {
    const item = { path: path, spotId: this.data.spot.id, name: this.data.spot.name, at: Date.now() };
    store.set('shots', [item].concat(store.get('shots', [])).slice(0, 30));
    this.setData({ shots: [item].concat(this.data.shots).slice(0, 6) });
    wx.showToast({ title: '已保存到我的', icon: 'success' });
  },
  preview(e) {
    const urls = this.data.shots.map((s) => s.path);
    wx.previewImage({ urls: urls, current: urls[e.currentTarget.dataset.index] });
  }
});
