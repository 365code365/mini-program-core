// 扫码充电：先调系统扫码（wx.scanCode），宿主没有摄像头能力时退回「输入终端编号」
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const start = require('../../utils/start.js');
Page({
  data: { code: '', error: '', found: null, light: false, recent: [], balance: '0.00', car: null, focus: false, inputSeq: 0 },
  onLoad(q) {
    if (q.no) this.lookup(String(q.no).toUpperCase());
  },
  onShow() {
    if (store.getSession()) { wx.redirectTo({ url: '/pages/charging/charging' }); return; }
    this.setData({ balance: store.balance().toFixed(2), recent: store.recent(), car: store.defaultCar() });
    if (this.data.code) this.lookup(this.data.code);
  },
  lookup(code) {
    this.setData({ code, found: store.terminal(code), error: '' });
  },
  onInput(e) { this.lookup(e.detail.value.toUpperCase()); },
  onBlur() { this.setData({ focus: false }); },
  onClear() { this.setData({ code: '', found: null, error: '', inputSeq: this.data.inputSeq + 1 }); },
  onRecent(e) { this.setData({ inputSeq: this.data.inputSeq + 1 }); this.lookup(String(e.currentTarget.dataset.code)); },
  onLight() { this.setData({ light: !this.data.light }); },
  onScanTap() {
    host.scan().then(code => {
      if (code === '') return;
      if (code === null) {
        this.setData({ focus: true });
        wx.showToast({ title: '当前环境无法打开摄像头，请输入桩身编号', icon: 'none' });
        return;
      }
      this.setData({ inputSeq: this.data.inputSeq + 1 });
      this.lookup(code);
      wx.showToast({ title: this.data.found ? '已识别 ' + code : '无法识别该二维码', icon: 'none' });
    });
  },
  onCar() {
    const cars = store.getCars();
    if (!cars.length) { wx.navigateTo({ url: '/pages/vehicle-edit/vehicle-edit' }); return; }
    const shown = cars.slice(0, 5);
    wx.showActionSheet({
      itemList: shown.map(c => c.plate + ' ' + c.model).concat(['管理车辆']),
      success: r => {
        if (r.tapIndex === shown.length) { wx.navigateTo({ url: '/pages/vehicles/vehicles' }); return; }
        store.setDefaultCar(shown[r.tapIndex].id);
        this.setData({ car: store.defaultCar() });
      }
    });
  },
  onStart() {
    const f = this.data.found;
    if (!this.data.code) { this.setData({ error: '请输入终端编号' }); return; }
    if (!f) { this.setData({ error: '未找到该终端，请核对桩身铭牌上的编号' }); return; }
    if (!start.guard(f.terminal.no, err => this.setData({ error: err }))) return;
    start.begin(f.terminal.no, true);
  }
});
