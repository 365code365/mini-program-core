const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { courseCount: 0, hours: 0, streak: 0, best: 0, favs: [], remind: true, remindTime: '20:30' },
  onShow() {
    const mine = store.myCourses();
    const favs = store.getFav().map(id => data.courseOf(id)).filter(Boolean).map(c => ({ id: c.id, title: c.title, cover: c.cover, mark: c.mark }));
    this.setData({
      courseCount: mine.length,
      hours: (mine.reduce((s, x) => s + x.done * 21, 0) / 60).toFixed(1),
      streak: store.streak(store.getCheckins()),
      best: wx.getStorageSync('zx_exam_best') || 0,
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
  orders() { wx.showModal({ title: '我的订单', content: 'Python 从入门到实战 ¥199（2024-09-02）\n职场高效沟通与汇报技巧 ¥99（2024-08-15）', showCancel: false }); },
  cert() { wx.showToast({ title: '学完任意一门课即可领取证书', icon: 'none' }); },
  service() {
    wx.showActionSheet({ itemList: ['在线客服', '拨打 400-820-6688', '意见反馈'], success: r => {
      wx.showToast({ title: ['客服马上就来', '正在呼叫…', '感谢你的反馈'][r.tapIndex], icon: 'none' });
    } });
  },
  clear() {
    wx.showModal({ title: '清除学习数据', content: '将重置学习进度、收藏和打卡记录，确定吗？', confirmColor: '#f0577a', success: r => {
      if (!r.confirm) return;
      ['zx_learn', 'zx_fav', 'zx_checkin', 'zx_today', 'zx_exam_best', 'zx_exam_today'].forEach(k => wx.removeStorageSync(k));
      this.onShow();
      wx.showToast({ title: '已恢复初始数据', icon: 'none' });
    } });
  }
});
