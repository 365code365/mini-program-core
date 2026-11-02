// 电子保单：待支付可继续支付 / 取消；生效后可申请理赔，列出这张保单的理赔记录
const store = require('../../utils/store.js');
const util = require('../../utils/util.js');
const products = require('../../utils/products.js');
Page({
  data: { p: null, st: {}, premiumText: '', coverage: [], claims: [], idMasked: '', phoneMasked: '' },
  onLoad(q) { this.id = q.id; },
  onShow() {
    const p = store.getPolicy(this.id);
    if (!p) { this.setData({ p: null }); return; }
    const prod = products.getProduct(p.productId);
    this.setData({
      p, st: store.statusOf(p), premiumText: util.money(p.premium),
      coverage: prod ? prod.coverage : [], idMasked: util.maskId(p.holder.idcard), phoneMasked: util.maskPhone(p.holder.phone),
      claims: store.claims().filter(c => c.policyId === p.id).map(c => store.claimView(c))
    });
    wx.setNavigationBarTitle({ title: p.policyNo ? '电子保单' : '投保单' });
  },
  pay() { wx.navigateTo({ url: '/pages/cashier/cashier?id=' + this.data.p.id }); },
  cancel() {
    wx.showModal({
      title: '取消投保', content: '取消后这张投保单将作废，需要时可以重新投保。', confirmText: '取消投保', cancelText: '再想想', confirmColor: '#f0524a',
      success: r => { if (r.confirm) { store.updatePolicy(this.data.p.id, { status: 'cancelled' }); this.onShow(); } }
    });
  },
  claim() { wx.navigateTo({ url: '/pages/claim/claim?id=' + this.data.p.id }); },
  openClaim(e) { wx.navigateTo({ url: '/pages/claim/claim?claim=' + e.currentTarget.dataset.id }); },
  copy() { wx.setClipboardData({ data: this.data.p.policyNo || this.data.p.id }); },
  renew(e) {
    store.updatePolicy(this.data.p.id, { autoRenew: e.detail.value });
    wx.showToast({ title: e.detail.value ? '已开启到期自动续保' : '已关闭自动续保', icon: 'none' });
  },
  rebuy() { wx.navigateTo({ url: '/pages/detail/detail?id=' + this.data.p.productId }); }
});
