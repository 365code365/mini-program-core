// tab 页 onShow 里调：同步自定义 tabBar 的选中项与「复习」角标（到期待复习的词数）
const store = require('./store.js');

function sync(page, selected) {
  const tb = page.getTabBar && page.getTabBar();
  if (!tb) return;
  const n = store.dueList().length;
  tb.setData({ selected, badge: n > 99 ? '99+' : n > 0 ? String(n) : '' });
}

module.exports = { sync };
