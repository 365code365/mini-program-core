const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const DOTS = 48;
const SPEED = 8;      // 演示加速倍数：真机上几秒就能看到电量与费用在走
function ringDots(soc) {
  const out = [];
  const R = 206, C = 220, half = 7;
  for (let i = 0; i < DOTS; i++) {
    const a = (i / DOTS) * Math.PI * 2;
    out.push({ i, x: Math.round(C + R * Math.sin(a) - half), y: Math.round(C - R * Math.cos(a) - half), on: i < Math.round(soc / 100 * DOTS) });
  }
  return out;
}
function pad(n) { return n < 10 ? '0' + n : '' + n; }
Page({
  data: { none: false, no: '', plate: '', stationName: '', soc: 0, dots: ringDots(0), kwhText: '0.00', feeText: '0.00', elecText: '0.00', serviceText: '0.00', duration: '00:00', remain: 0, powerNow: '0.0', volt: 0, amp: 0, price: '' },
  onLoad() {
    const s = store.getSession();
    if (!s) { this.setData({ none: true }); return; }
    this.s = s;
    const p = data.currentPeriod();
    this.p = p;
    this.priceVal = p.elec + p.service;
    this.setData({ no: s.no, plate: s.plate, stationName: data.getStation(s.station).name, price: this.priceVal.toFixed(2) });
    this.tick();
  },
  // 计时按 setInterval 的节拍累加，而不是 Date.now() 相减：无头快照里 Date 被固定住，
  // 且累加值随会话存进 storage，离开页面再回来接着走
  onShow() {
    if (!this.timer && this.s && !this.done) {
      this.timer = setInterval(() => {
        this.s.elapsed = (this.s.elapsed || 0) + 1;
        this.tick();
        this.s.soc = this.data.soc;
        store.saveSession(this.s);
        if (this.soc >= 100) this.finish('已充满，自动结束');
      }, 1000);
    }
  },
  onHide() { this.stopTimer(); },
  onUnload() { this.stopTimer(); },
  stopTimer() { if (this.timer) { clearInterval(this.timer); this.timer = null; } },
  tick() {
    const s = this.s;
    const sec = s.elapsed || 0;
    const battery = s.battery;
    // 功率随电量升高逐渐降下来（恒流 → 恒压），和真实快充曲线一个走向
    const maxKwh = battery * (100 - s.startSoc) / 100;
    const kwh = Math.min(maxKwh, sec * s.power * 0.9 / 3600 * SPEED);
    const soc = Math.min(100, s.startSoc + kwh / battery * 100);
    const taper = soc > 80 ? 0.45 : 1;
    const wobble = 1 + Math.sin(sec * 1.7) * 0.02;
    const powerNow = soc >= 100 ? 0 : s.power * 0.9 * taper * wobble;
    const volt = s.power > 7 ? Math.round(388 + soc * 0.6) : 220;
    const remainKwh = battery * (100 - soc) / 100;
    const remain = powerNow > 0 ? Math.ceil(remainKwh / powerNow * 60) : 0;
    const socInt = Math.floor(soc);
    this.kwh = kwh;
    this.soc = soc;
    // 与结算同一个算法（store.priced：度数取两位后分项取整），确认框里的费用就是扣款额
    const k2 = store.round2(kwh);
    const elec = store.round2(k2 * this.p.elec);
    const service = store.round2(k2 * this.p.service);
    this.setData({
      soc: socInt,
      dots: socInt === this.data.soc && this.data.kwhText !== '0.00' ? this.data.dots : ringDots(socInt),
      kwhText: kwh.toFixed(2),
      elecText: elec.toFixed(2),
      serviceText: service.toFixed(2),
      feeText: store.round2(elec + service).toFixed(2),
      duration: pad(Math.floor(sec / 60)) + ':' + pad(Math.floor(sec % 60)),
      remain,
      powerNow: powerNow.toFixed(1),
      volt,
      amp: powerNow > 0 ? Math.round(powerNow * 1000 / volt) : 0
    });
  },
  // 结算：券自动抵扣，余额扣款；余额不够就生成待支付订单，进详情页去付
  finish(tip) {
    this.stopTimer();
    this.done = true;
    const o = store.finishSession(this.kwh || 0, this.soc || this.s.startSoc);
    if (!o) return;
    wx.showToast({ title: o.status === 'paid' ? tip + '，已扣费 ¥' + o.pay.toFixed(2) : tip + '，待支付 ¥' + o.pay.toFixed(2), icon: 'none' });
    setTimeout(() => wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + o.id }), 800);
  },
  onStop() {
    // 弹窗期间先停表，确认框里的度数和费用就是最终结算值
    this.stopTimer();
    wx.showModal({
      title: '结束充电',
      content: '本次已充 ' + this.data.kwhText + ' 度，费用 ¥' + this.data.feeText + '，确定结束吗？',
      confirmText: '结束',
      confirmColor: '#F5455C',
      success: (r) => {
        if (!r.confirm) { this.onShow(); return; }
        this.finish('已结束充电');
      }
    });
  },
  onScan() { wx.redirectTo({ url: '/pages/scan/scan' }); }
});
