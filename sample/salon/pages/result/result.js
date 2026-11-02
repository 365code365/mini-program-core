// 预约结果：对勾用 wx.createAnimation 缩放弹出（export() 赋给 animation 属性），可加到手机日历
const store = require('../../utils/store.js');
const data = require('../../utils/data.js');
Page({
  data: { a: null, sub: false, mode: 'new', anim: null, shop: data.store },
  onLoad(q) {
    this.setData({ a: store.appt(q.id), sub: q.sub === '1', mode: q.mode || 'new' });
    wx.setNavigationBarTitle({ title: q.mode === 'reschedule' ? '改约成功' : '预约成功' });
  },
  onReady() {
    const an = wx.createAnimation({ duration: 260, timingFunction: 'ease-out' });
    an.scale(0.2).opacity(0).step({ duration: 0 });
    an.scale(1.18).opacity(1).step({ duration: 260 });
    an.scale(1).step({ duration: 140 });
    this.setData({ anim: an.export() });
  },
  addCalendar() {
    const a = this.data.a;
    const start = new Date((a.date + ' ' + a.time).replace(/-/g, '/')).getTime();
    wx.addPhoneCalendar({
      title: '型格美业 · ' + a.serviceName, startTime: Math.floor(start / 1000), endTime: Math.floor(start / 1000) + a.duration * 60,
      location: this.data.shop.address, description: '发型师 ' + a.stylistName + '，到店出示核销码 ' + a.code, alarm: true, alarmOffset: 7200,
      success: () => wx.showToast({ title: '已加入日历', icon: 'success' }),
      fail: () => wx.showModal({ title: '无法写入日历', content: '当前环境不支持写入系统日历。已为你在「我的预约」里保留提醒，开始前 2 小时会在首页提示。', showCancel: false })
    });
  },
  goAppts() { wx.redirectTo({ url: '/pages/appointments/appointments' }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
