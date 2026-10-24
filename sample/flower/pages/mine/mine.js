const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { counts: { paid: 0, shipping: 0, done: 0 }, favs: [], remind: true,
    dates: [
      { name: '妈妈生日', date: '11 月 08 日', left: 15, color: '#ff9a3d' },
      { name: '结婚纪念日', date: '12 月 20 日', left: 57, color: '#ef4f76' }
    ] },
  onShow() {
    const counts = { paid: 0, shipping: 0, done: 0 };
    store.getOrders().forEach(o => { if (counts[o.status] !== undefined) counts[o.status]++; });
    const favs = (wx.getStorageSync('fl_fav') || []).map(data.productOf).filter(Boolean);
    this.setData({ counts, favs, remind: wx.getStorageSync('fl_remind') !== false });
  },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  goDetail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onRemind(e) {
    wx.setStorageSync('fl_remind', e.detail.value);
    wx.showToast({ title: e.detail.value ? '纪念日前 3 天提醒你订花' : '已关闭纪念日提醒', icon: 'none' });
  },
  addDate() { wx.showToast({ title: '纪念日添加功能即将上线', icon: 'none' }); },
  menu(e) {
    const k = e.currentTarget.dataset.k;
    if (k === 'addr') wx.showModal({ title: '常用地址', content: '苏晚晴 139****2086\n上海市徐汇区漕溪北路 88 号 圣爱大厦 12 楼', showCancel: false });
    else if (k === 'coupon') wx.showModal({ title: '我的优惠券', content: '满 199 减 20（10 月 31 日到期）\n满 299 减 40（11 月 30 日到期）', showCancel: false });
    else if (k === 'vip') wx.showToast({ title: '再消费 ¥320 升级金卡会员', icon: 'none' });
    else wx.showActionSheet({ itemList: ['在线客服', '拨打 400-660-5520'], success: r => wx.showToast({ title: r.tapIndex ? '正在呼叫…' : '花艺师马上为您服务', icon: 'none' }) });
  }
});
