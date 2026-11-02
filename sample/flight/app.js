const store = require('./utils/store');

App({
  globalData: {
    // 当前预订：trip 单程 one / 往返 round；legs 已选的航段（航班 + 舱位）
    booking: null
  },
  onLaunch() {
    store.seed();
  },
  // tab 页 onShow 里调：选中态 + 「订单」角标（待支付数）
  syncTab(page, n) {
    const tb = page.getTabBar && page.getTabBar();
    if (!tb) return;
    const unpaid = store.orders().filter(o => o.status === 'unpaid').length;
    tb.setData({ selected: n, unpaid });
  }
});
