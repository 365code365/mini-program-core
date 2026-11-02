const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { courseCount: 0, hours: 0, streak: 0, best: 0, favs: [], remind: true, remindTime: '20:30', orderCount: 0, couponCount: 0, points: 0, certCount: 0, noteCount: 0 },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3 });
    const mine = store.myCourses();
    const favs = store.getFav().map(id => data.courseOf(id)).filter(Boolean).map(c => ({ id: c.id, title: c.title, cover: c.cover, mark: c.mark }));
    this.setData({
      courseCount: mine.length,
      hours: (store.learnedMins() / 60).toFixed(1),
      orderCount: store.getOrders().length,
      couponCount: store.getCoupons().filter(q => !q.used).length,
      points: store.getPoints(),
      certCount: mine.filter(x => x.finished).length,
      noteCount: store.getNotes().length,
      streak: store.streak(store.getCheckins()),
      best: Number(wx.getStorageSync('zx_exam_best')) || 0,
      favs,
      remind: wx.getStorageSync('zx_remind') !== false,
      remindTime: wx.getStorageSync('zx_remind_time') || '20:30'
    });
  },
  onRemind(e) {
    wx.setStorageSync('zx_remind', e.detail.value);
    this.setData({ remind: e.detail.value });
    wx.showToast({ title: e.detail.value ? '每天 ' + this.data.remindTime + ' 提醒你学习' : '已关闭学习提醒', icon: 'none' });
  },
  onTime(e) {
    wx.setStorageSync('zx_remind_time', e.detail.value);
    this.setData({ remindTime: e.detail.value });
  },
  goDetail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  goLearn() { wx.switchTab({ url: '/pages/learn/learn' }); },
  goCheckin() { wx.switchTab({ url: '/pages/checkin/checkin' }); },
  goExam() { wx.navigateTo({ url: '/pages/exam/exam' }); },
  orders() { wx.navigateTo({ url: '/pages/orders/orders' }); },
  cert() { wx.navigateTo({ url: '/pages/cert/cert' }); },
  notes() { wx.navigateTo({ url: '/pages/notes/notes' }); },
  coupons() { wx.navigateTo({ url: '/pages/coupons/coupons' }); },
  service() {
    wx.showActionSheet({ itemList: ['拨打客服 400-820-6688', '复制客服微信号', '常见问题'], success: r => {
      if (r.tapIndex === 0) host.call('400-820-6688');
      else if (r.tapIndex === 1) host.copy('zhixue_kf01', '微信号已复制');
      else wx.showModal({ title: '常见问题', content: '课程有效期：购买后永久有效\n退款：购买 7 天内、付费课时学习不足 3 节可在「我的订单」申请\n证书：学完全部课时自动发放', showCancel: false, confirmText: '知道了' });
    } });
  },
  clear() {
    wx.showModal({ title: '清除学习数据', content: '将重置学习进度、订单、优惠券、笔记、收藏和打卡记录，确定吗？', confirmColor: '#f0577a', success: r => {
      if (!r.confirm) return;
      ['zx_learn', 'zx_fav', 'zx_checkin', 'zx_today', 'zx_exam_best', 'zx_exam_today', 'zx_orders', 'zx_coupons', 'zx_points', 'zx_notes', 'zx_reviews', 'zx_follow', 'zx_learn_day', 'zx_task_claim'].forEach(k => wx.removeStorageSync(k));
      this.onShow();
      wx.showToast({ title: '已恢复初始数据', icon: 'none' });
    } });
  }
});
