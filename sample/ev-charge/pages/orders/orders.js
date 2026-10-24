const data = require('../../utils/data.js');

Page({
  data: { tabs: ['全部', '本月', '更早'], tIdx: 0, all: [], shown: [], session: null, sum: { count: 0, kwh: '0', co2: '0' } },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    const all = data.getOrders().map(o => Object.assign({}, o, { stationName: data.getStation(o.station).name, feeText: o.fee.toFixed(2) }));
    const kwh = all.reduce((a, o) => a + o.kwh, 0);
    const s = wx.getStorageSync('ev_session');
    this.setData({
      all,
      session: s ? { no: s.no, stationName: data.getStation(s.station).name } : null,
      // 电动车比燃油车每度电约少排 0.6kg CO₂（演示估算）
      sum: { count: all.length, kwh: kwh.toFixed(1), co2: (kwh * 0.6).toFixed(1) }
    });
    this.apply();
  },
  apply() {
    const t = this.data.tIdx;
    // 「本月」= 10 月（演示数据都在 10 月；新下的单 date 是当月）
    const month = (new Date().getMonth() + 1);
    const mm = month < 10 ? '0' + month : '' + month;
    const shown = this.data.all.filter(o => t === 0 || (t === 1 ? o.date.indexOf(mm + '-') === 0 : o.date.indexOf(mm + '-') !== 0));
    this.setData({ shown });
  },
  onTab(e) { this.setData({ tIdx: e.currentTarget.dataset.idx }); this.apply(); },
  onSession() { wx.navigateTo({ url: '/pages/charging/charging' }); },
  onOrder(e) {
    const o = this.data.shown[e.currentTarget.dataset.idx];
    wx.showActionSheet({ itemList: ['开具电子发票', '再次前往该电站', '联系客服'], success: (r) => {
      if (r.tapIndex === 1) wx.navigateTo({ url: '/pages/station/station?id=' + o.station });
      else wx.showToast({ title: r.tapIndex === 0 ? '发票将发送至绑定邮箱' : '客服电话 400-820-1234', icon: 'none' });
    } });
  }
});
