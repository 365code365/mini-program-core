const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const TABS = [
  { key: 'all', name: '全部' },
  { key: 'active', name: '进行中' },
  { key: 'done', name: '已完成' },
  { key: 'cancelled', name: '已取消' }
];
function match(tab, st) {
  if (tab === 'active') return st.active;
  if (tab === 'done') return st.stage === 3;
  if (tab === 'cancelled') return st.stage === -1;
  return true;
}
Page({
  data: {
    tabs: TABS,
    tab: 'all',
    list: []
  },
  onLoad(q) {
    if (q && q.tab && TABS.some(t => t.key === q.tab)) this.setData({ tab: q.tab });
  },
  onShow() {
    this.load();
  },
  load() {
    const now = Date.now();
    const list = store.getOrders().map(o => {
      const st = store.stageOf(o, now);
      return {
        id: o.id,
        code: o.code,
        mode: o.mode,
        storeName: data.findStore(o.storeId).name,
        status: st.status,
        st,
        imgs: o.items.slice(0, 3).map(i => i.img),
        summary: o.items.map(i => `${i.name}×${i.qty}`).join('、'),
        count: o.count,
        pay: o.pay,
        time: store.dateTime(o.createdAt),
        canRate: st.stage === 3 && !o.rating
      };
    }).filter(x => match(this.data.tab, x.st));
    this.setData({ list });
  },
  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.key });
    this.load();
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id });
  },
  onRate(e) {
    wx.navigateTo({ url: '/pages/order-detail/order-detail?rate=1&id=' + e.currentTarget.dataset.id });
  },
  onAgain(e) {
    const o = store.findOrder(e.currentTarget.dataset.id);
    if (o && store.reorder(o)) wx.navigateTo({ url: '/pages/cart/cart' });
  },
  onGoMenu() {
    wx.switchTab({ url: '/pages/menu/menu' });
  }
});
