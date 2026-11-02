const { goodsById, farmers } = require('../../utils/data.js');
const store = require('../../utils/store.js');
const ticker = require('../../utils/ticker.js');
const sku = require('../../utils/sku.js');

const MODE = {
  cart: { text: '单买价', ok: '加入购物车', field: 'price' },
  single: { text: '单买价', ok: '单独购买', field: 'price' },
  group: { text: '拼团价', ok: '发起拼团', field: 'groupPrice' },
  join: { text: '拼团价', ok: '参与拼团', field: 'groupPrice' }
};

Page({
  data: {
    g: null, farmer: {}, tab: 'video', imgIndex: 0, video: '/images/misc/origin.mp4', saveText: '', fav: false, cartCount: 0,
    groups: [], skuShow: false, mode: 'cart', modeText: '', okText: '', dims: [], skuKey: '', skuPrice: '', skuStock: '',
    chosenText: '', qty: 1, maxQty: 1
  },

  onLoad(q) {
    const g = goodsById(q.id || 'g01');
    if (!g) { wx.showToast({ title: '商品已下架', icon: 'none' }); setTimeout(() => wx.navigateBack(), 800); return; }
    this.g = g;
    this.sel = sku.firstInStock(g);
    wx.setNavigationBarTitle({ title: g.name.split(' ')[0] });
    this.setData({ g, farmer: farmers[g.farmer], saveText: store.money(g.price - g.groupPrice), fav: store.favs().indexOf(g.id) >= 0 });
    this.refreshSku();
    this.loadGroups();
    // 从拼团大厅点「参团」进来：直接弹规格面板
    if (q.join) setTimeout(() => this.showSku('join', q.join), 300);
  },
  // 页面里每个团一个倒计时，共用 ticker 的一个定时器；onHide / onUnload 退订
  onShow() {
    this.setData({ cartCount: store.cartCount() });
    if (!this.g) return;
    this.loadGroups();
    ticker.attach(this, (now) => this.tick(now));
  },
  onHide() { ticker.detach(this); },
  onUnload() { ticker.detach(this); },

  loadGroups() {
    this._groups = store.openGroups(this.g.id);
    this.setData({ groups: this._groups.map((x) => Object.assign({}, x, { cd: ticker.left(x.endAt) })) });
  },
  tick(now) {
    const patch = {};
    (this._groups || []).forEach((x, i) => { patch[`groups[${i}].cd`] = ticker.left(x.endAt, now); });
    this.setData(patch);
  },

  switchMedia(e) { this.setData({ tab: e.currentTarget.dataset.t }); },
  onImgChange(e) { this.setData({ imgIndex: e.detail.current }); },
  preview(e) { wx.previewImage({ urls: this.g.imgs, current: this.g.imgs[e.currentTarget.dataset.i] }); },
  onPlay() { this._playing = true; },
  onPause() { this._playing = false; },
  onEnded() { this._playing = false; },
  onVideoError() { wx.showToast({ title: '视频加载失败，已切到图集', icon: 'none' }); this.setData({ tab: 'img' }); },

  // ---------- 规格面板 ----------
  openSku(e) { this.showSku(e.currentTarget.dataset.mode || 'cart', null); },
  joinGroup(e) { this.showSku('join', e.currentTarget.dataset.id); },
  showSku(mode, joinId) {
    if (mode === 'join' && store.myGroups().some((x) => x.goodsId === this.g.id && x.status === 'ongoing')) {
      wx.showModal({ title: '提示', content: '你已在拼这件商品的团，每人限参 1 团', showCancel: false });
      return;
    }
    this.joinId = joinId;
    const m = MODE[mode];
    this.setData({ mode, modeText: m.text, okText: m.ok, skuShow: true });
    this.refreshSku();
  },
  closeSku() { this.setData({ skuShow: false }); },
  onSkuLeave() { this.setData({ skuShow: false }); },
  pick(e) {
    const { d, o } = e.currentTarget.dataset;
    const next = sku.toggle(this.g, this.sel, d, o);
    if (next === this.sel) { wx.showToast({ title: '该规格暂时缺货', icon: 'none' }); return; }
    this.sel = next;
    this.refreshSku();
  },
  refreshSku() {
    const g = this.g;
    const key = sku.keyOf(this.sel);
    const field = MODE[this.data.mode].field;
    const info = key ? g.sku.map[key] : null;
    const grp = this.data.mode === 'group' || this.data.mode === 'join';
    const maxQty = info ? Math.min(info.stock, grp ? 5 : 20) : 1;
    this.setData({
      dims: sku.view(g, this.sel), skuKey: key,
      skuPrice: info ? info[field] : sku.priceRange(g, this.sel, field),
      skuStock: info ? (info.stock < 100 ? `仅剩 ${info.stock} ${g.unit}` : `库存充足`) : '请选择规格',
      chosenText: key ? store.skuText(g, key) + ` × ${this.data.qty}` : '未选择规格',
      maxQty, qty: Math.max(1, Math.min(this.data.qty, maxQty))
    });
  },
  stepQty(e) {
    const q = this.data.qty + Number(e.currentTarget.dataset.d);
    if (q < 1 || q > this.data.maxQty) {
      if (q > this.data.maxQty) wx.showToast({ title: this.data.mode === 'cart' || this.data.mode === 'single' ? '已达库存上限' : '拼团每单最多 5 件', icon: 'none' });
      return;
    }
    this.setData({ qty: q });
    this.refreshSku();
  },
  confirmSku() {
    const key = this.data.skuKey;
    if (!key) { wx.showToast({ title: '请先选择规格', icon: 'none' }); return; }
    const { mode, qty } = this.data;
    if (mode === 'cart') {
      store.addCart(this.g.id, key, qty);
      this.setData({ skuShow: false, cartCount: store.cartCount() });
      wx.showToast({ title: '已加入购物车', icon: 'success' });
      return;
    }
    this.setData({ skuShow: false });
    let url = `/pages/checkout/checkout?mode=${mode}&goods=${this.g.id}&sku=${encodeURIComponent(key)}&qty=${qty}`;
    if (mode === 'join') url += '&join=' + this.joinId;
    wx.navigateTo({ url });
  },

  toggleFav() {
    const on = store.toggleFav(this.g.id);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: on ? 'success' : 'none' });
  },
  goCart() { wx.switchTab({ url: '/pages/cart/cart' }); },
  goGroupHall() { wx.navigateTo({ url: '/pages/group/group?goods=' + this.g.id }); },
  callFarmer() {
    const f = this.data.farmer;
    wx.makePhoneCall({
      phoneNumber: f.phone,
      fail: () => wx.showModal({
        title: `联系 ${f.name}`, content: `果园电话：${f.phone}\n（工作日 8:00-18:00）`, confirmText: '复制号码',
        success: (r) => { if (r.confirm) wx.setClipboardData({ data: f.phone }); }
      })
    });
  },
  onShareAppMessage() {
    return { title: `${this.g.groupSize}人拼 ¥${this.g.groupPrice}｜${this.g.name}`, path: '/pages/detail/detail?id=' + this.g.id, imageUrl: this.g.cover };
  }
});
