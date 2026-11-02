// tab 页 onShow 调：同步自定义 tabBar 选中项，「订单」tab 角标 = 待使用订单数
const store = require('./store.js');

function sync(page, selected) {
  const tb = page.getTabBar && page.getTabBar();
  if (!tb) return;
  const n = store.orders().filter((o) => o.status === 'paid').length;
  tb.setData({ selected, badge: n ? String(n) : '' });
}

module.exports = { sync };
