const S = require('../../utils/store.js');

Page({
  onShow() {
    const tb = this.getTabBar && this.getTabBar();
    if (tb) tb.setData({ selected: 2, badge: S.approvals().filter(a => a.needMe).length });
  }
});
