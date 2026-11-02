const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const FILTERS = [{ k: 'all', n: '全部' }, { k: 'good', n: '好评' }, { k: 'bad', n: '中差评' }];
Page({
  data: { mine: false, h: null, filters: FILTERS, filter: 'all', tags: data.RATE_TAGS, tag: '', list: [], count: 0, pending: 0, starList: [1, 2, 3, 4, 5] },
  onLoad(q) {
    const mine = String(q.mine) === '1';
    this.setData({ mine, tag: q.tag ? decodeURIComponent(q.tag) : '' });
    if (mine) { wx.setNavigationBarTitle({ title: '我的点评' }); return; }
    const h = data.hotelOf(q.hid);
    this.h = h;
    this.setData({ h });
  },
  onShow() { this.load(); },
  load() {
    if (this.data.mine) {
      const orders = store.getOrders();
      const list = orders.filter(o => o.rating).map(o => {
        const h = data.hotelOf(o.hid);
        return { k: o.id, id: o.id, hotel: h.name, room: (h.rooms.find(r => r.id === o.rid) || h.rooms[0]).name, score: o.rating.score, tags: o.rating.tags, text: o.rating.text, date: o.rating.time.slice(0, 10) };
      });
      this.setData({ list, count: list.length, pending: orders.filter(o => store.stageOf(o).canRate).length });
      return;
    }
    const all = store.reviewsOf(this.h);
    const { filter, tag } = this.data;
    const list = all.filter(r => (filter === 'all' || (filter === 'good' ? r.score >= 4 : r.score <= 3)) && (!tag || r.tags.indexOf(tag) >= 0));
    this.setData({ list, count: this.h.reviews + all.filter(r => r.mine).length });
  },
  onFilter(e) { this.setData({ filter: e.currentTarget.dataset.k }); this.load(); },
  onTag(e) { const t = e.currentTarget.dataset.t; this.setData({ tag: this.data.tag === t ? '' : t }); this.load(); },
  onOrder(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  // 待点评的订单在「订单」tab 的「已完成」里（switchTab 带不了参数，放在 storage）
  onPending() {
    wx.setStorageSync(store.K.orderTab, 'done');
    wx.switchTab({ url: '/pages/orders/orders' });
  }
});
