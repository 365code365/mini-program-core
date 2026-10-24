const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

function salesNum(s) { return s.indexOf('万') >= 0 ? parseFloat(s) * 10000 : parseInt(s.replace(/,/g, ''), 10); }

Page({
  data: {
    cats: ['全部', '主粮', '零食', '玩具', '用品', '洗护'], cat: '全部',
    sorts: [{ k: 'def', n: '综合' }, { k: 'sales', n: '销量' }, { k: 'price', n: '价格' }], sort: 'def',
    kw: '', list: [], cartCount: 0
  },
  onShow() { this.refresh(); this.setData({ cartCount: cart.count() }); },
  refresh() {
    const { cat, kw, sort } = this.data;
    let list = data.goods.filter(g => (cat === '全部' || g.cat === cat) && (!kw || g.name.indexOf(kw) >= 0 || g.sub.indexOf(kw) >= 0 || g.cat.indexOf(kw) >= 0));
    if (sort === 'sales') list = list.slice().sort((a, b) => salesNum(b.sales) - salesNum(a.sales));
    if (sort === 'price') list = list.slice().sort((a, b) => a.price - b.price);
    this.setData({ list });
  },
  onKw(e) { this.setData({ kw: e.detail.value.trim() }); this.refresh(); },
  clearKw() { this.setData({ kw: '' }); this.refresh(); },
  pickCat(e) { this.setData({ cat: e.currentTarget.dataset.c }); this.refresh(); },
  pickSort(e) { this.setData({ sort: e.currentTarget.dataset.k }); this.refresh(); },
  quickAdd(e) {
    const g = data.findGoods(e.currentTarget.dataset.id);
    this.setData({ cartCount: cart.add(g.id, g.specs[0], 1) });
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },
  goGoods(e) { wx.navigateTo({ url: '/pages/goods/goods?id=' + e.currentTarget.dataset.id }); },
  goCart() { wx.navigateTo({ url: '/pages/cart/cart' }); }
});
