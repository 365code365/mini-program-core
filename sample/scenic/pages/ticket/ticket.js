// 电子票：wx.createCanvasContext 画入园二维码；进页面调亮屏幕 + 常亮（方便闸机扫码），离开时恢复原亮度。
// 保存到相册：canvasToTempFilePath → saveImageToPhotosAlbum，失败（没授权 / 桌面没有相册）时打开大图让用户长按保存。
const store = require('../../utils/store.js');
const qr = require('../../utils/qr.js');
const SIZE = 200;

Page({
  data: { o: null, size: SIZE, bright: false, codeText: '', used: false },
  onLoad(q) {
    const o = store.order(q && q.no);
    if (!o) return;
    this.setData({ o, used: o.status !== 'paid', codeText: o.code.replace(/(.{4})/g, '$1 ').trim() });
  },
  onReady() { this.drawQr(); },
  onShow() { if (this.data.o && !this.data.used) this.brighten(); },
  onHide() { this.restore(); },
  onUnload() { this.restore(); },
  drawQr() {
    const o = this.data.o;
    if (!o) return;
    const ctx = wx.createCanvasContext('qr', this);
    qr.draw(ctx, o.code, SIZE, this.data.used ? '#b7c0bc' : '#1d2722');
    // 中间压一个品牌角标（二维码 H 级纠错允许遮住中心一小块）
    ctx.setFillStyle('#ffffff');
    ctx.fillRect(SIZE / 2 - 20, SIZE / 2 - 20, 40, 40);
    ctx.setFillStyle(this.data.used ? '#b7c0bc' : '#13a07b');
    ctx.fillRect(SIZE / 2 - 16, SIZE / 2 - 16, 32, 32);
    ctx.setFillStyle('#ffffff');
    ctx.setFontSize(18);
    ctx.setTextAlign('center');
    ctx.setTextBaseline('middle');
    ctx.fillText('山', SIZE / 2, SIZE / 2 + 1);
    ctx.draw();
  },
  brighten() {
    if (this._saved !== undefined) return;
    wx.getScreenBrightness({
      success: (r) => {
        this._saved = r.value;
        wx.setScreenBrightness({ value: 1, success: () => this.setData({ bright: true }) });
      },
      fail: () => { this._saved = null; }
    });
    wx.setKeepScreenOn({ keepScreenOn: true });
  },
  restore() {
    if (this._saved === undefined) return;
    if (typeof this._saved === 'number') wx.setScreenBrightness({ value: this._saved });
    wx.setKeepScreenOn({ keepScreenOn: false });
    this._saved = undefined;
    this.setData({ bright: false });
  },
  onToggleBright() { if (this.data.bright) this.restore(); else this.brighten(); },
  onSave() {
    wx.canvasToTempFilePath({
      canvasId: 'qr', width: SIZE, height: SIZE, destWidth: SIZE * 3, destHeight: SIZE * 3,
      success: (r) => {
        wx.saveImageToPhotosAlbum({
          filePath: r.tempFilePath,
          success: () => wx.showToast({ title: '已保存到相册', icon: 'success' }),
          fail: (e) => {
            console.log('saveImageToPhotosAlbum fail:', e && e.errMsg);
            wx.showModal({
              title: '没能存进相册', content: '可能没有相册权限。可以打开大图后长按保存，或入园时直接刷身份证。', confirmText: '打开大图',
              success: (m) => { if (m.confirm) wx.previewImage({ urls: [r.tempFilePath], current: r.tempFilePath }); }
            });
          }
        });
      },
      fail: () => wx.showToast({ title: '生成图片失败，请截图保存', icon: 'none' })
    }, this);
  },
  onCopy() { wx.setClipboardData({ data: this.data.o.code }); },
  goOrder() { wx.navigateTo({ url: '/pages/order-detail/order-detail?no=' + this.data.o.no }); }
});
