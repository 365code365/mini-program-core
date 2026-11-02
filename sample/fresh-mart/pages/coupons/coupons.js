const store = require('../../utils/store.js');

Page({
  data: {
    tabs: [],
    tab: 'valid',
    list: []
  },

  onShow() {
    this.load();
  },

  load() {
    const all = store.allCoupons();
    const n = k => all.filter(c => c.status === k).length;
    this.setData({
      tabs: [
        { key: 'valid', name: `可使用(${n('valid')})` },
        { key: 'used', name: `已使用(${n('used')})` },
        { key: 'expired', name: `已过期(${n('expired')})` }
      ],
      list: all.filter(c => c.status === this.data.tab)
    });
  },

  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.key });
    this.load();
  },

  // 去使用：限品类的券带去那个分类，其余的：购物车有东西就去结算，没有就去首页逛
  onUse(e) {
    const c = this.data.list.find(x => x.id === e.currentTarget.dataset.id);
    if (c && c.cats) {
      wx.setStorageSync('fm_cate', c.cats[0]);
      wx.switchTab({ url: '/pages/category/category' });
      return;
    }
    wx.switchTab({ url: store.getCart().length ? '/pages/cart/cart' : '/pages/index/index' });
  }
});
