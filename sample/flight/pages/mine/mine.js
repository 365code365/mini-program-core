const store = require('../../utils/store');
const app = getApp();

Page({
  data: { name: '', legs: 0, km: 0, paxCount: 0, hisCount: 0, level: '' },
  onShow() {
    app.syncTab(this, 2);
    const done = store.orders().filter(o => o.status === 'paid' || o.status === 'changed');
    let legs = 0, km = 0;
    done.forEach(o => o.legs.forEach(l => { legs += 1; km += l.flight.dist || 0; }));
    const self = store.passengers().find(p => p.self);
    this.setData({
      name: self ? self.name : '云翼旅客', legs, km, paxCount: store.passengers().length, hisCount: store.history().length,
      level: km >= 20000 ? '金卡会员' : km >= 5000 ? '银卡会员' : '普卡会员'
    });
  },
  goPax() { wx.navigateTo({ url: '/pages/passengers/passengers' }); },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  clearHistory() {
    if (!this.data.hisCount) return wx.showToast({ title: '没有搜索历史', icon: 'none' });
    wx.showModal({ title: '清除搜索历史', content: '共 ' + this.data.hisCount + ' 条，确定清除吗？', success: (r) => { if (r.confirm) { store.clearHistory(); this.setData({ hisCount: 0 }); } } });
  },
  call() {
    wx.makePhoneCall({ phoneNumber: '4008207788', fail: () => wx.setClipboardData({ data: '400-820-7788', success: () => wx.showToast({ title: '客服电话已复制', icon: 'none' }) }) });
  },
  about() {
    wx.showModal({ title: '关于云翼机票', content: '云翼机票 v2.6.0\n航班数据为演示数据，航司为虚构，机场三字码真实。', showCancel: false });
  }
});
