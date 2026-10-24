const app = getApp();
const BGS = ['linear-gradient(135deg, #FF9A8B, #E8243F)', 'linear-gradient(135deg, #8EC5FF, #3D7BFF)', 'linear-gradient(135deg, #FFD36B, #F08A24)', 'linear-gradient(135deg, #8DE0B9, #1E9E6A)', 'linear-gradient(135deg, #C7B2FF, #6A4BD8)', 'linear-gradient(135deg, #FFB3D1, #D9407A)'];
const DAYS = ['今天', '明天', '后天'];
Page({
  data: { m: null, expand: false, day: 0, cinemas: [], wished: false },
  onLoad(q) {
    const m = app.getMovie(q.id);
    const cast = m.actors.map((a, i) => ({ name: a[0], role: a[1], glyph: a[0].charAt(0), bg: BGS[i % BGS.length] }));
    const now = new Date();
    const days = DAYS.map((label, i) => {
      const d = new Date(now.getTime() + i * 86400000);
      return { label: label + ' ' + (d.getMonth() + 1) + '月' + d.getDate() + '日' };
    });
    const s = m.score;
    const five = Math.round((s - 7) * 30);
    const dist = [{ star: 5, pct: five }, { star: 4, pct: Math.round((100 - five) * 0.65) }, { star: 3, pct: Math.round((100 - five) * 0.22) }, { star: 2, pct: Math.round((100 - five) * 0.08) }];
    this.setData({
      m, cast, days, dist, directorGlyph: m.director.charAt(0),
      ratingCount: (m.wish * 3.2 / 10000).toFixed(1) + '万',
      wishText: (m.wish / 10000).toFixed(1) + '万'
    });
    wx.setNavigationBarTitle({ title: m.title });
    this.loadShows();
  },
  onShow() { this.setData({ wished: (wx.getStorageSync('cinema_wish') || []).indexOf(this.data.m.id) >= 0 }); },
  loadShows() {
    const cinemas = app.CINEMAS.map(c => Object.assign({}, c, { shows: app.shows(this.data.m, c, this.data.day) }));
    this.setData({ cinemas });
  },
  onDay(e) { this.setData({ day: e.currentTarget.dataset.idx }); this.loadShows(); },
  onExpand() { this.setData({ expand: !this.data.expand }); },
  onWish() {
    const id = this.data.m.id;
    let w = wx.getStorageSync('cinema_wish') || [];
    const on = w.indexOf(id) < 0;
    w = on ? w.concat([id]) : w.filter(x => x !== id);
    wx.setStorageSync('cinema_wish', w);
    this.setData({ wished: on });
    wx.showToast({ title: on ? '已标记想看' : '已取消', icon: 'none' });
  },
  onRate() {
    if (this.data.m.status !== 'now') { wx.showToast({ title: '开售后提醒你', icon: 'none' }); return; }
    wx.showActionSheet({ itemList: ['★★★★★ 神作', '★★★★ 推荐', '★★★ 还行', '★★ 较差'], success: () => wx.showToast({ title: '评分成功', icon: 'success' }) });
  },
  onPickShow(e) {
    const { cid, time } = e.currentTarget.dataset;
    const c = this.data.cinemas.find(x => x.id === cid);
    const show = c.shows.find(x => x.time === time);
    wx.setStorageSync('cinema_pick', { mid: this.data.m.id, cid, day: this.data.days[this.data.day].label, time: show.time, end: show.end, lang: show.lang, hall: show.hall, price: show.price });
    wx.navigateTo({ url: '/pages/seat/seat?mid=' + this.data.m.id + '&cid=' + cid + '&time=' + show.time });
  }
});
