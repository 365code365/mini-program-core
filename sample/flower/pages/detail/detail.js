const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: {
    p: null, stems: data.stems, packs: data.packs, slots: data.slots,
    stem: 11, pack: 'mist', date: '', minDate: '', maxDate: '', slotIdx: 2, card: '', qty: 1, unit: 0, total: 0, cartCount: 0, fav: false,
    cardTpls: ['生日快乐，岁岁平安！', '谢谢你一直在我身边。', '祝您早日康复。', '开业大吉，生意兴隆！']
  },
  onLoad(q) {
    const p = data.productOf(q.id || 'f1') || data.products[0];
    this.setData({ p, card: p.words, date: store.dayAfter(1), minDate: store.dayAfter(0), maxDate: store.dayAfter(30),
      fav: (wx.getStorageSync('fl_fav') || []).indexOf(p.id) >= 0 });
    this.calc();
  },
  onShow() { this.setData({ cartCount: store.cartCount() }); },
  calc() {
    const unit = data.unitPrice(this.data.p, this.data.stem, this.data.pack);
    this.setData({ unit, total: unit * this.data.qty });
  },
  onStem(e) { this.setData({ stem: Number(e.currentTarget.dataset.k) }); this.calc(); },
  onPack(e) { this.setData({ pack: e.currentTarget.dataset.k }); this.calc(); },
  onDate(e) { this.setData({ date: e.detail.value }); },
  onSlot(e) { this.setData({ slotIdx: Number(e.detail.value) }); },
  onCard(e) { this.setData({ card: e.detail.value }); },
  useTpl(e) { this.setData({ card: e.currentTarget.dataset.t }); },
  minus() { if (this.data.qty > 1) { this.setData({ qty: this.data.qty - 1 }); this.calc(); } },
  plus() { if (this.data.qty < 99) { this.setData({ qty: this.data.qty + 1 }); this.calc(); } },
  toggleFav() {
    let fav = wx.getStorageSync('fl_fav') || [];
    const id = this.data.p.id;
    fav = fav.indexOf(id) >= 0 ? fav.filter(x => x !== id) : [id].concat(fav);
    wx.setStorageSync('fl_fav', fav);
    this.setData({ fav: fav.indexOf(id) >= 0 });
    wx.showToast({ title: this.data.fav ? '已收藏' : '已取消收藏', icon: 'none' });
  },
  line() {
    const d = this.data;
    return { id: d.p.id, stems: d.stem, pack: d.pack, date: d.date, slot: d.slots[d.slotIdx], card: d.card, qty: d.qty };
  },
  addCart() {
    store.addToCart(this.line());
    this.setData({ cartCount: store.cartCount() });
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },
  buyNow() {
    wx.setStorageSync('fl_checkout', [this.line()]);
    wx.navigateTo({ url: '/pages/confirm/confirm?from=detail' });
  },
  goCart() { wx.switchTab({ url: '/pages/cart/cart' }); }
});
