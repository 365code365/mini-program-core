const store = require('../../utils/store.js');

Page({
  data: { height: 176, weight: 72.5, bmi: '', bmiText: '', bookings: [], remind: true },
  onShow() {
    const w = wx.getStorageSync('fit_weight') || 72.5;
    this.setWeight(w);
    this.setData({ bookings: store.getBookings(), remind: wx.getStorageSync('fit_remind') !== false });
  },
  setWeight(w) {
    const bmi = w / Math.pow(this.data.height / 100, 2);
    this.setData({ weight: w, bmi: bmi.toFixed(1), bmiText: bmi < 18.5 ? '偏瘦' : (bmi < 24 ? '正常' : (bmi < 28 ? '超重' : '肥胖')) });
  },
  onWeight(e) {
    wx.setStorageSync('fit_weight', e.detail.value);
    this.setWeight(e.detail.value);
    wx.showToast({ title: '体重已记录', icon: 'success' });
  },
  onRemind(e) { wx.setStorageSync('fit_remind', e.detail.value); },
  goCard() { wx.navigateTo({ url: '/pages/card/card' }); },
  goClasses() { wx.switchTab({ url: '/pages/classes/classes' }); },
  editProfile() {
    wx.showActionSheet({ itemList: ['更换头像', '修改昵称', '训练目标：减脂塑形'], success: () => wx.showToast({ title: '已保存', icon: 'none' }) });
  },
  contact() { wx.showModal({ title: '燃动 · 科技园店', content: '0755-8666 2031\n营业时间 06:30-23:00', confirmText: '拨打' }); }
});
