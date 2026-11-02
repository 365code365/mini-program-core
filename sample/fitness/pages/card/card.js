const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
// 入场码点阵：每次刷新换一组（演示动态码），三个角留给定位框
function dots() {
  const out = [];
  for (let r = 0; r < 6; r++) {
    for (let c = 0; c < 6; c++) {
      const corner = (r < 2 && c < 2) || (r < 2 && c > 3) || (r > 3 && c < 2);
      if (!corner && Math.random() < 0.5) out.push({ k: r * 6 + c, x: 10 + c * 14, y: 10 + r * 14 });
    }
  }
  return out;
}
Page({
  data: {
    card: { name: '', level: 'LV.3 黑金会员', expire: '', leftDays: 0, usedPct: 0, valid: true },
    qrDots: [], codeLeft: 60,
    perks: [
      { ch: '课', name: '团课无限约', desc: '每日 2 节' },
      { ch: '店', name: '全国通店', desc: '86 家门店' },
      { ch: '浴', name: '淋浴毛巾', desc: '免费使用' },
      { ch: '测', name: '体测报告', desc: '每月 1 次' },
      { ch: '教', name: '私教体验', desc: '赠 2 节' },
      { ch: '停', name: '免费停卡', desc: '30 天/年' },
      { ch: '友', name: '带友入场', desc: '每月 2 次' },
      { ch: '饮', name: '能量吧', desc: '饮品 8 折' }
    ],
    plans: data.cardPlans, planIdx: 2, autoRenew: false
  },
  onShow() {
    this.load();
    this.setData({ autoRenew: !!wx.getStorageSync('fit_auto_renew') });
    this.refreshCode();
    clearInterval(this.timer);
    this.timer = setInterval(() => {
      const n = this.data.codeLeft - 1;
      if (n <= 0) this.refreshCode();
      else this.setData({ codeLeft: n });
    }, 1000);
  },
  onHide() { clearInterval(this.timer); },
  onUnload() { clearInterval(this.timer); },
  load() {
    const c = store.getCard();
    const left = store.cardLeftDays();
    this.setData({
      card: { name: c.plan, level: 'LV.3 黑金会员', expire: c.expire, leftDays: left, valid: store.cardValid(), usedPct: Math.max(4, Math.min(100, 100 - Math.round(left / 365 * 100))) }
    });
  },
  refreshCode() { this.setData({ qrDots: dots(), codeLeft: 60 }); },
  // 入场码：手动刷新（截图转发的码 60 秒后失效）
  showCode() {
    if (!this.data.card.valid) return wx.showToast({ title: '会员卡已过期，续费后才能入场', icon: 'none' });
    this.refreshCode();
    wx.showToast({ title: '入场码已刷新，60 秒内有效', icon: 'none' });
  },
  pickPlan(e) { this.setData({ planIdx: Number(e.currentTarget.dataset.index) }); },
  onAuto(e) {
    wx.setStorageSync('fit_auto_renew', e.detail.value);
    this.setData({ autoRenew: e.detail.value });
    wx.showToast({ title: e.detail.value ? '到期前 1 天自动续费，可随时关闭' : '已关闭自动续费', icon: 'none' });
  },
  pay() {
    if (this.paying) return;
    const plan = this.data.plans[this.data.planIdx];
    const from = this.data.card.valid ? '在 ' + this.data.card.expire + ' 基础上顺延 ' + plan.days + ' 天' : '从今天起 ' + plan.days + ' 天';
    this.paying = true;
    host.pay(plan.price, '燃动 ' + plan.name + '\n有效期' + from).then(ok => {
      this.paying = false;
      if (!ok) return wx.showToast({ title: '已取消支付', icon: 'none' });
      const o = store.buyCard(plan);
      this.load();
      wx.showToast({ title: '续费成功，有效期至 ' + o.newExpire, icon: 'none' });
    });
  },
  goOrders() { wx.navigateTo({ url: '/pages/orders/orders' }); }
});
