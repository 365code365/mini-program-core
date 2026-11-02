// 我的：预约记录（按状态筛）、收藏 / 足迹 / 已报名活动统计、联系服务台、清除本地数据
const { WEEKLY } = require('../../utils/data.js');
const U = require('../../utils/util.js');

const FILTERS = ['全部', '待使用', '已取消', '已过期'];
const ALL_EV = [].concat(...Object.keys(WEEKLY).map(k => WEEKLY[k]));

Page({
  data: { filters: FILTERS, f: 0, bookings: [], shown: [], stats: {}, events: [] },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3 });
    const bookings = U.load('bookings', []);
    const events = U.load('events', []).map(k => {
      const [id, date] = k.split('@');
      const ev = ALL_EV.find(e => e.id === id);
      return ev ? { k, date, time: ev.time, title: ev.title, place: ev.place } : null;
    }).filter(Boolean).sort((a, b) => (a.date + a.time < b.date + b.time ? -1 : 1));
    this.setData({
      bookings, events,
      stats: { book: bookings.filter(b => b.status === '待使用').length, fav: U.load('fav', []).length, his: U.load('history', []).length }
    });
    this.filter();
  },
  filter() {
    const name = FILTERS[this.data.f];
    this.setData({ shown: this.data.bookings.filter(b => name === '全部' || b.status === name) });
  },
  onF(e) { this.setData({ f: +e.currentTarget.dataset.i }); this.filter(); },
  openTicket(e) { wx.navigateTo({ url: '/pages/ticket/ticket?id=' + e.currentTarget.dataset.id }); },
  openFav() { wx.navigateTo({ url: '/pages/favorites/favorites' }); },
  openHis() { wx.navigateTo({ url: '/pages/favorites/favorites?tab=1' }); },
  openCal() { wx.switchTab({ url: '/pages/calendar/calendar' }); },
  book() { wx.navigateTo({ url: '/pages/booking/booking' }); },
  call() {
    wx.makePhoneCall({
      phoneNumber: '0571-88886666',
      fail: () => wx.showModal({
        title: '服务台电话', content: '0571-88886666（开放日 09:00-17:00）', confirmText: '复制号码',
        success: r => { if (r.confirm) wx.setClipboardData({ data: '0571-88886666' }); }
      })
    });
  },
  clearAll() {
    wx.showModal({
      title: '清除本地数据', content: '将清除收藏、足迹与活动报名（预约记录保留），确定吗？', confirmText: '清除', confirmColor: '#c0392b',
      success: r => {
        if (!r.confirm) return;
        ['fav', 'history', 'events'].forEach(k => U.save(k, []));
        this.onShow();
        wx.showToast({ title: '已清除', icon: 'success' });
      }
    });
  }
});
