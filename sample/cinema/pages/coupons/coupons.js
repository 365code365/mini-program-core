// 优惠券：可用 / 已使用，兑换码领券；下单时在确认页按票价门槛自动挑
const store = require('../../utils/store.js');
Page({
  data: { tab: 0, usable: [], used: [], code: '', formSeq: 0 },
  onShow() { this.load(); },
  load() { this.setData(store.couponLists()); },
  onTab(e) { this.setData({ tab: Number(e.currentTarget.dataset.i) }); },
  onCode(e) { this.setData({ code: e.detail.value }); },
  onRedeem() {
    if (!this.data.code.trim()) { wx.showToast({ title: '请输入兑换码', icon: 'none' }); return; }
    const r = store.redeem(this.data.code);
    wx.showToast({ title: r.msg, icon: 'none' });
    if (!r.ok) return;
    this.setData({ code: '', formSeq: this.data.formSeq + 1, tab: 0 });
    this.load();
  },
  onUse() { wx.switchTab({ url: '/pages/index/index' }); }
});
