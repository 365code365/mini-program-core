// live-pusher 演示：相机预览同层渲染，LivePusherContext 推 RTMP（宿主硬编 H.264 + AAC）。
// ?auto=1：等 1003（摄像头打开）→ start → 等 1002（开始推流）→ 美颜 9 / 美白 5 / 怀旧滤镜 + playBGM（3 秒 880Hz）推 2 秒 →
// 全关再推 2 秒（收端按前后两段比颜色、比音量，tools/android-native-components.sh）→ 小窗：picture-in-picture-mode=push、
// navigateTo 别的页、2.5 秒后返回（enter / leave 都要到）→ snapshot / getMaxZoom → stop → 打一行 `NC-CHECK pusher …`；
// chain=1 时这是最后一页，再打 `NC-CHECK done`。?push=<rtmp 地址> 换地址（默认本机 127.0.0.1:1935，配 adb reverse）
const FILTERS = ['standard', 'pink', 'nostalgia', 'blues', 'romantic', 'cool', 'fresher', 'solor', 'aestheticism', 'whitening', 'cerisered'];
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
Page({
  data: { url: 'rtmp://127.0.0.1:1935/live/nc', mic: true, position: 'front', result: '', shot: '', net: '-', logs: [],
    beauty: 0, whiteness: 0, filter: 'standard', pip: '' },
  onLoad(q) {
    this.auto = q && q.auto === '1';
    this.chain = q && q.chain === '1';
    this.started = Date.now();
    this.codes = [];
    this.netOk = false;
    this.ev = { bgmStart: false, bgmProgress: 0, bgmComplete: false, pipEnter: false, pipLeave: false };
    if (q && q.push) this.setData({ url: decodeURIComponent(q.push) });
  },
  onReady() { this.ctx = wx.createLivePusherContext(); },
  log(s) {
    const t = new Date();
    this.setData({ logs: [t.getMinutes() + ':' + ('0' + t.getSeconds()).slice(-2) + ' ' + s].concat(this.data.logs).slice(0, 14) });
    console.log('[pusher] ' + s);
  },
  onUrl(e) { this.setData({ url: e.detail.value }); },
  call(m, opt) {
    return new Promise((resolve) => {
      this.ctx[m](Object.assign({}, opt || {}, {
        success: (r) => { this.setData({ result: m + ' ✓' }); resolve(r || {}); },
        fail: (r) => { this.setData({ result: m + ' ✗ ' + r.errMsg }); this.log(m + ' ✗ ' + r.errMsg); resolve(null); }
      }));
    });
  },
  cmd(e) { this.call(e.currentTarget.dataset.m); },
  bgm() { return this.call('playBGM', { url: '/pages/pusher/bgm.m4a' }); },
  beautyOn() { this.setData({ beauty: this.data.beauty ? 0 : 9, whiteness: this.data.whiteness ? 0 : 5 }); },
  nextFilter() { this.setData({ filter: FILTERS[(FILTERS.indexOf(this.data.filter) + 1) % FILTERS.length] }); },
  pipGo() { this.setData({ pip: 'push' }); setTimeout(() => wx.navigateTo({ url: '/pages/basic/basic' }), 50); },
  onBgmStart() { this.ev.bgmStart = true; this.log('bgmstart'); },
  onBgmProgress(e) { this.ev.bgmProgress++; if (this.ev.bgmProgress < 3) this.log('bgmprogress ' + e.detail.progress + '/' + e.detail.duration); },
  onBgmComplete() { this.ev.bgmComplete = true; this.log('bgmcomplete'); },
  onPipEnter() { this.ev.pipEnter = true; this.log('enterpictureinpicture'); console.log('NC-MARK pip-enter'); },
  onPipLeave() { this.ev.pipLeave = true; this.log('leavepictureinpicture'); console.log('NC-MARK pip-leave'); },
  async snap() {
    const r = await this.call('snapshot');
    if (r) this.setData({ shot: r.tempImagePath, result: 'snapshot ✓ ' + r.width + '×' + r.height });
    return r;
  },
  onState(e) {
    const c = e.detail.code;
    this.codes.push(c);
    this.log('statechange ' + c + ' ' + (e.detail.message || ''));
    if (!this.auto || this.autoDone) return;
    if (c === 1007 && !this.startedPush) {
      this.startedPush = true;
      setTimeout(() => this.call('start'), 600);
    }
    if (c === 1002) setTimeout(() => this.runAuto(), 2500);
    if (c === -1307 || c === -1301) this.finish({ ok: false, code: c });
  },
  onNet(e) {
    const i = e.detail.info || {};
    this.setData({ net: i.videoBitrate + ' / ' + i.audioBitrate + ' kbps · ' + i.videoFPS + ' fps · ' + i.videoWidth + '×' + i.videoHeight });
    if (i.videoBitrate > 0) this.netOk = true;
  },
  onError(e) {
    this.log('error ' + JSON.stringify(e.detail));
    if (this.auto && e.detail.errCode === 10001) this.finish({ ok: false, error: e.detail });
  },
  async runAuto() {
    if (this.autoDone || this.autoRunning) return;
    this.autoRunning = true;
    // 美颜 + 滤镜 + 背景音推 2 秒，再全关推 2 秒（NC-MARK 给收端对时）
    this.setData({ beauty: 9, whiteness: 5, filter: 'nostalgia' });
    const bgm = await this.bgm();
    console.log('NC-MARK fx-on');
    await wait(2000);
    this.setData({ beauty: 0, whiteness: 0, filter: 'standard' });
    console.log('NC-MARK fx-off');
    await wait(2000);
    // 小窗：盖住这一页时缩成浮窗、推流不断，2.5 秒后返回
    this.setData({ pip: 'push' });
    await wait(100);
    wx.navigateTo({ url: '/pages/basic/basic' });
    await wait(2500);
    wx.navigateBack();
    for (let i = 0; i < 30 && !this.ev.pipLeave; i++) await wait(100);
    this.setData({ pip: '' });
    const shot = await this.snap();
    const zoom = await this.call('getMaxZoom');
    await new Promise((r) => setTimeout(r, 1500));
    const stop = await this.call('stop');
    const need = [1003, 1007, 1008, 1001, 1002, 1009];
    this.finish({
      ok: need.every((c) => this.codes.indexOf(c) >= 0) && this.netOk && !!(shot && shot.width > 0) && !!stop &&
        !!bgm && this.ev.bgmStart && this.ev.bgmProgress > 0 && this.ev.pipEnter && this.ev.pipLeave,
      ev: this.ev,
      codes: this.codes,
      net: this.data.net,
      snapshot: shot ? shot.width + 'x' + shot.height : null,
      maxZoom: zoom && zoom.maxZoom
    });
  },
  finish(r) {
    if (this.autoDone) return;
    this.autoDone = true;
    r.ms = Date.now() - this.started;
    console.log('NC-CHECK pusher ' + JSON.stringify(r));
    if (this.chain) console.log('NC-CHECK done');
  }
});
