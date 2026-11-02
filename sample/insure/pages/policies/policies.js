// 我的保单：按状态筛选；状态由日期实时算（待支付 / 待生效 / 保障中 / 已过期 / 已取消）
const store = require('../../utils/store.js');
const util = require('../../utils/util.js');
const TABS = [{ key: 'all', name: '全部' }, { key: 'active', name: '保障中' }, { key: 'waiting', name: '待生效' }, { key: 'pending', name: '待支付' }, { key: 'expired', name: '已失效' }];
Page({
  data: { tabs: TABS, tab: 'all', list: [], counts: {}, total: 0 },
  onShow() {
    const tb = this.getTabBar && this.getTabBar();
    if (tb) tb.setData({ selected: 2 });
    this.refresh();
  },
  refresh() {
    const all = store.policies().map(p => {
      const st = store.statusOf(p);
      return Object.assign({}, p, { st, premiumText: util.money(p.premium), group: st.key === 'cancelled' ? 'expired' : st.key });
    });
    const counts = { all: all.length };
    all.forEach(p => { counts[p.group] = (counts[p.group] || 0) + 1; });
    const t = this.data.tab;
    this.setData({ list: t === 'all' ? all : all.filter(p => p.group === t), counts, total: all.filter(p => p.group === 'active').length });
  },
  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.key }); this.refresh(); },
  open(e) { wx.navigateTo({ url: '/pages/policy/policy?id=' + e.currentTarget.dataset.id }); },
  goBuy() { wx.switchTab({ url: '/pages/products/products' }); },
  onPullDownRefresh() { this.refresh(); wx.stopPullDownRefresh(); }
});
