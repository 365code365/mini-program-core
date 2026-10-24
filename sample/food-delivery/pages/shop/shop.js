const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

// 右侧列表的固定行高（rpx），用来把 scrollTop 换算成左侧当前分类
const SEC_TITLE = 64;
const DISH_ROW = 196;

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
    reviews: [
      { name: '爱吃鱼的猫', short: '猫', color: '#ffb347', stars: '★★★★★', date: '10-21', text: '分量很足，送到还是烫的，骑手小哥很客气，下次还点。' },
      { name: '望京打工人', short: '望', color: '#7cc6ff', stars: '★★★★★', date: '10-19', text: '中午 12 点下单 25 分钟就到了，包装很严实没有洒。' },
      { name: 'Lily', short: 'L', color: '#ffa6c1', stars: '★★★★☆', date: '10-16', text: '味道不错，就是稍微有点咸，备注了少盐下次试试。' }
    ]
  },

  onLoad(options) {
    const shop = data.findShop(options.id || 's1');
    wx.setNavigationBarTitle({ title: shop.name });
    const info = wx.getWindowInfo();
    const ratio = info.windowWidth / 750;
    this.ratio = ratio;
    // 头部 260 + tab 80（rpx）以下全给左右两栏
    const bodyHeight = Math.floor(info.windowHeight - (260 + 80) * ratio);
    // 每个分类在右栏里的起始位置（px）
    let top = 0;
    this.secTops = data.menus[shop.id].map(g => {
      const t = top;
      top += (SEC_TITLE + g.items.length * DISH_ROW) * ratio;
      return t;
    });
    const favs = wx.getStorageSync('fd_favs') || [];
    this.setData({ shop, bodyHeight, faved: favs.indexOf(shop.id) >= 0 });
    this.refreshCart();
  },

  onShow() {
    if (this.data.shop.id) this.refreshCart();
  },

  refreshCart() {
    const shop = this.data.shop;
    const s = cart.summary(shop.id);
    const menu = data.menus[shop.id].map(g => ({
      name: g.name,
      items: g.items,
      count: g.items.reduce((n, d) => n + (s.items[d.id] || 0), 0)
    }));
    const diff = cart.round2(shop.minOrder - s.amount);
    const canSubmit = s.count > 0 && diff <= 0;
    let goText = `¥${shop.minOrder} 起送`;
    if (s.count && diff > 0) goText = `还差 ¥${diff}`;
    if (canSubmit) goText = '去结算';
    this.setData({ menu, counts: s.items, cart: s, canSubmit, goText, showCart: this.data.showCart && s.count > 0 });
  },

  onPlus(e) {
    const c = cart.getCart();
    const id = e.currentTarget.dataset.id;
    if (c.shopId && c.shopId !== this.data.shop.id && Object.keys(c.items).length) {
      wx.showModal({
        title: '提示',
        content: '购物车里有其他店铺的商品，继续添加将清空原购物车',
        confirmText: '继续添加',
        success: (res) => {
          if (res.confirm) {
            cart.change(this.data.shop.id, id, 1);
            this.refreshCart();
          }
        }
      });
      return;
    }
    cart.change(this.data.shop.id, id, 1);
    this.refreshCart();
  },

  onMinus(e) {
    cart.change(this.data.shop.id, e.currentTarget.dataset.id, -1);
    this.refreshCart();
  },

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
          cart.clear();
          this.setData({ showCart: false });
          this.refreshCart();
        }
      }
    });
  },

  onFav() {
    const favs = wx.getStorageSync('fd_favs') || [];
    const id = this.data.shop.id;
    const i = favs.indexOf(id);
    if (i >= 0) favs.splice(i, 1);
    else favs.push(id);
    wx.setStorageSync('fd_favs', favs);
    this.setData({ faved: i < 0 });
    wx.showToast({ title: i < 0 ? '已收藏' : '已取消收藏', icon: 'none' });
  },

  onCall() {
    wx.showModal({ title: '联系商家', content: '010-6478 2211', confirmText: '呼叫' });
  },

  onCheckout() {
    if (!this.data.canSubmit) {
      if (this.data.cart.count) wx.showToast({ title: this.data.goText, icon: 'none' });
      return;
    }
    wx.navigateTo({ url: `/pages/checkout/checkout?shop=${this.data.shop.id}` });
  }
});
