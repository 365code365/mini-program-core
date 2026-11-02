// 优惠券：可使用 / 已使用 / 已过期；兑换码（data.CODES，一码一次）；「去使用」回首页并记住这张券，
// 限车型的券顺带切到那个车型 —— 券只在付款时核销，没下单不会用掉
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const TABS = [{ key: 'ok', name: '可使用' }, { key: 'used', name: '已使用' }, { key: 'expired', name: '已过期' }];

Page({
  data: { tabs: TABS, tab: 'ok', list: [], code: '', exSeq: 0, counts: {}, pick: '' },
  onLoad(q) {
    if (q.tab && TABS.some(t => t.key === q.tab)) this.setData({ tab: q.tab });
  },
  onShow() {
    this.refresh();
  },
  refresh() {
    const all = store.allCoupons();
    const groups = { ok: all.filter(c => !c.used), used: all.filter(c => c.used), expired: data.EXPIRED };
    this.setData({
      list: groups[this.data.tab],
      counts: { ok: groups.ok.length, used: groups.used.length, expired: groups.expired.length },
      pick: store.getPick()
    });
  },
  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.key });
    this.refresh();
  },
  onCode(e) {
    this.setData({ code: e.detail.value });
  },
  onExchange() {
    const code = String(this.data.code || '').trim();
    if (!code) {
      wx.showToast({ title: '请输入兑换码', icon: 'none' });
      return;
    }
    const r = store.redeem(code);
    if (!r.ok) {
      wx.showToast({ title: r.msg, icon: 'none' });
      return;
    }
    // 兑换成功清空输入框（exSeq 换输入框 id，见 wxml 注释）
    this.setData({ code: '', exSeq: this.data.exSeq + 1, tab: 'ok' });
    this.refresh();
    wx.showToast({ title: '兑换成功：' + r.coupon.name + ' ¥' + r.coupon.amount, icon: 'none' });
  },
  onUse(e) {
    const id = String(e.currentTarget.dataset.id);
    const c = store.validCoupons().find(x => x.id === id);
    if (!c) return;
    store.setPick(c.id);
    if (c.types && c.types.length === 1) store.setType(c.types[0]);
    wx.showToast({ title: '叫车时优先使用「' + c.name + '」', icon: 'none' });
    wx.switchTab({ url: '/pages/index/index' });
  }
});
