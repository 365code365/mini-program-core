// 二维码与条形码：<camera mode="scanCode"> 在同层画面上连续扫（bindscancode），wx.scanCode 调宿主的扫码界面，
// 下面几张测试码给另一台手机扫（或打印出来贴着扫）
const app = getApp();
const CODES = [
  { name: '二维码 · 网址', value: 'https://minirender.dev/lab?from=qr', src: '/images/codes/qr.png' },
  { name: '二维码 · 中文', value: '组件实验室·扫码测试', src: '/images/codes/qr-zh.png' },
  { name: 'EAN-13（商品条码）', value: '6901234567892', src: '/images/codes/ean13.png', wide: true },
  { name: 'Code 128', value: 'MINI-LAB-2026', src: '/images/codes/code128.png', wide: true }
];
Page({
  onUnload() { clearTimeout(this.hitTimer); },
  data: { log: [], flash: 'off', hit: false, last: null, count: 0, camErr: '', api: '', codes: CODES },
  onInit(e) { app.note(this, `initdone maxZoom=${e.detail.maxZoom}`); },
  onStop() { app.note(this, 'stop（相机被中断）'); },
  onCamErr(e) {
    // 没有相机（模拟器、桌面）或没授权：照实显示原因
    this.setData({ camErr: '相机不可用：' + (e.detail.errMsg || JSON.stringify(e.detail)) });
    app.note(this, `binderror ${JSON.stringify(e.detail)}`);
  },
  onScan(e) {
    const d = e.detail || {};
    // 扫到时扫描线停 1.2 秒再接着扫（相机一直在扫，下一个码照样会报）
    clearTimeout(this.hitTimer);
    this.hitTimer = setTimeout(() => this.setData({ hit: false }), 1200);
    this.setData({ hit: true, last: { type: d.type, result: d.result, charSet: d.charSet || '-' }, count: this.data.count + 1 });
    app.note(this, `scancode ${d.type} ${d.result}`);
    wx.vibrateShort({ type: 'light' });
  },
  toggleFlash() { this.setData({ flash: this.data.flash === 'off' ? 'torch' : 'off' }); },
  clear() { this.setData({ last: null, count: 0 }); },
  scan(scanType) {
    wx.scanCode({
      onlyFromCamera: true,
      scanType,
      success: (r) => {
        this.setData({ api: `${r.scanType} · ${r.result}${r.path ? ' · path ' + r.path : ''}` });
        app.note(this, `scanCode ok ${r.scanType} ${r.result}`);
      },
      fail: (err) => {
        this.setData({ api: err.errMsg });
        app.note(this, `scanCode ${err.errMsg}`);
      }
    });
  },
  scanAny() { this.scan(['qrCode', 'barCode', 'datamatrix', 'pdf417']); },
  scanQr() { this.scan(['qrCode']); },
  scanBar() { this.scan(['barCode']); },
  preview(e) {
    const src = e.currentTarget.dataset.src;
    wx.previewImage({ current: src, urls: CODES.map((c) => c.src) });
  }
});
