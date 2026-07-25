Page({
  data: {
    tab: 'available',
    coupons: [
      { id: 1, amount: 10, min: 39, name: '新人无门槛券', expire: '2026-12-31', claimed: false },
      { id: 2, amount: 30, min: 99, name: '满99减30券', expire: '2026-12-31', claimed: false },
      { id: 3, amount: 50, min: 199, name: '满199减50券', expire: '2026-12-31', claimed: false },
      { id: 4, amount: 88, min: 399, name: '满399减88券', expire: '2026-12-31', claimed: false }
    ],
    shown: [],
    mineCount: 0
  },

  onLoad: function() {
    this.refresh();
  },

  refresh: function() {
    var tab = this.data.tab;
    var all = this.data.coupons;
    var shown = tab === 'mine'
      ? all.filter(function(c) { return c.claimed; })
      : all.filter(function(c) { return !c.claimed; });
    var mineCount = all.filter(function(c) { return c.claimed; }).length;
    this.setData({ shown: shown, mineCount: mineCount });
  },

  onTab: function(e) {
    this.setData({ tab: e.currentTarget.dataset.tab });
    this.refresh();
  },

  onClaim: function(e) {
    var id = e.currentTarget.dataset.id;
    var coupons = this.data.coupons.map(function(c) {
      if (c.id === id && !c.claimed) { c.claimed = true; }
      return c;
    });
    this.setData({ coupons: coupons });
    this.refresh();
    wx.showToast({ title: '领取成功', icon: 'success' });
  },
  onBack: function () {
    wx.navigateBack();
  },

  onNavHome: function () {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
