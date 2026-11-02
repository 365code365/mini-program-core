// 我的：订单概览、收藏的景区、常用游客、导览设置（提示音 / 自动播放下一段）、客服、清缓存
const store = require('../../utils/store.js');
const data = require('../../utils/data.js');
const tab = require('../../utils/tab.js');
Page({
  data: { stat: { paid: 0, used: 0, refunded: 0 }, favs: [], touristCount: 0, set: {}, spent: 0, cache: '' },
  onShow() {
    store.ensureSeed();
    tab.sync(this, 3);
    const os = store.orders();
    const cnt = (s) => os.filter((o) => o.status === s).length;
    const info = wx.getStorageInfoSync();
    this.setData({
      stat: { paid: cnt('paid'), used: cnt('used'), refunded: cnt('refunded') },
      spent: os.filter((o) => o.status !== 'refunded').reduce((s, o) => s + o.total, 0),
      favs: store.favs().map((id) => data.find(id)).filter(Boolean),
      touristCount: store.tourists().length, set: store.settings(), cache: info.currentSize + 'KB'
    });
  },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  goScenic(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  goTourists() { wx.navigateTo({ url: '/pages/tourists/tourists' }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  onChime(e) { this.setData({ set: store.saveSettings({ chime: e.detail.value }) }); },
  onAutoNext(e) { this.setData({ set: store.saveSettings({ autoNext: e.detail.value }) }); },
  call() {
    const tel = data.SERVICE_TEL;
    wx.makePhoneCall({ phoneNumber: tel, fail: () => wx.showModal({ title: '客服电话', content: tel + '（每天 8:00-22:00）', confirmText: '复制号码', success: (r) => { if (r.confirm) wx.setClipboardData({ data: tel }); } }) });
  },
  clear() {
    wx.showModal({
      title: '清除本地数据', content: '订单、常用游客、收藏都会被清空（演示数据会重新生成）。', confirmColor: '#e5484d',
      success: (r) => { if (r.confirm) { wx.clearStorageSync(); wx.showToast({ title: '已清除', icon: 'none' }); this.onShow(); } }
    });
  }
});
