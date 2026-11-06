// 相机：CameraContext（拍照、录像带 timeout、变焦）、前后置、闪光灯、onCameraFrame 实时帧
const app = getApp();
const FLASH = ['off', 'on', 'auto', 'torch'];
Page({
  data: { log: [], position: 'back', flash: 'off', maxZoom: '-', nextZoom: 2, frame: '-', framing: false, recording: false, video: '', photo: '', err: '' },
  onReady() { this.ctx = wx.createCameraContext(); },
  onUnload() { if (this.listener) this.listener.stop(); },
  onInit(e) {
    this.maxZoom = e.detail.maxZoom;
    this.setData({ maxZoom: String(e.detail.maxZoom) });
    app.note(this, `initdone maxZoom=${e.detail.maxZoom}（${typeof e.detail.maxZoom}）`);
  },
  onError(e) {
    this.setData({ err: '相机不可用：' + (e.detail.errMsg || JSON.stringify(e.detail)) });
    app.note(this, `binderror ${JSON.stringify(e.detail)}`);
  },
  onStop() { app.note(this, 'stop（相机被中断）'); },
  call(m, opt) {
    return new Promise((resolve) => {
      this.ctx[m](Object.assign({}, opt || {}, {
        success: (r) => resolve(r || {}),
        fail: (r) => { app.note(this, `${m} ✗ ${r.errMsg}`); resolve(null); }
      }));
    });
  },
  async takePhoto() {
    const r = await this.call('takePhoto', { quality: 'high' });
    if (r) { this.setData({ photo: r.tempImagePath }); app.note(this, `takePhoto ✓ ${r.tempImagePath}`); }
  },
  async startRec() {
    const r = await this.call('startRecord', {
      timeout: 10,
      timeoutCallback: (res) => { this.setData({ recording: false, video: res && res.tempVideoPath }); app.note(this, `录到 10 秒上限 ${res && res.tempVideoPath}`); }
    });
    if (r) { this.setData({ recording: true }); app.note(this, 'startRecord ✓'); }
  },
  async stopRec() {
    const r = await this.call('stopRecord');
    this.setData({ recording: false });
    if (r) { this.setData({ video: r.tempVideoPath }); app.note(this, `stopRecord ✓ ${r.tempVideoPath}（封面 ${r.tempThumbPath}）`); }
  },
  async zoom() {
    const r = await this.call('setZoom', { zoom: this.data.nextZoom });
    if (r) app.note(this, `setZoom → ${r.zoom}`);
    // 1 → 2 → 最大 → 1
    const max = Number(this.maxZoom) || 2;
    const next = this.data.nextZoom === 1 ? 2 : (this.data.nextZoom < max ? max : 1);
    this.setData({ nextZoom: next });
  },
  switchPos() { this.setData({ position: this.data.position === 'back' ? 'front' : 'back' }); },
  switchFlash() { this.setData({ flash: FLASH[(FLASH.indexOf(this.data.flash) + 1) % FLASH.length] }); },
  toggleFrames() {
    if (this.data.framing) {
      this.listener.stop();
      this.setData({ framing: false });
      return;
    }
    let n = 0;
    this.listener = this.ctx.onCameraFrame((f) => {
      // 每 10 帧更新一次显示：每帧 setData 白占逻辑线程
      if (++n % 10 === 1) this.setData({ frame: `${f.width}×${f.height} · ${f.data.byteLength} 字节 · 第 ${n} 帧` });
    });
    this.listener.start({ success: () => this.setData({ framing: true }), fail: (r) => app.note(this, `onCameraFrame ✗ ${r.errMsg}`) });
  },
  previewPhoto() { wx.previewImage({ urls: [this.data.photo] }); }
});
