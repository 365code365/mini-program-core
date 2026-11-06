// 直播拉流：<live-player> 换流、LivePlayerContext（播放控制、静音、全屏、截图）、状态码与网络状态
const app = getApp();
const SOURCES = [
  { name: '火山引擎 HLS 演示流', url: 'https://sf1-cdn-tos.huoshanstatic.com/obj/media-fe/xgplayer_doc_video/hls/xgplayer-demo.m3u8' },
  { name: 'Mux HLS 测试流', url: 'https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8' },
  { name: '包内 mp4（回放）', url: '/media/clip.mp4' },
  { name: 'RTMP（系统播放器不支持，看 -2301）', url: 'rtmp://live.example.invalid/app/stream' }
];
// 拉流状态码（官方 live-player 文档）
const STATE = {
  2001: '已连接服务器', 2002: '已连接，开始拉流', 2003: '收到首帧数据', 2004: '视频播放开始', 2006: '播放结束',
  2007: '视频加载中', 2008: '解码器启动', 2009: '分辨率改变', 2103: '网络断开，自动重连', '-2301': '网络断开，重连已用完（或宿主不支持这个协议）'
};
Page({
  data: { log: [], sources: SOURCES, current: 0, src: SOURCES[0].url, custom: '', muted: false, fit: 'contain', stateText: '加载中', size: '-', fullscreen: '否', shot: '' },
  onReady() { this.ctx = wx.createLivePlayerContext('player', this); },
  pick(e) {
    const i = Number(e.currentTarget.dataset.index);
    this.setData({ current: i, src: SOURCES[i].url, size: '-', shot: '', stateText: '加载中' });
    app.note(this, `换流：${SOURCES[i].name}`);
  },
  onCustom(e) { this.setData({ custom: e.detail.value.trim() }); },
  playCustom() {
    if (!this.data.custom) return;
    this.setData({ current: -1, src: this.data.custom, size: '-', shot: '', stateText: '加载中' });
    app.note(this, `换流：${this.data.custom}`);
  },
  call(m, opt) {
    return new Promise((resolve) => {
      this.ctx[m](Object.assign({}, opt || {}, {
        success: (r) => { app.note(this, `${m} ✓`); resolve(r || {}); },
        fail: (r) => { app.note(this, `${m} ✗ ${r.errMsg}`); resolve(null); }
      }));
    });
  },
  cmd(e) { this.call(e.currentTarget.dataset.m); },
  toggleMute() { this.setData({ muted: !this.data.muted }); },
  full() { this.call('requestFullScreen', { direction: 90 }); },
  toggleFit() { this.setData({ fit: this.data.fit === 'contain' ? 'fillCrop' : 'contain' }); },
  async snap() {
    const r = await this.call('snapshot');
    if (r) this.setData({ shot: r.tempImagePath });
  },
  onState(e) {
    const c = e.detail.code;
    const text = STATE[c] || e.detail.message || '';
    this.setData({ stateText: `${c} ${text}` });
    app.note(this, `statechange ${c} ${text}`);
  },
  onNet(e) {
    const i = e.detail.info || {};
    if (i.videoWidth) this.setData({ size: `${i.videoWidth}×${i.videoHeight}` });
  },
  onFull(e) {
    this.setData({ fullscreen: e.detail.fullScreen ? `是（${e.detail.direction}）` : '否' });
    app.note(this, `fullscreenchange ${JSON.stringify(e.detail)}`);
  },
  onError(e) { app.note(this, `binderror ${JSON.stringify(e.detail)}`); }
});
