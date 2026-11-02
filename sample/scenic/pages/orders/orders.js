const store = require('../../utils/store.js');
const tab = require('../../utils/tab.js');
const TABS = [{ k: 'all', n: '全部' }, { k: 'paid', n: '待使用' }, { k: 'used', n: '已使用' }, { k: 'refunded', n: '已退款' }];

Page({
  data: { tabs: TABS, cur: 0, list: [], loading: true },
  onShow() {
    tab.sync(this, 2);
    this.load();
  },
  load() {
    this.setData({ loading: true });
    // 本地读很快，留一帧给加载态，和接口返回的节奏一致
    setTimeout(() => {
      const k = TABS[this.data.cur].k;
      const list = store.orders().filter((o) => k === 'all' || o.status === k)
        .map((o) => Object.assign({}, o, { summary: o.items.map((x) => x.name + '×' + x.n).join('，') }));
      this.setData({ list, loading: false });
    }, 120);
  },
  onTab(e) { this.setData({ cur: +e.currentTarget.dataset.i }); this.load(); },
  goDetail(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?no=' + e.currentTarget.dataset.no }); },
  goTicket(e) { wx.navigateTo({ url: '/pages/ticket/ticket?no=' + e.currentTarget.dataset.no }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
