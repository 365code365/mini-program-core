const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const flows = require('../../utils/flows.js');
const host = require('../../utils/host.js');
const pad = v => (v < 10 ? '0' + v : '' + v);
Page({
  data: {
    c: {}, dateText: '', endTime: '', state: 'open', bid: '', canSign: false, left: 0, booked: 0, rank: 0,
    followed: false, coachInitial: '', avatars: ['#ffb38a', '#8ab4ff', '#9be3c3', '#d3a8ff'], steps: [], levels: [1, 2, 3], shop: {}
  },
  onLoad(q) {
    this.day = Number(q.day || 0);
    const list = data.scheduleFor(this.day);
    const c = list.find(x => x.id === (q.id || 'c1')) || list[0];
    const days = data.nextDays(7);
    const [h, m] = c.time.split(':').map(Number);
    const end = h * 60 + m + c.dur;
    const warm = 8, cool = 7;
    this.c = c;
    this.setData({
      c, dateText: days[this.day].week + ' ' + c.dateText, endTime: pad(Math.floor(end / 60)) + ':' + pad(end % 60),
      coachInitial: c.coachInfo.name.split(' ').pop().charAt(0),
      followed: (wx.getStorageSync('fit_follow') || []).indexOf(c.coach) >= 0,
      steps: [{ name: '热身激活', min: warm }, { name: '主训练 · ' + c.type, min: c.dur - warm - cool }, { name: '拉伸放松', min: cool }]
    });
    wx.setNavigationBarTitle({ title: c.name });
  },
  onShow() {
    store.tick();
    this.setData({ shop: store.currentStore() });
    this.refresh();
  },
  refresh() {
    const c = this.c;
    const b = store.bookingOf(c.id, c.date);
    const now = Date.now();
    const mine = b && (b.status === 'booked' || b.status === 'done') ? 1 : 0;
    const state = b ? b.status : (now >= store.startOf(c) ? (now >= store.endOf(c) ? 'ended' : 'started') : (c.left - mine > 0 ? 'open' : 'full'));
    this.setData({ state, bid: b ? b.bid : '', rank: b ? b.rank : 0, canSign: !!b && store.signState(b), left: c.left - mine, booked: c.booked + mine });
  },
  book() {
    const s = this.data.state;
    const b = this.data.bid ? store.findBooking(this.data.bid) : null;
    if (s === 'booked' || s === 'wait') return flows.cancel(b).then(() => this.refresh());
    if (s === 'done' || s === 'miss' || s === 'started' || s === 'ended') {
      wx.showToast({ title: { done: '这节课已上完，去记录里看看', miss: '这节课没有签到，记了 1 次爽约', started: '已开课，下次早点来', ended: '这节课已经结束了' }[s], icon: 'none' });
      return;
    }
    flows.book(this.c).then(() => this.refresh());
  },
  sign() { flows.sign(store.findBooking(this.data.bid)).then(() => this.refresh()); },
  openShop() { host.openMap(this.data.shop); },
  follow() {
    let f = wx.getStorageSync('fit_follow') || [];
    const on = !this.data.followed;
    f = on ? f.concat(this.c.coach) : f.filter(x => x !== this.c.coach);
    wx.setStorageSync('fit_follow', f);
    this.setData({ followed: on });
    wx.showToast({ title: on ? '已关注，' + this.c.coachInfo.name.split(' ')[0] + ' 开新课会提醒你' : '已取消关注', icon: 'none' });
  }
});
