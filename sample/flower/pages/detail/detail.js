const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: {
    p: null, stems: data.stems, packs: data.packs, slots: data.slots, half: false,
    stem: 11, pack: 'mist', date: '', minDate: '', maxDate: '', slotIdx: 2, card: '', qty: 1, unit: 0, total: 0, cartCount: 0, fav: false,
    cardTpls: ['生日快乐，岁岁平安！', '谢谢你一直在我身边。', '祝您早日康复。', '开业大吉，生意兴隆！'],
    reviews: [], rateCount: 0, starList: [1, 2, 3, 4, 5]
  },
  onLoad(q) {
    const p = data.productOf(q.id || 'f1') || data.products[0];
    const fx = store.fixSlot(store.dayAfter(1), data.slots[2]);
    this.setData({ p, half: p.id === data.HALF_ID, card: p.words, date: fx.date, slotIdx: data.slots.indexOf(fx.slot), minDate: store.dayAfter(0), maxDate: store.dayAfter(30),
      fav: (wx.getStorageSync('fl_fav') || []).indexOf(p.id) >= 0 });
    wx.setNavigationBarTitle({ title: p.name });
    this.calc();
  },
  onShow() {
    this.setData({ cartCount: store.cartCount() });
    this.loadReviews();
  },
  // 买家评价：自己评价过的订单（含这束花）排前面，后面是示例评价
  loadReviews() {
    const id = this.data.p.id;
    const mine = store.getOrders().filter(o => o.rating && o.items.some(x => x.id === id))
      .map(o => ({ user: '我', stars: o.rating.stars, text: o.rating.text || o.rating.tags.join('、') || '好评', time: o.rating.time.slice(0, 10), mine: true }));
    const all = mine.concat(data.reviewPool[this.data.p.use] || []).slice(0, 3)
      .map((r, i) => Object.assign({ k: 'r' + i, av: r.user.slice(0, 1) }, r));
    this.setData({ reviews: all, rateCount: Math.round(this.data.p.sold / 9) + mine.length });
  },
  calc() {
    const unit = data.unitPrice(this.data.p, this.data.stem, this.data.pack);
    this.setData({ unit, total: unit * this.data.qty });
  },
  onStem(e) { this.setData({ stem: Number(e.currentTarget.dataset.k) }); this.calc(); },
  onPack(e) { this.setData({ pack: e.currentTarget.dataset.k }); this.calc(); },
  // 当天的时段要来得及配送（同城 3 小时达），来不及就换成最早能送的
  onDate(e) {
    const fx = store.fixSlot(e.detail.value, data.slots[this.data.slotIdx]);
    this.setData({ date: fx.date, slotIdx: data.slots.indexOf(fx.slot) });
    if (fx.changed) wx.showToast({ title: '所选时段来不及配送，已改为 ' + fx.date.slice(5) + ' ' + fx.slot, icon: 'none' });
  },
  onSlot(e) {
    const i = Number(e.detail.value);
    if (!store.slotOpen(this.data.date, data.slots[i])) { wx.showToast({ title: '这个时段来不及配送了，请选更晚的时段', icon: 'none' }); return; }
    this.setData({ slotIdx: i });
  },
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
    return { id: d.p.id, stems: d.stem, pack: d.pack, date: d.date, slot: d.slots[d.slotIdx], card: d.card.trim(), qty: d.qty };
  },
  addCart() {
    store.addToCart(this.line());
    this.setData({ cartCount: store.cartCount() });
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },
  buyNow() {
    wx.setStorageSync('fl_checkout', [Object.assign({ key: 'now' }, this.line())]);
    wx.navigateTo({ url: '/pages/confirm/confirm?from=detail' });
  },
  goCart() { wx.switchTab({ url: '/pages/cart/cart' }); }
});
