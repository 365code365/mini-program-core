// 影院页：影院信息（导航 / 电话 / 设为常去）+ 在映影片 + 按天的场次
const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { c: null, movies: [], mid: '', m: null, day: 0, days: [], shows: [], fav: false },
  onLoad(q) {
    const c = D.getCinema(q.id);
    const movies = D.MOVIES.filter(m => m.status === 'now');
    const mid = movies.some(m => m.id === q.mid) ? q.mid : movies[0].id;
    this.setData({ c, movies, days: D.days(), fav: store.favCinema() === c.id });
    wx.setNavigationBarTitle({ title: c.name.replace(/（.*）/, '') });
    this.pickMovie(mid);
  },
  pickMovie(mid) {
    const m = D.getMovie(mid);
    this.setData({ mid, m: Object.assign({}, m, { scoreText: m.score.toFixed(1) }) });
    this.loadShows();
  },
  loadShows() { this.setData({ shows: D.shows(this.data.m, this.data.c, this.data.day) }); },
  onMovie(e) { this.pickMovie(e.currentTarget.dataset.id); },
  onDay(e) { this.setData({ day: Number(e.currentTarget.dataset.idx) }); this.loadShows(); },
  onDetail() { wx.navigateTo({ url: '/pages/movie/movie?id=' + this.data.mid }); },
  onNav() { host.openMap(this.data.c); },
  onCall() { host.call(this.data.c.phone); },
  onFav() {
    if (this.data.fav) { wx.showToast({ title: '已经是常去影院', icon: 'none' }); return; }
    store.setFavCinema(this.data.c.id);
    this.setData({ fav: true });
    wx.showToast({ title: '已设为常去影院', icon: 'none' });
  },
  onShowTap(e) {
    const s = this.data.shows[Number(e.currentTarget.dataset.i)];
    if (s.stop) { wx.showToast({ title: '该场次已停止售票', icon: 'none' }); return; }
    wx.setStorageSync('cinema_pick', { mid: this.data.mid, cid: this.data.c.id, day: this.data.days[this.data.day].label, time: s.time, end: s.end, lang: s.lang, hall: s.hall, price: s.price, ts: s.ts });
    wx.navigateTo({ url: '/pages/seat/seat?mid=' + this.data.mid + '&cid=' + this.data.c.id + '&time=' + s.time });
  }
});
