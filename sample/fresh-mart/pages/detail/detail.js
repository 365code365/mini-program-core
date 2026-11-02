const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');

const pad = store.pad;

const STORE = {
  fruit: ['常温 / 冷藏 0~4℃', '3~5 天', '收到后尽快食用，表皮有果霜属正常现象。'],
  veg: ['冷藏 0~4℃', '2~3 天', '当日采摘，叶面微湿是保鲜喷淋，不影响品质。'],
  meat: ['冷藏 0~4℃ / 冷冻 -18℃', '冷鲜 3 天，冷冻 6 个月', '全程冷链配送，到手请及时放入冰箱。'],
  sea: ['冷冻 -18℃', '12 个月', '解冻后请勿反复冷冻，建议冷水自然解冻。'],
  dairy: ['冷藏 2~6℃', '7 天', '开封后请尽快饮用，每日清晨到店。'],
  grain: ['阴凉干燥处', '12 个月', '开封后建议密封保存，避免受潮。'],
  snack: ['阴凉干燥处', '9 个月', '独立小包装，每日一包刚刚好。'],
  drink: ['常温 / 冷藏风味更佳', '12 个月', '开封后请冷藏并尽快饮用。'],
  frozen: ['冷冻 -18℃', '12 个月', '冷链保温箱配送，到手请立即放入冷冻室。'],
  daily: ['常温干燥处', '36 个月', '家庭常备，囤货更划算。']
};

Page({
  data: {
    goods: {},
    flash: null,
    limit: data.FLASH_LIMIT,
    current: 0,
    specIdx: 0,
    qty: 1,
    unit: 0,
    faved: false,
    cartCount: 0,
    address: '',
    eta: '',
    hh: '00', mm: '00', ss: '00',
    params: [],
    desc: '',
    starList: [1, 2, 3, 4, 5],
    reviews: [],
    reviewCount: 0
  },

  onLoad(options) {
    const goods = data.findGoods(options.id || 'p1');
    const st = STORE[goods.cat] || STORE.fruit;
    const eta = new Date(Date.now() + 30 * 60000);
    wx.setNavigationBarTitle({ title: goods.name });
    this.setData({
      goods,
      flash: data.flashOf(goods.id),
      eta: pad(eta.getHours()) + ':' + pad(eta.getMinutes()),
      params: [
        { k: '产地', v: goods.place },
        { k: '规格', v: goods.spec },
        { k: '储存条件', v: st[0] },
        { k: '保质期', v: st[1] }
      ],
      desc: `${goods.name}，${goods.tags.join('、')}。${st[2]}`
    });
    this.calc();
  },

  onShow() {
    const a = store.currentAddr();
    const seed = (data.reviews[this.data.goods.cat] || []).map(r => ({ user: r[0], stars: r[1], text: r[2], time: '' }));
    const mine = orders.myReviews(this.data.goods.id);
    this.setData({
      cartCount: store.cartCount(),
      faved: store.isFav(this.data.goods.id),
      address: a ? a.detail : '请先添加收货地址',
      reviews: mine.concat(seed),
      reviewCount: Math.floor(this.data.goods.sales * 0.3) + mine.length
    });
    if (this.data.flash) {
      this.tick();
      clearInterval(this.timer);
      this.timer = setInterval(() => this.tick(), 1000);
    }
  },

  onHide() {
    clearInterval(this.timer);
  },

  onUnload() {
    clearInterval(this.timer);
  },

  tick() {
    const now = new Date();
    const startHour = Math.floor(now.getHours() / 2) * 2;
    const end = new Date(now.getFullYear(), now.getMonth(), now.getDate(), startHour + 2, 0, 0).getTime();
    const left = Math.max(0, Math.floor((end - now.getTime()) / 1000));
    this.setData({ hh: pad(Math.floor(left / 3600)), mm: pad(Math.floor((left % 3600) / 60)), ss: pad(left % 60) });
  },

  calc() {
    const g = this.data.goods;
    const spec = g.specs[this.data.specIdx][0];
    this.setData({ unit: store.unitPrice(g.id, spec) });
  },

  onSwiper(e) {
    this.setData({ current: e.detail.current });
  },

  onSpec(e) {
    this.setData({ specIdx: Number(e.currentTarget.dataset.idx) });
    this.calc();
  },

  onMinus() {
    if (this.data.qty > 1) this.setData({ qty: this.data.qty - 1 });
  },

  onPlus() {
    const max = this.data.flash ? data.FLASH_LIMIT : 99;
    if (this.data.qty >= max) {
      if (this.data.flash) wx.showToast({ title: `秒杀价每单限购 ${max} 件`, icon: 'none' });
      return;
    }
    this.setData({ qty: this.data.qty + 1 });
  },

  onAddress() {
    wx.navigateTo({ url: '/pages/address/address?select=1' });
  },

  onFav() {
    const on = store.toggleFav(this.data.goods.id);
    this.setData({ faved: on });
    wx.showToast({ title: on ? '收藏成功' : '已取消收藏', icon: 'none' });
  },

  onAdd() {
    const g = this.data.goods;
    const r = store.addToCart(g.id, this.data.qty, g.specs[this.data.specIdx][0]);
    this.setData({ cartCount: r.count });
    if (r.ok && !r.msg) wx.showToast({ title: '已加入购物车', icon: 'success' });
    else wx.showToast({ title: r.msg, icon: 'none' });
  },

  // 立即购买：只结算这一件，不动购物车
  onBuy() {
    const g = this.data.goods;
    store.setCheckout([{ id: g.id, spec: g.specs[this.data.specIdx][0], qty: this.data.qty }], 'buy');
    wx.navigateTo({ url: '/pages/checkout/checkout' });
  },

  onHome() {
    wx.switchTab({ url: '/pages/index/index' });
  },

  onCart() {
    wx.switchTab({ url: '/pages/cart/cart' });
  }
});
