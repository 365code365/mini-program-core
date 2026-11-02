const store = require('../../utils/store.js');
Page({
  data: { tabs: [], tab: 'valid', list: [] },
  onShow() { this.load(); },
  load() {
    const all = store.allCoupons();
    const n = k => all.filter(c => c.status === k).length;
    this.setData({
      tabs: [{ k: 'valid', n: '可使用(' + n('valid') + ')' }, { k: 'used', n: '已使用(' + n('used') + ')' }, { k: 'expired', n: '已过期(' + n('expired') + ')' }],
      list: all.filter(c => c.status === this.data.tab)
    });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.load(); },
  // 下单时自动挑最省的一张；钟点房券直接把搜索切到钟点房
  onUse(e) {
    const c = this.data.list.find(x => x.id === String(e.currentTarget.dataset.id));
    const s = store.getSearch();
    s.kind = c && c.hourly ? 2 : (c && c.type === '民宿' ? 1 : 0);
    if (s.kind === 1) s.star = 5;
    else if (s.star === 5) s.star = 0;
    store.saveSearch(s);
    wx.switchTab({ url: '/pages/index/index' });
  }
});
