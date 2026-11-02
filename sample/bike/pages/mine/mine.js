// 个人中心：骑行统计、常用入口、客服电话（拨号失败复制号码）、震动 / 屏幕常亮设置
const store = require('../../utils/store.js');

Page({
  data: { stat: null, balance: '0.00', cardTx: '', faults: 0, settings: {} },

  onShow() {
    const trips = store.trips();
    const km = trips.reduce((s, t) => s + t.meters, 0) / 1000;
    const sec = trips.reduce((s, t) => s + t.sec, 0);
    const w = store.wallet();
    const now = Date.now();
    const cards = w.cards.filter(c => c.expireAt > now);
    this.setData({
      stat: { count: trips.length, km: km.toFixed(1), hours: (sec / 3600).toFixed(1), carbon: (km * 0.15).toFixed(1) },
      balance: w.balance.toFixed(2),
      cardTx: cards.length ? cards.map(c => c.name).join('、') : '未购买',
      faults: store.faults().filter(f => f.status !== 'done').length,
      settings: store.settings()
    });
  },

  go(e) { wx.navigateTo({ url: e.currentTarget.dataset.url }); },

  onSwitch(e) {
    const s = Object.assign({}, this.data.settings, { [e.currentTarget.dataset.k]: e.detail.value });
    store.saveSettings(s);
    this.setData({ settings: s });
    if (e.currentTarget.dataset.k === 'vibrate' && e.detail.value) wx.vibrateShort({ type: 'medium' });
  },

  onService() {
    wx.makePhoneCall({
      phoneNumber: '4000000999',
      fail: () => wx.setClipboardData({
        data: '400-000-0999',
        success: () => wx.showToast({ title: '客服电话已复制', icon: 'none' })
      })
    });
  }
});
