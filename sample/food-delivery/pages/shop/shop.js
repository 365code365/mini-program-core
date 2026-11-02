const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');
const host = require('../../utils/host.js');

// 右侧列表的固定行高（rpx），用来把 scrollTop 换算成左侧当前分类
const SEC_TITLE = 64;
const DISH_ROW = 196;
const REVIEWS = [
  { id: 'r1', name: '爱吃鱼的猫', short: '猫', color: '#ffb347', stars: 5, date: '10-21', text: '分量很足，送到还是烫的，骑手小哥很客气，下次还点。' },
  { id: 'r2', name: '望京打工人', short: '望', color: '#7cc6ff', stars: 5, date: '10-19', text: '中午 12 点下单 25 分钟就到了，包装很严实没有洒。' },
  { id: 'r3', name: 'Lily', short: 'L', color: '#ffa6c1', stars: 4, date: '10-16', text: '味道不错，就是稍微有点咸，备注了少盐下次试试。' }
];

Page({
  data: {
    shop: {},
    menu: [],
    tabs: ['点菜', '评价', '商家'],
    tabIdx: 0,
    activeCat: 0,
    intoView: '',
    bodyHeight: 500,
    counts: {},
    cart: { list: [], count: 0, amount: 0 },
    canSubmit: false,
    goText: '',
    showCart: false,
    faved: false,
    starList: [1, 2, 3, 4, 5],
    reviews: REVIEWS,
    // 规格面板
    sheet: null,
    picks: [],
    specQty: 1,
    specTotal: 0,
    specText: ''
  },

  onLoad(options) {
    const shop = data.findShop(options.id || 's1');
    wx.setNavigationBarTitle({ title: shop.name });
    const info = wx.getWindowInfo();
    const ratio = info.windowWidth / 750;
    // 头部 260 + tab 80（rpx）以下全给左右两栏
    const bodyHeight = Math.floor(info.windowHeight - (260 + 80) * ratio);
    // 每个分类在右栏里的起始位置（px）
    let top = 0;
    this.secTops = data.menus[shop.id].map(g => {
      const t = top;
      top += (SEC_TITLE + g.items.length * DISH_ROW) * ratio;
      return t;
    });
    this.setData({ shop, bodyHeight, faved: store.getFavs().indexOf(shop.id) >= 0 });
    this.refreshCart();
  },

  onShow() {
    if (!this.data.shop.id) return;
    this.refreshCart();
    // 我自己的评价排在最前面（订单详情里评过的）
    const mine = orders.getOrders().filter(o => o.shopId === this.data.shop.id && o.rating).map(o => ({
      id: o.id, name: '陈小饱', short: '陈', color: '#1677ff', stars: o.rating.stars, date: store.md(o.createdAt),
      text: o.rating.text || o.rating.tags.join('，') || '默认好评', mine: true
    }));
    this.setData({ reviews: mine.concat(REVIEWS) });
  },

  refreshCart() {
    const shop = this.data.shop;
    const s = store.summary(shop.id);
    const menu = data.menus[shop.id].map(g => ({
      name: g.name,
      items: g.items,
      count: g.items.reduce((n, d) => n + (s.counts[d.id] || 0), 0)
    }));
    const diff = store.round2(shop.minOrder - s.amount);
    const canSubmit = s.count > 0 && diff <= 0;
    let goText = shop.minOrder ? `¥${shop.minOrder} 起送` : '去结算';
    if (s.count && diff > 0) goText = `还差 ¥${diff}`;
    if (canSubmit) goText = '去结算';
    this.setData({ menu, counts: s.counts, cart: s, canSubmit, goText, showCart: this.data.showCart && s.count > 0 });
  },

  // 购物车里是别家的菜：先确认清空再加
  guard(fn) {
    const c = store.getCart();
    if (c.shopId && c.shopId !== this.data.shop.id && c.lines.length) {
      wx.showModal({
        title: '提示',
        content: `购物车里有「${data.findShop(c.shopId).name}」的商品，继续添加将清空原购物车`,
        confirmText: '继续添加',
        success: (res) => { if (res.confirm) fn(); }
      });
      return;
    }
    fn();
  },

  onPlus(e) {
    const id = e.currentTarget.dataset.id;
    const d = data.findDish(this.data.shop.id, id);
    if (d.spec) {
      this.openSpec(d);
      return;
    }
    this.guard(() => {
      store.changeDish(this.data.shop.id, id, 1);
      this.refreshCart();
    });
  },

  onMinus(e) {
    store.changeDish(this.data.shop.id, e.currentTarget.dataset.id, -1);
    this.refreshCart();
  },

  onLinePlus(e) {
    store.changeLine(this.data.shop.id, e.currentTarget.dataset.key, 1);
    this.refreshCart();
  },

  onLineMinus(e) {
    store.changeLine(this.data.shop.id, e.currentTarget.dataset.key, -1);
    this.refreshCart();
  },

  // ───────── 规格面板 ─────────
  openSpec(d) {
    const groups = (data.specs[d.spec] || []).map(g => ({
      name: g.name,
      opts: g.opts.map(o => ({ n: o.n, label: o.add ? `${o.n} +¥${o.add}` : o.n }))
    }));
    this.setData({
      sheet: { id: d.id, name: d.name, desc: d.desc, glyph: d.glyph, c1: d.c1, c2: d.c2, groups },
      picks: groups.map(() => 0),
      specQty: 1
    });
    this.calcSpec();
  },

  calcSpec() {
    const line = store.lineOf(this.data.shop.id, this.data.sheet.id, this.data.picks, this.data.specQty);
    this.setData({ specTotal: store.round2(line.unit * line.qty), specText: line.spec });
  },

  onSpecPick(e) {
    const gi = Number(e.currentTarget.dataset.g);
    const oi = Number(e.currentTarget.dataset.o);
    this.setData({ [`picks[${gi}]`]: oi });
    this.calcSpec();
  },

  onSpecMinus() {
    if (this.data.specQty <= 1) return;
    this.setData({ specQty: this.data.specQty - 1 });
    this.calcSpec();
  },

  onSpecPlus() {
    if (this.data.specQty >= 99) return;
    this.setData({ specQty: this.data.specQty + 1 });
    this.calcSpec();
  },

  onSpecAdd() {
    const shopId = this.data.shop.id;
    const line = store.lineOf(shopId, this.data.sheet.id, this.data.picks, this.data.specQty);
    this.guard(() => {
      store.addLine(shopId, line);
      this.setData({ sheet: null });
      this.refreshCart();
      wx.showToast({ title: '已加入购物车', icon: 'success' });
    });
  },

  onCloseSpec() {
    this.setData({ sheet: null });
  },

  noop() {},

  onCat(e) {
    const idx = Number(e.currentTarget.dataset.idx);
    this.lockUntil = Date.now() + 500; // 点左栏引起的滚动不反过来改左栏
    this.setData({ activeCat: idx, intoView: `sec-${idx}` });
  },

  onDishScroll(e) {
    if (this.lockUntil && Date.now() < this.lockUntil) return;
    const y = e.detail.scrollTop + 4;
    let idx = 0;
    for (let i = 0; i < this.secTops.length; i++) if (y >= this.secTops[i]) idx = i;
    if (idx !== this.data.activeCat) this.setData({ activeCat: idx });
  },

  onTab(e) {
    this.setData({ tabIdx: Number(e.currentTarget.dataset.idx) });
  },

  onToggleCart() {
    if (!this.data.cart.count) return;
    this.setData({ showCart: !this.data.showCart });
  },

  onClearCart() {
    wx.showModal({
      title: '清空购物车',
      content: '确定清空已选的全部商品吗？',
      success: (res) => {
        if (res.confirm) {
          store.clearCart();
          this.setData({ showCart: false });
          this.refreshCart();
        }
      }
    });
  },

  onFav() {
    const faved = store.toggleFav(this.data.shop.id);
    this.setData({ faved });
    wx.showToast({ title: faved ? '已收藏，可在「我的收藏」找到' : '已取消收藏', icon: 'none' });
  },

  onCall() {
    host.call(this.data.shop.phone, '联系商家');
  },

  onNav() {
    host.openMap(this.data.shop);
  },

  onCheckout() {
    if (!this.data.canSubmit) {
      if (this.data.cart.count) wx.showToast({ title: this.data.goText, icon: 'none' });
      return;
    }
    wx.navigateTo({ url: `/pages/checkout/checkout?shop=${this.data.shop.id}` });
  }
});
