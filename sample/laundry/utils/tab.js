// tab 页 onShow：选中态 + 价目表角标（购物篮件数）
const store = require('./store.js');

function sync(page, index) {
  if (typeof page.getTabBar !== 'function' || !page.getTabBar()) return;
  page.getTabBar().setData({ selected: index, cartN: store.cartSummary().count });
}

module.exports = { sync };
