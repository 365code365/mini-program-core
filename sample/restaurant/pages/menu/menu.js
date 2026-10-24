const S = require('../../utils/store.js');
const { CATS, DISHES, SIZES, SPICY, EXTRAS } = require('../../utils/data.js');

// 右栏高度是写死的 rpx（见 menu.wxss），这样可以直接算出每个分类的起始位置，不依赖节点查询
const HEAD_RPX = 72;
const DISH_RPX = 200;

Page({
  data: {
    table: '',
    cats: CATS,
    sections: [],
    activeCat: 0,
    intoView: '',
    bodyHeight: 480,
    cart: [],
    sum: {},
    counts: {},
    showCart: false,
    showClear: false,
    // 规格弹层
    spec: null,
    sizes: SIZES.map(z => ({ id: z.id, delta: z.delta, label: z.name + (z.delta > 0 ? ' +' + z.delta / 100 : z.delta < 0 ? ' -' + (-z.delta / 100) : '') })),
    spicyList: SPICY,
    extras: EXTRAS.map(x => ({ id: x.id, label: x.name + ' +' + x.price / 100 })),
    sizeId: 'm',
    spicy: '微辣',
    extrasOn: {},
    specQty: 1,
    specPrice: '0',
    specDesc: ''
  },
  onLoad() {
    const info = wx.getSystemInfoSync();
    const r = info.windowWidth / 750;
    // 顶部桌号条 96 + 底部购物车 166 + 自定义 tabBar 100（rpx）
    const bodyHeight = Math.floor(info.windowHeight - (96 + 166 + 100) * r);
    const sections = CATS.map(c => ({ id: c.id, name: c.name, dishes: DISHES.filter(d => d.cat === c.id).map(d => Object.assign({ priceText: S.yuan(d.price) }, d)) }));
    let top = 0;
    this.secTops = sections.map(s => { const t = top; top += (HEAD_RPX + DISH_RPX * s.dishes.length) * r; return t; });
    this.setData({ sections, bodyHeight });
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    this.setData({ table: S.getTable() });
    this.refreshCart(S.getCart());
  },
  refreshCart(cart) {
    const counts = {};
    cart.forEach(i => { counts[i.id] = (counts[i.id] || 0) + i.qty; });
    const list = cart.map(i => Object.assign({}, i, { lineText: S.yuan(i.unit * i.qty) }));
    const sum = S.summary(cart);
    const patch = { cart: list, counts, sum };
    if (!cart.length) { patch.showCart = false; patch.showClear = false; }
    this.setData(patch);
  },
  // 左侧点分类 → 右侧 scroll-into-view 定位；定位引起的滚动回调 400ms 内不反向改高亮
  onCat(e) {
    const idx = Number(e.currentTarget.dataset.i);
    this.lockUntil = Date.now() + 400;
    this.setData({ activeCat: idx, intoView: 'sec-' + CATS[idx].id });
  },
  // 右侧滚动 → 高亮左侧分类
  onDishScroll(e) {
    if (this.lockUntil && Date.now() < this.lockUntil) return;
    const y = e.detail.scrollTop + 8;
    let idx = 0;
    for (let i = 0; i < this.secTops.length; i++) if (y >= this.secTops[i]) idx = i;
    if (idx !== this.data.activeCat) this.setData({ activeCat: idx });
  },
  plus(e) {
    const d = S.dishById(e.currentTarget.dataset.id);
    if (d.spec) return this.openSpec(d);
    this.refreshCart(S.addToCart(S.plainItem(d)));
  },
  minus(e) {
    const d = S.dishById(e.currentTarget.dataset.id);
    if (d.spec) {
      wx.showToast({ title: '多规格菜品请在购物车里减', icon: 'none' });
      return;
    }
    this.refreshCart(S.changeQty(d.id, -1));
  },
  openSpec(d) {
    this.setData({ spec: { id: d.id, name: d.name, desc: d.desc, bg: d.bg, ch: d.ch }, sizeId: 'm', spicy: '微辣', extrasOn: {}, specQty: 1 });
    this.calcSpec();
  },
  openSpecTap(e) { this.openSpec(S.dishById(e.currentTarget.dataset.id)); },
  // 引擎问题：--touch 点面板里的选项、处理函数里 setData 之后，外层遮罩会再收到一次 tap（--click 不会）。
  // 面板 catchtap 时记下时间，紧跟着到来的遮罩 tap 忽略掉；在微信里这段判断永远不会命中。
  closeSpec() { if (this.innerTapAt && Date.now() - this.innerTapAt < 400) return; this.setData({ spec: null }); },
  onSize(e) { this.setData({ sizeId: e.currentTarget.dataset.id }); this.calcSpec(); },
  onSpicy(e) { this.setData({ spicy: e.currentTarget.dataset.v }); this.calcSpec(); },
  onExtra(e) {
    const id = e.currentTarget.dataset.id;
    const on = Object.assign({}, this.data.extrasOn);
    if (on[id]) delete on[id]; else on[id] = true;
    this.setData({ extrasOn: on });
    this.calcSpec();
  },
  specMinus() { if (this.data.specQty > 1) { this.setData({ specQty: this.data.specQty - 1 }); this.calcSpec(); } },
  specPlus() { if (this.data.specQty < 20) { this.setData({ specQty: this.data.specQty + 1 }); this.calcSpec(); } },
  extraIds() { return EXTRAS.filter(x => this.data.extrasOn[x.id]).map(x => x.id); },
  calcSpec() {
    const d = S.dishById(this.data.spec.id);
    const unit = S.specPrice(d, this.data.sizeId, this.extraIds());
    this.setData({
      specPrice: S.yuan(unit * this.data.specQty),
      specDesc: S.specText(d, this.data.sizeId, this.data.spicy, this.extraIds())
    });
  },
  addSpec() {
    const d = S.dishById(this.data.spec.id);
    const ex = this.extraIds();
    const item = {
      key: [d.id, this.data.sizeId, this.data.spicy].concat(ex).join('|'),
      id: d.id, name: d.name, spec: S.specText(d, this.data.sizeId, this.data.spicy, ex),
      unit: S.specPrice(d, this.data.sizeId, ex), qty: this.data.specQty, bg: d.bg, ch: d.ch
    };
    this.refreshCart(S.addToCart(item));
    this.setData({ spec: null });
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },
  toggleCart() {
    if (!this.data.cart.length) return;
    this.setData({ showCart: !this.data.showCart });
  },
  closeCart() { if (this.innerTapAt && Date.now() - this.innerTapAt < 400) return; this.setData({ showCart: false }); },
  cartPlus(e) { this.refreshCart(S.changeQty(e.currentTarget.dataset.key, 1)); },
  cartMinus(e) { this.refreshCart(S.changeQty(e.currentTarget.dataset.key, -1)); },
  askClear() { this.setData({ showClear: true }); },
  cancelClear() { this.setData({ showClear: false }); },
  doClear() {
    S.saveCart([]);
    this.refreshCart([]);
    wx.showToast({ title: '购物车已清空', icon: 'none' });
  },
  noop() { this.innerTapAt = Date.now(); },
  checkout() {
    if (!this.data.sum.count) return;
    if (!this.data.table) {
      wx.showModal({
        title: '还没选桌号', content: '扫桌贴二维码或手动输入桌号后才能下单', confirmText: '去选桌',
        success: r => { if (r.confirm) wx.switchTab({ url: '/pages/scan/scan' }); }
      });
      return;
    }
    this.setData({ showCart: false });
    wx.navigateTo({ url: '/pages/confirm/confirm' });
  },
  changeTable() { wx.switchTab({ url: '/pages/scan/scan' }); }
});
