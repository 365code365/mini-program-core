// 模拟收银台：wx.requestPayment 没有宿主（桌面引擎 / 未接支付的 App）时由投保页 redirect 过来。
// 用户确认才算支付成功（store.markPaid 出保单号、次日生效）；取消则保单留在「待支付」，可在保单页继续支付。
const store = require('../../utils/store.js');
const util = require('../../utils/util.js');
Page({
  data: {
    policy: null, premiumText: '0.00', way: 0, paying: false,
    ways: [
      { name: '零钱', sub: '余额 ¥12,860.00', color: '#f7b500' },
      { name: '招商银行储蓄卡（8848）', sub: '单笔限额 5 万元', color: '#d6252c' },
      { name: '建设银行信用卡（6201）', sub: '可用额度充足', color: '#0b4ea2' }
    ]
  },
  onLoad(q) {
    const policy = store.getPolicy(q.id);
    if (!policy) { wx.showToast({ title: '投保单不存在', icon: 'none' }); return; }
    this.setData({ policy, premiumText: util.money(policy.premium) });
  },
  pick(e) { this.setData({ way: Number(e.currentTarget.dataset.i) }); },
  pay() {
    const p = this.data.policy;
    if (!p || this.data.paying) return;
    this.setData({ paying: true });
    wx.showLoading({ title: '支付中', mask: true });
    setTimeout(() => {
      wx.hideLoading();
      store.markPaid(p.id, this.data.ways[this.data.way].name);
      wx.redirectTo({ url: '/pages/result/result?id=' + p.id });
    }, 600);
  },
  cancel() {
    const p = this.data.policy;
    wx.showModal({
      title: '放弃支付？', content: '投保单将保留 24 小时，可在「保单」页继续支付。', confirmText: '放弃', cancelText: '继续支付', confirmColor: '#f0524a',
      success: r => { if (r.confirm && p) wx.redirectTo({ url: '/pages/policy/policy?id=' + p.id }); }
    });
  }
});
