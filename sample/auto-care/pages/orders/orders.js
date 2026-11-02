const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { STATUS_TEXT } = require('../../utils/data.js');

const TABS = [
  { k: 'all', name: '全部' }, { k: 'pending', name: '待服务' }, { k: 'serving', name: '服务中' },
  { k: 'done', name: '待评价' }, { k: 'reviewed', name: '已完成' }
];

Page({
  data: { tabs: TABS, tab: 'all', list: [], loading: true, counts: {} },
  onLoad(q) { if (q.tab) this.setData({ tab: q.tab }); },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 2 });
    // 从「我的」点某个状态进来时带着 tab
    const want = getApp().globalData.orderTab;
    if (want) { getApp().globalData.orderTab = ''; this.setData({ tab: want }); }
    this.load(true);
  },
  onPullDownRefresh() { this.load(true, () => wx.stopPullDownRefresh()); },
  load(showLoading, done) {
    if (showLoading) this.setData({ loading: true });
    // 模拟一次网络读取，列表先显示加载态
    setTimeout(() => {
      const all = db.getOrders().map((o) => Object.assign({}, o, {
        statusText: STATUS_TEXT[o.status], plateText: util.fmtPlate(o.plate),
        itemText: o.items.map((i) => i.name).join('、')
      }));
      const counts = {};
      all.forEach((o) => { counts[o.status] = (counts[o.status] || 0) + 1; });
      const list = this.data.tab === 'all' ? all : all.filter((o) => o.status === this.data.tab);
      this.setData({ list, counts, loading: false });
      done && done();
    }, 250);
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.load(false); },
  goDetail(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  goReview(e) { wx.navigateTo({ url: '/pages/review/review?id=' + e.currentTarget.dataset.id }); },
  goBook() { wx.navigateTo({ url: '/pages/service/service?cat=care' }); }
});
