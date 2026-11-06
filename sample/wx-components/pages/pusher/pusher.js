// 直播推流：<live-pusher> 相机预览同层渲染，LivePusherContext 推 RTMP；美颜 / 美白 / 滤镜、背景音（BGM 混进推流）、
// 小窗（picture-in-picture-mode="push"：离开本页时缩成浮窗、推流不断）、截图
const app = getApp();
const FILTERS = [
  ['standard', '标准'], ['pink', '粉嫩'], ['nostalgia', '怀旧'], ['blues', '蓝调'], ['romantic', '浪漫'],
  ['cool', '清凉'], ['fresher', '清新'], ['solor', '日系'], ['aestheticism', '唯美'], ['whitening', '美白'], ['cerisered', '樱红']
];
// 推流状态码（官方 live-pusher 文档）
const STATE = {
  1001: '已连上推流服务器', 1002: '开始推流', 1003: '摄像头打开', 1007: '首帧画面采集完成', 1008: '编码器启动',
  1101: '上行带宽不足', 1102: '网络断开，自动重连中', 3001: 'RTMP DNS 解析失败', 3002: 'RTMP 服务器连接失败',
  3003: 'RTMP 服务器握手失败', 3004: '服务器主动断开', 3005: 'RTMP 读写失败', '-1301': '打开摄像头失败', '-1302': '打开麦克风失败', '-1307': '网络断开，重连已用完'
};
Page({
  data: {
    log: [], url: 'rtmp://127.0.0.1:1935/live/lab', mic: true, position: 'front', beauty: 0, whiteness: 0,
    filter: 'standard', filterName: '标准', pip: '', live: false, stateText: '未推流', net: '-', bgmText: '', shot: ''
  },
  onReady() { this.ctx = wx.createLivePusherContext(); },
  onUrl(e) { this.setData({ url: e.detail.value.trim() }); },
  call(m, opt) {
    return new Promise((resolve) => {
      this.ctx[m](Object.assign({}, opt || {}, {
        success: (r) => { app.note(this, `${m} ✓`); resolve(r || {}); },
        fail: (r) => { app.note(this, `${m} ✗ ${r.errMsg}`); resolve(null); }
      }));
    });
  },
  cmd(e) { this.call(e.currentTarget.dataset.m); },
  start() {
    if (!/^rtmps?:\/\//.test(this.data.url)) return wx.showToast({ title: '先填 rtmp:// 地址', icon: 'none' });
    this.call('start');
  },
  onState(e) {
    const c = e.detail.code;
    const text = STATE[c] || e.detail.message || '';
    if (c === 1002) this.setData({ live: true });
    if (c === 3004 || c === -1307) this.setData({ live: false });
    this.setData({ stateText: `${c} ${text}` });
    app.note(this, `statechange ${c} ${text}`);
  },
  onNet(e) {
    const i = e.detail.info || {};
    this.setData({ net: `${i.videoBitrate || 0} / ${i.audioBitrate || 0} kbps · ${i.videoFPS || 0} fps · ${i.videoWidth || 0}×${i.videoHeight || 0}` });
  },
  onError(e) { app.note(this, `binderror ${JSON.stringify(e.detail)}`); },
  beautyOn() { this.setData({ beauty: this.data.beauty ? 0 : 9, whiteness: this.data.whiteness ? 0 : 5 }); },
  nextFilter() {
    const i = (FILTERS.findIndex((f) => f[0] === this.data.filter) + 1) % FILTERS.length;
    this.setData({ filter: FILTERS[i][0], filterName: FILTERS[i][1] });
  },
  toggleMic() { this.setData({ mic: !this.data.mic }); },
  bgm() { this.call('playBGM', { url: '/media/bgm.m4a' }); },
  onBgm(e) { app.note(this, e.type); if (e.type === 'bgmcomplete') this.setData({ bgmText: '放完了' }); },
  onBgmProgress(e) { this.setData({ bgmText: `${(e.detail.progress / 1000).toFixed(1)} / ${(e.detail.duration / 1000).toFixed(1)} 秒` }); },
  onPip(e) { app.note(this, e.type); },
  pipGo() {
    this.setData({ pip: 'push' });
    setTimeout(() => wx.navigateTo({ url: '/pages/text/text' }), 50);
  },
  async snap() {
    const r = await this.call('snapshot', { quality: 'compressed' });
    if (r) this.setData({ shot: r.tempImagePath });
  }
});
