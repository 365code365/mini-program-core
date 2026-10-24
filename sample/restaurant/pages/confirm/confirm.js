const S = require('../../utils/store.js');

const TAGS = ['少放辣', '不要香菜', '不要葱', '米饭少一点', '汤分开装', '尽快出餐'];
const MAX_REMARK = 50;
const PACK_FEE = 100; // 打包每份 1 元

Page({
  data: {
    table: '',
    items: [],
    dine: '堂食',
    remark: '',
    remarkLen: 0,
    maxRemark: MAX_REMARK,
    tags: [],
    tableware: 1,
    sum: {},
    packText: '0',
    payText: '0',
    submitting: false
  },
  onLoad() {
    const cart = S.getCart();
    this.cart = cart;
    this.setData({
      table: S.getTable(),
      items: cart.map(i => Object.assign({}, i, { lineText: S.yuan(i.unit * i.qty) })),
      tableware: Math.max(1, Math.min(4, Math.ceil(S.summary(cart).count / 2)))
    });
    this.calc();
    this.syncTags('');
  },
  calc() {
    const sum = S.summary(this.cart);
    const pack = this.data.dine === '打包' ? PACK_FEE * sum.count : 0;
    this.pay = sum.pay + pack;
    this.pack = pack;
    this.setData({ sum, packText: S.yuan(pack), payText: S.yuan(this.pay) });
  },
  onDine(e) { this.setData({ dine: e.currentTarget.dataset.v }); this.calc(); },
  syncTags(remark) {
    this.setData({ tags: TAGS.map(t => ({ t, on: remark.indexOf(t) >= 0 })) });
  },
  // 受控输入：超过 50 字截断，回写 value
  onRemark(e) {
    const v = (e.detail.value || '').slice(0, MAX_REMARK);
    this.setData({ remark: v, remarkLen: v.length });
    this.syncTags(v);
    return v;
  },
  // 快捷标签：点一下追加到备注，再点一下从备注里去掉
  onTag(e) {
    const t = e.currentTarget.dataset.t;
    let parts = this.data.remark ? this.data.remark.split('，').filter(Boolean) : [];
    if (parts.indexOf(t) >= 0) parts = parts.filter(p => p !== t);
    else parts.push(t);
    let v = parts.join('，');
    if (v.length > MAX_REMARK) { wx.showToast({ title: '备注最多 50 字', icon: 'none' }); return; }
    this.setData({ remark: v, remarkLen: v.length });
    this.syncTags(v);
  },
  clearRemark() { this.setData({ remark: '', remarkLen: 0 }); this.syncTags(''); },
  wareMinus() { if (this.data.tableware > 0) this.setData({ tableware: this.data.tableware - 1 }); },
  warePlus() {
    if (this.data.tableware >= 10) { wx.showToast({ title: '最多 10 份餐具', icon: 'none' }); return; }
    this.setData({ tableware: this.data.tableware + 1 });
  },
  submit() {
    if (this.data.submitting || !this.cart.length) return;
    this.setData({ submitting: true });
    wx.showLoading({ title: '提交中' });
    setTimeout(() => {
      wx.hideLoading();
      const sum = this.data.sum;
      const o = S.createOrder({
        table: this.data.table, items: this.cart, amount: sum.amount, discount: sum.discount,
        pack: this.pack, pay: this.pay, remark: this.data.remark, tableware: this.data.tableware, dine: this.data.dine
      });
      S.saveCart([]);
      wx.redirectTo({ url: '/pages/pay/pay?id=' + o.id });
    }, 500);
  }
});
