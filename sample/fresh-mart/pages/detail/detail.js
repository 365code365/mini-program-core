const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

function pad(n) {
  return n < 10 ? '0' + n : '' + n;
}

const STORE = {
  fruit: ['常温 / 冷藏 0~4℃', '3~5 天', '收到后尽快食用，表皮有果霜属正常现象。'],
  veg: ['冷藏 0~4℃', '2~3 天', '当日采摘，叶面微湿是保鲜喷淋，不影响品质。'],
  meat: ['冷藏 0~4℃ / 冷冻 -18℃', '冷鲜 3 天，冷冻 6 个月', '全程冷链配送，到手请及时放入冰箱。'],
  sea: ['冷冻 -18℃', '12 个月', '解冻后请勿反复冷冻，建议冷水自然解冻。'],
  dairy: ['冷藏 2~6℃', '7 天', '开封后请尽快饮用，每日清晨到店。'],
  grain: ['阴凉干燥处', '12 个月', '新米上市，开袋后建议密封保存。'],
  snack: ['阴凉干燥处', '9 个月', '独立小包装，每日一包刚刚好。'],
  drink: ['常温 / 冷藏风味更佳', '12 个月', '100% 椰青原液，无添加。']
};

Page({
  data: {
    goods: {},
    flash: null,
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
    desc: ''
  },

  onLoad(options) {
    const goods = data.findGoods(options.id || 'p1');
    const st = STORE[goods.cat] || STORE.fruit;
    const favs = wx.getStorageSync('fm_favs') || [];
    const eta = new Date(Date.now() + 30 * 60000);
    wx.setNavigationBarTitle({ title: goods.name });
    this.setData({
      goods,
      flash: data.flashOf(goods.id),
      faved: favs.indexOf(goods.id) >= 0,
      address: getApp().globalData.address,
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
    this.setData({ cartCount: cart.count() });
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
    this.setData({ unit: cart.unitPrice(g.id, spec) });
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
    if (this.data.qty >= 99) return;
    this.setData({ qty: this.data.qty + 1 });
  },

  onFav() {
    const favs = wx.getStorageSync('fm_favs') || [];
    const id = this.data.goods.id;
    const i = favs.indexOf(id);
    if (i >= 0) favs.splice(i, 1);
    else favs.push(id);
    wx.setStorageSync('fm_favs', favs);
    this.setData({ faved: i < 0 });
    wx.showToast({ title: i < 0 ? '收藏成功' : '已取消收藏', icon: 'none' });
  },

  onAdd() {
    const g = this.data.goods;
    const n = cart.add(g.id, this.data.qty, g.specs[this.data.specIdx][0]);
    this.setData({ cartCount: n });
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },

  onBuy() {
    this.onAdd();
    wx.switchTab({ url: '/pages/cart/cart' });
  },

  onHome() {
    wx.switchTab({ url: '/pages/index/index' });
  },

  onCart() {
    wx.switchTab({ url: '/pages/cart/cart' });
  }
});
