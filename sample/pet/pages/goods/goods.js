const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

Page({
  data: {
    g: {}, gallery: ['#fff7ef', '#f3f8ff', '#f4fbf6'], spec: '', qty: 1, fav: false, cartCount: 0, saved: 0,
    reviews: [
      { name: '奶盖妈妈', sx: '奶', pet: '布偶猫 · 1 岁', text: '回购第三次了，家里挑食的小祖宗也吃得很香，便便成型很好。', color: '#ffb38a' },
      { name: 'Momo', sx: 'M', pet: '柯基 · 2 岁', text: '物流很快，第二天就到了，包装严实。颗粒大小合适。', color: '#8ab4ff' }
    ]
  },
  onLoad(q) {
    const g = data.findGoods(q.id || 'g1') || data.goods[0];
    const favs = wx.getStorageSync('pet_favs') || [];
    this.setData({ g, spec: g.specs[0], fav: favs.indexOf(g.id) >= 0, saved: Math.round((g.orig - g.price) * 10) / 10 });
  },
  onShow() { this.setData({ cartCount: cart.count() }); },
  pickSpec(e) { this.setData({ spec: e.currentTarget.dataset.s }); },
  inc() { if (this.data.qty < 99) this.setData({ qty: this.data.qty + 1 }); },
  dec() { if (this.data.qty > 1) this.setData({ qty: this.data.qty - 1 }); },
  toggleFav() {
    let favs = wx.getStorageSync('pet_favs') || [];
    const on = !this.data.fav;
    favs = on ? favs.concat(this.data.g.id) : favs.filter(x => x !== this.data.g.id);
    wx.setStorageSync('pet_favs', favs);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
  },
  addCart() {
    this.setData({ cartCount: cart.add(this.data.g.id, this.data.spec, this.data.qty) });
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },
  buyNow() {
    cart.add(this.data.g.id, this.data.spec, this.data.qty);
    wx.navigateTo({ url: '/pages/cart/cart' });
  },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  goCart() { wx.navigateTo({ url: '/pages/cart/cart' }); }
});
