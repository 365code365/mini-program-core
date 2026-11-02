// 购物车：勾选、步进器改数量、编辑模式批量删除、全选、合计；空购物车给常备药推荐。
// 结算时选中了处方药且没有有效处方 → 先去审方（rx 页），审核通过后直接进订单确认
const { BY_ID, card } = require('../../utils/drugs.js');
const S = require('../../utils/store.js');
const F = require('../../utils/fly.js');

const RECO = ['d1', 'd5', 'd15', 'd24'];

Page({
  data: { list: [], edit: false, allSel: false, selN: 0, total: '0.00', freeGap: '', hasRx: false, reco: RECO.map(id => card(BY_ID[id])) },
  onShow() { this.load(); },
  load() {
    const list = S.cartDetail();
    this.calc(list);
  },
  calc(list) {
    const sel = list.filter(c => c.sel);
    const total = sel.reduce((a, c) => a + c.price * c.qty, 0);
    const n = list.reduce((a, c) => a + c.qty, 0);
    this.setData({
      list, allSel: list.length > 0 && sel.length === list.length, selN: sel.reduce((a, c) => a + c.qty, 0),
      total: S.money(total), freeGap: total > 0 && total < 49 ? S.money(49 - total) : '', hasRx: sel.some(c => c.type === 'RX')
    });
    S.saveCart(list);
    F.syncTab(this, 2, n);
  },
  toggle(e) {
    const list = this.data.list;
    const it = list.find(c => c.key === e.currentTarget.dataset.key);
    it.sel = !it.sel;
    this.calc(list);
  },
  toggleAll() {
    const v = !this.data.allSel;
    this.calc(this.data.list.map(c => Object.assign(c, { sel: v })));
  },
  onQty(e) {
    const list = this.data.list;
    list.find(c => c.key === e.currentTarget.dataset.key).qty = e.detail.value;
    this.calc(list);
  },
  onLimit(e) {
    if (e.detail.type === 'max') { wx.showToast({ title: '单品限购 99 件', icon: 'none' }); return; }
    const key = e.currentTarget.dataset.key;
    const it = this.data.list.find(c => c.key === key);
    wx.showModal({
      title: '移出购物车', content: '确定不要「' + it.name + '」了吗？', confirmText: '移出', confirmColor: '#f04a3e',
      success: r => { if (r.confirm) this.calc(this.data.list.filter(c => c.key !== key)); }
    });
  },
  switchEdit() { this.setData({ edit: !this.data.edit }); },
  removeSel() {
    const n = this.data.list.filter(c => c.sel).length;
    if (!n) { wx.showToast({ title: '请先勾选要删除的商品', icon: 'none' }); return; }
    wx.showModal({
      title: '删除商品', content: '确定删除选中的 ' + n + ' 种商品吗？', confirmText: '删除', confirmColor: '#f04a3e',
      success: r => {
        if (!r.confirm) return;
        this.calc(this.data.list.filter(c => !c.sel));
        if (!this.data.list.length) this.setData({ edit: false });
      }
    });
  },
  checkout() {
    const sel = this.data.list.filter(c => c.sel);
    if (!sel.length) { wx.showToast({ title: '请勾选要结算的商品', icon: 'none' }); return; }
    const pending = S.rxPending(sel.map(c => c.id));
    if (pending.length) {
      wx.showModal({
        title: '处方药需审方', content: '选中的商品含处方药（' + pending.map(id => BY_ID[id].name).join('、') + '），需上传处方或在线问诊，药师审核通过后才能下单。',
        confirmText: '去问诊', cancelText: '暂不', confirmColor: '#12a37f',
        success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/rx/rx?from=cart&ids=' + pending.join(',') }); }
      });
      return;
    }
    wx.navigateTo({ url: '/pages/confirm/confirm?from=cart' });
  },
  openDrug(e) { wx.navigateTo({ url: '/pages/drug/drug?id=' + e.currentTarget.dataset.id }); },
  addReco(e) {
    S.addCart(e.currentTarget.dataset.id, 0, 1);
    this.load();
    wx.showToast({ title: '已加入购物车', icon: 'none' });
  },
  goShop() { wx.switchTab({ url: '/pages/index/index' }); }
});
