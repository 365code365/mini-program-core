const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: { c: {}, dateText: '', endTime: '', mine: false, followed: false, coachInitial: '', avatars: ['#ffb38a', '#8ab4ff', '#9be3c3', '#d3a8ff'], steps: [] },
  onLoad(q) {
    const day = Number(q.day || 0);
    const c = data.scheduleFor(day).find(x => x.id === (q.id || 'c1')) || data.scheduleFor(day)[0];
    const days = data.nextDays(7);
    c.dateText = days[day].key;
    const [h, m] = c.time.split(':').map(Number);
    const end = h * 60 + m + c.dur;
    const pad = v => (v < 10 ? '0' + v : '' + v);
    const warm = 8, cool = 7;
    this.c = c;
    this.setData({
      c, dateText: days[day].week + ' ' + c.dateText, endTime: pad(Math.floor(end / 60)) + ':' + pad(end % 60),
      coachInitial: c.coachInfo.name.split(' ').pop().charAt(0),
      followed: (wx.getStorageSync('fit_follow') || []).indexOf(c.coach) >= 0,
      steps: [{ name: '热身激活', min: warm }, { name: '主训练 · ' + c.type, min: c.dur - warm - cool }, { name: '拉伸放松', min: cool }]
    });
    wx.setNavigationBarTitle({ title: c.name });
    this.refresh();
  },
  refresh() { this.setData({ mine: store.isBooked(this.c.id, this.c.dayIdx) }); },
  book() {
    const c = this.c;
    if (!this.data.mine && !c.left) {
      wx.showToast({ title: '已加入候补', icon: 'none' });
      return;
    }
    const on = store.toggleBooking(c);
    this.refresh();
    wx.showToast({ title: on ? '预约成功' : '已取消预约', icon: on ? 'success' : 'none' });
  },
  follow() {
    let f = wx.getStorageSync('fit_follow') || [];
    const on = !this.data.followed;
    f = on ? f.concat(this.c.coach) : f.filter(x => x !== this.c.coach);
    wx.setStorageSync('fit_follow', f);
    this.setData({ followed: on });
  }
});
