const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: {
    banners: [
      { title: '七夕 · 把心意说出口', sub: '红玫瑰花束直降 60 元，当日达', btn: '去挑选', bg: 'linear-gradient(120deg, #ff8fa8, #e8466f)', img: '/images/b1.png', id: 'f1' },
      { title: '教师节 · 感恩有你', sub: '向日葵花束 · 第二束半价', btn: '送老师', bg: 'linear-gradient(120deg, #ffcf6b, #ff9a3d)', img: '/images/b4.png', id: 'f4' },
      { title: '中秋 · 花好月圆', sub: '探望长辈花礼满 199 减 30', btn: '看花礼', bg: 'linear-gradient(120deg, #6fd3a8, #20a37a)', img: '/images/b6.png', id: 'f6' }
    ],
    uses: data.uses,
    use: '',
    left: [], right: [], count: 0, cartCount: 0
  },
  onLoad() { this.refresh(); },
  onShow() { this.setData({ cartCount: store.cartCount() }); },
  refresh() {
    const list = data.products.filter(p => !this.data.use || p.use === this.data.use)
      .map(p => Object.assign({}, p, { soldTx: p.sold >= 10000 ? (p.sold / 10000).toFixed(1) + '万' : p.sold }));
    // 双列瀑布：按下标奇偶分两列
    this.setData({ left: list.filter((x, i) => i % 2 === 0), right: list.filter((x, i) => i % 2 === 1), count: list.length });
  },
  onUse(e) {
    const k = e.currentTarget.dataset.k;
    this.setData({ use: this.data.use === k ? '' : k });
    this.refresh();
  },
  goDetail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  quickAdd(e) {
    const id = e.currentTarget.dataset.id;
    store.addToCart({ id, stems: 11, pack: 'mist', date: store.dayAfter(1), slot: '15:00-18:00', card: data.productOf(id).words, qty: 1 });
    this.setData({ cartCount: store.cartCount() });
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },
  goCart() { wx.switchTab({ url: '/pages/cart/cart' }); },
  onService() {
    wx.showActionSheet({ itemList: ['在线客服', '拨打 400-660-5520', '企业团购咨询'], success: r => {
      wx.showToast({ title: ['花艺师马上为您服务', '正在呼叫…', '商务顾问将在 1 小时内联系您'][r.tapIndex], icon: 'none' });
    } });
  }
});
