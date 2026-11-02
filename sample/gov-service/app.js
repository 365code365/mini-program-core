App({
  globalData: { orderTab: '' },
  onLaunch() {
    // 首次进入预置两条历史办件（读一次就会写入 storage）
    require('./utils/store').getOrders();
  }
});
