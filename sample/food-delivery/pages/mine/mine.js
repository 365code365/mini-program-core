const cart = require('../../utils/cart.js');

Page({
  data: {
    saved: 46.5,
    stats: [
      { name: '红包', value: 6 },
      { name: '优惠券', value: 3 },
      { name: '余额', value: '¥88' },
      { name: '吃货豆', value: 1260 }
    ],
    orderEntries: [
      { name: '待付款', glyph: '付', color: 'linear-gradient(135deg, #ffb347, #ff7a1a)', badge: 0 },
      { name: '进行中', glyph: '送', color: 'linear-gradient(135deg, #7cc6ff, #2a7fff)', badge: 0 },
      { name: '待评价', glyph: '评', color: 'linear-gradient(135deg, #ffa6c1, #e8487c)', badge: 0 },
      { name: '退款/售后', glyph: '退', color: 'linear-gradient(135deg, #c8ced8, #7a8494)', badge: 0 }
    ],
    menus: [
      { name: '收货地址', glyph: '址', color: '#1677ff', extra: '3 个' },
      { name: '我的收藏', glyph: '藏', color: '#ff8a00', extra: '' },
      { name: '我的评价', glyph: '评', color: '#e8487c', extra: '' },
      { name: '邀请好友得红包', glyph: '邀', color: '#ff4d2d', extra: '最高 ¥20' },
      { name: '客服中心', glyph: '服', color: '#5a6af0', extra: '7×24 小时' }
    ],
    notify: true
  },

  onShow() {
    const orders = cart.getOrders();
    const going = orders.filter(o => o.status === 'delivering').length;
    const done = orders.filter(o => o.status === 'done').length;
    const favs = (wx.getStorageSync('fd_favs') || []).length;
    this.setData({
      'orderEntries[1].badge': going,
      'orderEntries[2].badge': done,
      'menus[1].extra': favs ? favs + ' 家' : '',
      notify: wx.getStorageSync('fd_notify') !== false
    });
  },

  onStat(e) {
    wx.showToast({ title: `${e.currentTarget.dataset.name}明细 即将开放`, icon: 'none' });
  },

  onVip() {
    wx.switchTab({ url: '/pages/index/index' });
  },

  onOrders() {
    wx.switchTab({ url: '/pages/orders/orders' });
  },

  onMenu(e) {
    const name = e.currentTarget.dataset.name;
    if (name === '客服中心') {
      wx.showModal({ title: '客服中心', content: '客服热线 400-820-8820，在线客服 7×24 小时为你服务', showCancel: false });
      return;
    }
    wx.showToast({ title: name, icon: 'none' });
  },

  onNotify(e) {
    wx.setStorageSync('fd_notify', e.detail.value);
    this.setData({ notify: e.detail.value });
  },

  onSetting() {
    wx.showActionSheet({ itemList: ['账号与安全', '隐私设置', '清除缓存', '关于饱了么'] });
  }
});
