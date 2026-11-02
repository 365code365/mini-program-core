// 充电券：可用 / 已使用，兑换码领券。结算时自动选最省的一张（store.bestCoupon）
const store = require('../../utils/store.js');
function view(c) {
  return Object.assign({}, c, {
    big: c.kind === 'service' ? String(Math.round(c.rate * 100) / 10) : String(c.amount),
    unit: c.kind === 'service' ? '折' : '元',
    cond: c.kind === 'service' ? '仅抵服务费' : (c.min ? '满 ' + c.min + ' 元可用' : '无门槛')
  });
}
Page({
  data: { tab: 0, list: [], validCount: 0, usedCount: 0, code: '', codeSeq: 0 },
  onShow() { this.load(); },
  load() {
    const all = store.getCoupons();
    const valid = all.filter(c => c.status === 'valid');
    const used = all.filter(c => c.status !== 'valid');
    this.setData({ list: (this.data.tab === 0 ? valid : used).map(view), validCount: valid.length, usedCount: used.length });
  },
  onTab(e) { this.setData({ tab: Number(e.currentTarget.dataset.i) }); this.load(); },
  onCode(e) { this.setData({ code: e.detail.value }); },
  onRedeem() {
    if (!this.data.code.trim()) { wx.showToast({ title: '请输入兑换码', icon: 'none' }); return; }
    const r = store.redeem(this.data.code);
    if (r.err) { wx.showToast({ title: r.err, icon: 'none' }); return; }
    this.setData({ code: '', codeSeq: this.data.codeSeq + 1, tab: 0 });
    this.load();
    wx.showToast({ title: '兑换成功：' + r.coupon.title, icon: 'none' });
  },
  onUse() { wx.switchTab({ url: '/pages/index/index' }); }
});
