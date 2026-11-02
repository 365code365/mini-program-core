// 药品详情：规格、步进器（components/stepper，triggerEvent change）、说明书分段折叠、处方药标识。
// 处方药「立即购买」先查有没有 7 天内审核通过的处方，没有就先去上传处方 / 问诊
const { BY_ID, DRUGS, card } = require('../../utils/drugs.js');
const S = require('../../utils/store.js');
const F = require('../../utils/fly.js');

function sections(d) {
  if (d.device) {
    return [
      { k: 'ind', t: '产品用途', v: d.ind }, { k: 'usage', t: '使用方法', v: d.usage }, { k: 'comp', t: '包装清单', v: d.comp },
      { k: 'note', t: '注意事项', v: '本品为医疗器械，请仔细阅读产品说明书或在医务人员指导下购买和使用。' }
    ];
  }
  return [
    { k: 'ind', t: '适应症 / 功能主治', v: d.ind }, { k: 'usage', t: '用法用量', v: d.usage }, { k: 'comp', t: '主要成分', v: d.comp },
    { k: 'adverse', t: '不良反应', v: d.adverse || '尚不明确。' },
    { k: 'taboo', t: '禁忌', v: d.taboo || '对本品过敏者禁用，过敏体质者慎用。' },
    { k: 'note', t: '注意事项', v: d.note || '请仔细阅读说明书并按说明使用，或在药师指导下购买和使用。' }
  ];
}

Page({
  data: { d: null, si: 0, qty: 1, price: '0.00', secs: [], open: { ind: true }, allOpen: false, cartN: 0, related: [], rxOk: false, ball: { on: false }, ballX: {}, ballY: {} },
  onLoad(q) {
    const d = card(BY_ID[q.id] || BY_ID.d1);
    wx.setNavigationBarTitle({ title: d.brand + ' ' + d.name });
    this.setData({
      d, secs: sections(d), price: S.money(d.specs[0].p),
      related: DRUGS.filter(x => x.dept === d.dept && x.id !== d.id).slice(0, 3).map(card)
    });
  },
  onShow() {
    this.setData({ cartN: S.cartCount(), rxOk: this.data.d.type === 'RX' && !S.rxPending([this.data.d.id]).length });
  },
  onSpec(e) {
    const si = +e.currentTarget.dataset.i;
    this.setData({ si, price: S.money(this.data.d.specs[si].p) });
  },
  onQty(e) { this.setData({ qty: e.detail.value }); },
  onLimit(e) { wx.showToast({ title: e.detail.type === 'min' ? '最少购买 1 件' : '单次限购 99 件', icon: 'none' }); },
  toggle(e) {
    const k = e.currentTarget.dataset.k;
    this.setData({ ['open.' + k]: !this.data.open[k] });
  },
  toggleAll() {
    const all = !this.data.allOpen, open = {};
    this.data.secs.forEach(s => { open[s.k] = all; });
    this.setData({ allOpen: all, open });
  },
  addCart(e) {
    const { d, si, qty } = this.data;
    const n = S.addCart(d.id, si, qty);
    this.createSelectorQuery().select('#cartIcon').boundingClientRect().exec(res => {
      const r = res[0] || { left: 40, top: 620, width: 40, height: 40 };
      F.fly(this, e, { x: r.left + r.width / 2 - 9, y: r.top + 4 }, () => {
        this.setData({ cartN: n });
        this.animate('#cartIcon', [{ scale: [1, 1] }, { scale: [1.3, 1.3], offset: 0.4 }, { scale: [1, 1] }], 320, () => this.clearAnimation('#cartIcon'));
      });
    });
    wx.showToast({ title: d.type === 'RX' ? '已加入，结算前需审方' : '已加入购物车', icon: 'none' });
  },
  buyNow() {
    const { d, si, qty } = this.data;
    getApp().globalData.buyNow = { id: d.id, si, qty };
    if (d.type === 'RX' && S.rxPending([d.id]).length) {
      wx.showModal({
        title: '处方药需审方', content: '「' + d.name + '」为处方药，请上传医生开具的处方或在线问诊，药师审核通过后发货。',
        confirmText: '去问诊', cancelText: '再看看', confirmColor: '#12a37f',
        success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/rx/rx?from=buy&ids=' + d.id }); }
      });
      return;
    }
    wx.navigateTo({ url: '/pages/confirm/confirm?from=buy' });
  },
  goCart() { wx.switchTab({ url: '/pages/cart/cart' }); },
  openDrug(e) { wx.redirectTo({ url: '/pages/drug/drug?id=' + e.currentTarget.dataset.id }); },
  call() {
    wx.makePhoneCall({
      phoneNumber: '0571-87001234',
      fail: () => wx.showModal({ title: '药师咨询', content: '执业药师热线 0571-87001234（每天 8:00-22:00）', showCancel: false, confirmText: '知道了' })
    });
  },
  onShareAppMessage() { return { title: this.data.d.brand + ' ' + this.data.d.name, path: '/pages/drug/drug?id=' + this.data.d.id }; }
});
