// live-player 演示：换流、LivePlayerContext、状态码（bindstatechange）、网络状态（bindnetstatus）、全屏、截图。
// ?auto=1：等到 2004（视频播放开始）→ 截图 → 静音 → 打一行 `NC-CHECK live …`（真机脚本据此判通过）
const SOURCES = [
  { name: '火山引擎 HLS 演示流', url: 'https://sf1-cdn-tos.huoshanstatic.com/obj/media-fe/xgplayer_doc_video/hls/xgplayer-demo.m3u8' },
  { name: 'Mux HLS 测试流', url: 'https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8' },
  { name: '包内 mp4（回放）', url: '/images/clip.mp4' },
  { name: 'RTMP 地址（系统播放器不支持，看 -2301）', url: 'rtmp://live.example.invalid/app/stream' }
];

Page({
  data: {
    sources: SOURCES,
    current: 0,
    src: SOURCES[0].url,
    muted: false,
    fit: 'contain',
    ctxResult: '自动起播（autoplay）；下面的按钮调 LivePlayerContext',
    shot: '',
    size: '-',
    fullscreen: '否',
    logs: []
  },
  onLoad(q) {
    this.auto = q && q.auto === '1';
    this.chain = q && q.chain === '1';
    this.started = Date.now();
    if (q && q.src) this.setData({ src: decodeURIComponent(q.src), current: -1 });
  },
  onReady() {
    this.ctx = wx.createLivePlayerContext('player', this);
  },
  log(s) {
    const t = new Date();
    this.setData({ logs: [t.getMinutes() + ':' + ('0' + t.getSeconds()).slice(-2) + ' ' + s].concat(this.data.logs).slice(0, 12) });
    console.log('[live] ' + s);
  },
  pick(e) {
    const i = Number(e.currentTarget.dataset.index);
    this.setData({ current: i, src: SOURCES[i].url, shot: '', size: '-' });
    this.log('换流：' + SOURCES[i].name);
  },
  call(m, opt) {
    return new Promise((resolve) => {
      this.ctx[m](Object.assign({}, opt || {}, {
        success: (r) => { this.setData({ ctxResult: m + ' ✓ ' + r.errMsg }); resolve(r); },
        fail: (r) => { this.setData({ ctxResult: m + ' ✗ ' + r.errMsg }); resolve(null); }
      }));
    });
  },
  cmd(e) { this.call(e.currentTarget.dataset.m); },
  full() { this.call('requestFullScreen', { direction: 90 }); },
  async snap() {
    const r = await this.call('snapshot');
    if (r) this.setData({ shot: r.tempImagePath, ctxResult: 'snapshot ✓ ' + r.width + '×' + r.height + ' ' + r.tempImagePath });
    return r;
  },
  toggleFit() { this.setData({ fit: this.data.fit === 'contain' ? 'fillCrop' : 'contain' }); },
  onState(e) {
    const c = e.detail.code;
    this.log('statechange ' + c + ' ' + (e.detail.message || ''));
    if (c === 2004 && this.auto && !this.autoDone) {
      this.autoDone = true;
      setTimeout(() => this.runAuto(), 1200);
    }
    if (c === -2301 && this.auto && !this.autoDone) {
      this.autoDone = true;
      console.log('NC-CHECK live ' + JSON.stringify({ ok: false, code: c, src: this.data.src }));
      if (this.chain) wx.redirectTo({ url: '/pages/camera/camera?auto=1&chain=1' });
    }
  },
  onNet(e) {
    const i = e.detail.info || {};
    if (i.videoWidth) this.setData({ size: i.videoWidth + '×' + i.videoHeight });
  },
  onFullscreen(e) {
    this.setData({ fullscreen: e.detail.fullScreen ? '是（' + e.detail.direction + '）' : '否' });
    this.log('fullscreenchange ' + JSON.stringify(e.detail));
  },
  async runAuto() {
    const shot = await this.snap();
    const mute = await this.call('mute');
    const pause = await this.call('pause');
    const resume = await this.call('resume');
    const r = {
      ok: !!(shot && shot.width > 0 && mute && pause && resume),
      snapshot: shot ? shot.width + 'x' + shot.height : null,
      size: this.data.size,
      ms: Date.now() - this.started
    };
    console.log('NC-CHECK live ' + JSON.stringify(r));
    // 接着验相机与推流（camera → pusher，最后一页打 NC-CHECK done）
    if (this.chain) wx.redirectTo({ url: '/pages/camera/camera?auto=1&chain=1' });
  }
});
