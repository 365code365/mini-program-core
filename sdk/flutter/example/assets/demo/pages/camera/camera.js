// camera 演示：宿主自带的相机（Android Camera2 / iOS AVCaptureSession），画面同层渲染进页面。
// CameraContext：拍照、录像（timeout）、变焦；onCameraFrame（帧由引擎从画面里取，frame-size 决定尺寸）；扫码模式。
// ?auto=1：等 initdone → 收 3 帧 → setZoom(2) → takePhoto → 录 2.5 秒 → 打一行 `NC-CHECK camera …`；chain=1 时接着去推流页
Page({
  data: { mode: 'normal', position: 'back', flash: 'off', maxZoom: '-', frame: '-', photo: '', video: '', result: '', scan: '', logs: [] },
  onLoad(q) {
    this.auto = q && q.auto === '1';
    this.chain = q && q.chain === '1';
    this.started = Date.now();
    this.frames = 0;
  },
  onReady() {
    this.ctx = wx.createCameraContext();
  },
  log(s) {
    const t = new Date();
    this.setData({ logs: [t.getMinutes() + ':' + ('0' + t.getSeconds()).slice(-2) + ' ' + s].concat(this.data.logs).slice(0, 12) });
    console.log('[camera] ' + s);
  },
  call(m, opt) {
    return new Promise((resolve) => {
      this.ctx[m](Object.assign({}, opt || {}, {
        success: (r) => { this.setData({ result: m + ' ✓' }); resolve(r); },
        fail: (r) => { this.setData({ result: m + ' ✗ ' + r.errMsg }); this.log(m + ' ✗ ' + r.errMsg); resolve(null); }
      }));
    });
  },
  onInit(e) {
    this.setData({ maxZoom: String(e.detail.maxZoom) });
    this.log('initdone maxZoom=' + e.detail.maxZoom + '（' + typeof e.detail.maxZoom + '）');
    if (this.auto && !this.autoDone) {
      this.autoDone = true;
      setTimeout(() => this.runAuto(e.detail.maxZoom), 800);
    }
  },
  onError(e) {
    this.log('error ' + JSON.stringify(e.detail));
    if (this.auto && !this.autoDone) {
      this.autoDone = true;
      this.finish({ ok: false, error: e.detail.errMsg });
    }
  },
  onStop() { this.log('stop（摄像头被中断）'); },
  onScan(e) { this.setData({ scan: e.detail.result }); this.log('scancode ' + e.detail.type + ' ' + e.detail.result); },
  frames3() {
    return new Promise((resolve) => {
      let n = 0, first = null;
      const l = this.ctx.onCameraFrame((f) => {
        if (!first) first = [f.width, f.height, f.data.byteLength === f.width * f.height * 4, f.data instanceof ArrayBuffer];
        this.setData({ frame: f.width + '×' + f.height });
        if (++n === 3) { l.stop(); resolve(first); }
      });
      l.start({ fail: (r) => resolve(null) });
      setTimeout(() => resolve(first), 5000);
    });
  },
  takePhoto() { return this.call('takePhoto', { quality: 'normal' }).then((r) => { if (r) this.setData({ photo: r.tempImagePath }); return r; }); },
  startRec() { this.call('startRecord', { timeout: 10, timeoutCallback: (r) => { this.log('录到上限 ' + (r && r.tempVideoPath)); this.setData({ video: r && r.tempVideoPath }); } }); },
  stopRec() { return this.call('stopRecord').then((r) => { if (r) this.setData({ video: r.tempVideoPath }); return r; }); },
  zoom2() { this.call('setZoom', { zoom: 2 }).then((r) => r && this.log('zoom=' + r.zoom)); },
  switchPos() { this.setData({ position: this.data.position === 'back' ? 'front' : 'back' }); },
  switchFlash() { const f = ['off', 'on', 'auto', 'torch']; this.setData({ flash: f[(f.indexOf(this.data.flash) + 1) % f.length] }); },
  switchMode() { this.setData({ mode: this.data.mode === 'normal' ? 'scanCode' : 'normal' }); },
  size(path) {
    return new Promise((resolve) => wx.getFileSystemManager().getFileInfo({ filePath: path, success: (r) => resolve(r.size), fail: () => resolve(0) }));
  },
  async runAuto(maxZoom) {
    const frame = await this.frames3();
    const z = await this.call('setZoom', { zoom: 2 });
    const photo = await this.takePhoto();
    const info = photo ? await new Promise((resolve) => wx.getImageInfo({ src: photo.tempImagePath, success: resolve, fail: () => resolve(null) })) : null;
    const started = await this.call('startRecord', { timeout: 30 });
    await new Promise((r) => setTimeout(r, 2500));
    const rec = started ? await this.stopRec() : null;
    const videoSize = rec ? await this.size(rec.tempVideoPath) : 0;
    const thumbSize = rec && rec.tempThumbPath ? await this.size(rec.tempThumbPath) : 0;
    this.finish({
      ok: !!(typeof maxZoom === 'number' && frame && frame[2] && frame[3] && z && info && info.width > 0 && videoSize > 10000),
      maxZoom: maxZoom,
      frame: frame,
      zoom: z && z.zoom,
      photo: info ? info.width + 'x' + info.height : null,
      video: videoSize,
      thumb: thumbSize
    });
  },
  finish(r) {
    r.ms = Date.now() - this.started;
    console.log('NC-CHECK camera ' + JSON.stringify(r));
    if (this.chain) wx.redirectTo({ url: '/pages/pusher/pusher?auto=1&chain=1' });
  }
});
