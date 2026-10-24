const data = require('../../utils/data.js');

const DOTS = 48;
const BATTERY = 64;   // 演示车型电池容量（度）
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
  data: { no: '', stationName: '', soc: 0, dots: ringDots(0), kwhText: '0.00', feeText: '0.00', duration: '00:00', remain: 0, powerNow: '0.0', volt: 0, amp: 0, price: '' },
  onLoad(q) {
    let s = wx.getStorageSync('ev_session');
    if (!s || (q.no && s.no !== q.no)) {
      const hit = data.findTerminal(q.no || 'XX0128-02');
      s = { no: hit.terminal.no, station: hit.station.id, power: hit.terminal.power, elapsed: 0, startSoc: 32 };
      wx.setStorageSync('ev_session', s);
    }
    this.s = s;
    const p = data.currentPeriod();
    this.priceVal = p.elec + p.service;
    this.setData({ no: s.no, stationName: data.getStation(s.station).name, price: this.priceVal.toFixed(2) });
    this.tick();
  },
  // 计时按 setInterval 的节拍累加，而不是 Date.now() 相减：无头快照里 Date 被固定住，
  // 且累加值随会话存进 storage，离开页面再回来接着走
  onShow() {
    if (!this.timer && this.s) this.timer = setInterval(() => { this.s.elapsed = (this.s.elapsed || 0) + 1; wx.setStorageSync('ev_session', this.s); this.tick(); }, 1000);
  },
  onHide() { this.stopTimer(); },
  onUnload() { this.stopTimer(); },
  stopTimer() { if (this.timer) { clearInterval(this.timer); this.timer = null; } },
  tick() {
    const s = this.s;
    const sec = s.elapsed || 0;
    // 功率随电量升高逐渐降下来（恒流 → 恒压），和真实快充曲线一个走向
    const maxKwh = BATTERY * (100 - s.startSoc) / 100;
    let kwh = Math.min(maxKwh, sec * s.power * 0.9 / 3600 * SPEED);
    const soc = Math.min(100, s.startSoc + kwh / BATTERY * 100);
    const taper = soc > 80 ? 0.45 : 1;
    const wobble = 1 + Math.sin(sec * 1.7) * 0.02;
    const powerNow = soc >= 100 ? 0 : s.power * 0.9 * taper * wobble;
    const volt = s.power > 7 ? Math.round(388 + soc * 0.6) : 220;
    const remainKwh = BATTERY * (100 - soc) / 100;
    const remain = powerNow > 0 ? Math.ceil(remainKwh / powerNow * 60) : 0;
    const socInt = Math.floor(soc);
    this.kwh = kwh;
    this.setData({
      soc: socInt,
      dots: socInt === this.data.soc && this.data.kwhText !== '0.00' ? this.data.dots : ringDots(socInt),
      kwhText: kwh.toFixed(2),
      feeText: (kwh * this.priceVal).toFixed(2),
      duration: pad(Math.floor(sec / 60)) + ':' + pad(Math.floor(sec % 60)),
      remain,
      powerNow: powerNow.toFixed(1),
      volt,
      amp: powerNow > 0 ? Math.round(powerNow * 1000 / volt) : 0
    });
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
        const s = this.s;
        const fee = Number(this.data.feeText);
        const now = new Date();
        const order = {
          id: 'EV' + now.getFullYear() + pad(now.getMonth() + 1) + pad(now.getDate()) + pad(now.getHours()) + pad(now.getMinutes()) + pad(now.getSeconds()),
          station: s.station, terminal: s.no, kwh: Number(this.data.kwhText), fee,
          minutes: Math.max(1, Math.round((s.elapsed || 0) / 60)),
          date: pad(now.getMonth() + 1) + '-' + pad(now.getDate()) + ' ' + pad(now.getHours()) + ':' + pad(now.getMinutes()),
          status: 'done'
        };
        wx.setStorageSync('ev_orders', [order].concat(wx.getStorageSync('ev_orders') || []));
        data.setBalance(data.getBalance() - fee);
        wx.removeStorageSync('ev_session');
        wx.showToast({ title: '已结束，扣费 ¥' + fee.toFixed(2), icon: 'none' });
        setTimeout(() => wx.switchTab({ url: '/pages/orders/orders' }), 900);
      }
    });
  }
});
