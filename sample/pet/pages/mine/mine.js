const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

Page({
  data: {
    pets: data.pets, bookings: [], cartCount: 0, favCount: 0, remind: true,
    orderTabs: [
      { n: '待付款', ch: '付', bg: 'linear-gradient(135deg, #ffcf6b, #ffa53d)', c: 0 },
      { n: '待发货', ch: '发', bg: 'linear-gradient(135deg, #7ec8ff, #3b9cf0)', c: 0 },
      { n: '待收货', ch: '收', bg: 'linear-gradient(135deg, #7fe0b0, #2fb57a)', c: 1 },
      { n: '评价', ch: '评', bg: 'linear-gradient(135deg, #ff9fb5, #f2557a)', c: 0 }
    ]
  },
  onShow() {
    const bookings = (wx.getStorageSync('pet_bookings') || []).map(b => {
      const p = b.date.split('-');
      return Object.assign({}, b, { dayNum: +p[2], monthText: (+p[1]) + '月' });
    });
    const paid = (wx.getStorageSync('pet_orders') || []).length;
    this.setData({
      bookings, cartCount: cart.count(), favCount: (wx.getStorageSync('pet_favs') || []).length,
      remind: wx.getStorageSync('pet_remind') !== false,
      'orderTabs[1].c': paid
    });
  },
  cancel(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '取消预约', content: '确定取消这次预约吗？', confirmColor: '#ff5b35',
      success: (r) => {
        if (!r.confirm) return;
        wx.setStorageSync('pet_bookings', (wx.getStorageSync('pet_bookings') || []).filter(b => b.id !== id));
        this.onShow();
      }
    });
  },
  onRemind(e) { wx.setStorageSync('pet_remind', e.detail.value); },
  goPet() { wx.switchTab({ url: '/pages/pet/pet' }); },
  goCart() { wx.navigateTo({ url: '/pages/cart/cart' }); },
  goBooking() { wx.navigateTo({ url: '/pages/booking/booking' }); },
  orderTip() { wx.showToast({ title: '订单功能演示版暂未开放', icon: 'none' }); },
  contact() { wx.showModal({ title: '萌宠家客服', content: '400-826-2031（9:00-22:00）', confirmText: '拨打' }); }
});
