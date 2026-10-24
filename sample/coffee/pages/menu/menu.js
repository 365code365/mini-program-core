const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: {
    store: data.stores[0],
    mode: 'pickup',
    categories: data.categories,
    catId: 'new',
    catName: '当季新品',
    list: [],
    listTop: 0,
    bodyHeight: 500,
    tabGap: 0,
    cart: { count: 0, amount: 0 },
    sheet: null,
    sizes: data.sizes,
    sugars: data.sugars,
    sizeIdx: 0,
    tempIdx: 0,
    sugarIdx: 0,
    qty: 1,
    unitPrice: 0
  },

  onLoad() {
    const info = wx.getWindowInfo();
    // 兼容：微信里 tab 页的 windowHeight 已经扣掉了 tabBar，position:fixed 的 bottom:0 贴在 tabBar 上沿；
    // 本引擎 windowHeight 含 tabBar 区域，fixed 层也画到屏幕最底（被 tabBar 盖住、点击被 tabBar 吃掉）。
    // 用「屏高 - 窗口高」判断窗口里有没有扣掉 tabBar，没扣就自己让出 50px。
    const tabGap = info.screenHeight - info.windowHeight > 90 ? 0 : 50;
    const bodyHeight = Math.floor(info.windowHeight - tabGap - 128 * info.windowWidth / 750);
    this.setData({ bodyHeight, tabGap });
    this.showCat('new');
  },

  onShow() {
    const storeId = getApp().globalData.storeId;
    this.setData({
      mode: store.getMode(),
      store: data.stores.find(s => s.id === storeId) || data.stores[0]
    });
    this.refreshCart();
    // 首页 banner / 推荐点进来：直接弹规格
    const openId = wx.getStorageSync('cf_open_drink');
    if (openId) {
      wx.removeStorageSync('cf_open_drink');
      const d = data.findDrink(openId);
      this.showCat(d.cat);
      this.openSheet(d);
    }
  },

  showCat(id) {
    const cat = data.categories.find(c => c.id === id) || data.categories[0];
    this.setData({
      catId: cat.id,
      catName: cat.name,
      list: data.drinks.filter(d => d.cat === cat.id),
      listTop: this.data.listTop === 0 ? 0.1 : 0
    });
  },

  refreshCart() {
    const s = store.summary();
    this.setData({ cart: { count: s.count, amount: s.amount } });
  },

  onCat(e) {
    this.showCat(e.currentTarget.dataset.id);
  },

  onMode(e) {
    const mode = e.currentTarget.dataset.mode;
    store.setMode(mode);
    this.setData({ mode });
    wx.showToast({ title: mode === 'pickup' ? '已切换为到店自取' : '已切换为外卖送达', icon: 'none' });
  },

  onStore() {
    wx.showActionSheet({
      itemList: data.stores.map(s => `${s.name}  ${s.distance}`),
      success: (res) => {
        const s = data.stores[res.tapIndex];
        getApp().globalData.storeId = s.id;
        wx.setStorageSync('cf_store', s.id);
        this.setData({ store: s });
      }
    });
  },

  onOpen(e) {
    this.openSheet(data.findDrink(e.currentTarget.dataset.id));
  },

  openSheet(d) {
    // 记住每款饮品上次选的规格
    const last = (wx.getStorageSync('cf_last_spec') || {})[d.id] || {};
    const tempIdx = Math.min(last.tempIdx || 0, d.temps.length - 1);
    this.setData({
      sheet: d,
      sizeIdx: last.sizeIdx || 0,
      tempIdx,
      sugarIdx: last.sugarIdx || 0,
      qty: 1
    });
    this.calc();
  },

  calc() {
    const d = this.data.sheet;
    if (!d) return;
    this.setData({ unitPrice: d.price + data.sizes[this.data.sizeIdx].add });
  },

  onSize(e) {
    this.setData({ sizeIdx: Number(e.currentTarget.dataset.idx) });
    this.calc();
  },

  onTemp(e) {
    this.setData({ tempIdx: Number(e.currentTarget.dataset.idx) });
  },

  onSugar(e) {
    this.setData({ sugarIdx: Number(e.currentTarget.dataset.idx) });
  },

  onQtyMinus() {
    if (this.data.qty > 1) this.setData({ qty: this.data.qty - 1 });
  },

  onQtyPlus() {
    if (this.data.qty >= 9) {
      wx.showToast({ title: '单款最多 9 杯', icon: 'none' });
      return;
    }
    this.setData({ qty: this.data.qty + 1 });
  },

  onClose() {
    this.setData({ sheet: null });
  },

  onAdd() {
    const d = this.data.sheet;
    const { sizeIdx, tempIdx, sugarIdx, qty, unitPrice } = this.data;
    store.addToCart({
      drinkId: d.id,
      size: data.sizes[sizeIdx].name,
      temp: d.temps[tempIdx],
      sugar: data.sugars[sugarIdx],
      qty,
      unit: unitPrice
    });
    const last = wx.getStorageSync('cf_last_spec') || {};
    last[d.id] = { sizeIdx, tempIdx, sugarIdx };
    wx.setStorageSync('cf_last_spec', last);
    this.setData({ sheet: null });
    this.refreshCart();
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },

  onCart() {
    wx.navigateTo({ url: '/pages/cart/cart' });
  }
});
