const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
const BGS = ['linear-gradient(135deg, #FF9A8B, #E8243F)', 'linear-gradient(135deg, #8EC5FF, #3D7BFF)', 'linear-gradient(135deg, #FFD36B, #F08A24)', 'linear-gradient(135deg, #8DE0B9, #1E9E6A)', 'linear-gradient(135deg, #C7B2FF, #6A4BD8)', 'linear-gradient(135deg, #FFB3D1, #D9407A)'];
Page({
  data: { starList: [1, 2, 3, 4, 5], m: null, expand: false, day: 0, cinemas: [], wished: false, reviews: [], myCount: 0 },
  onLoad(q) {
    const m = D.getMovie(q.id);
    const cast = m.actors.map((a, i) => ({ name: a[0], role: a[1], glyph: a[0].charAt(0), bg: BGS[i % BGS.length] }));
    const s = m.score;
    const five = Math.round((s - 7) * 30);
    const dist = [{ star: 5, pct: five }, { star: 4, pct: Math.round((100 - five) * 0.65) }, { star: 3, pct: Math.round((100 - five) * 0.22) }, { star: 2, pct: Math.round((100 - five) * 0.08) }];
    this.setData({
      m, cast, days: D.days(), dist, directorGlyph: m.director.charAt(0),
      ratingCount: (m.wish * 3.2 / 10000).toFixed(1) + '万',
      wishText: (m.wish / 10000).toFixed(1) + '万'
    });
    wx.setNavigationBarTitle({ title: m.title });
    this.loadShows();
    // 从列表上的「购票」进来：直接滚到场次
    if (q.buy && m.status === 'now') setTimeout(() => wx.pageScrollTo({ selector: '.shows-card', duration: 0 }), 50);
  },
  onShow() {
    this.setData({ wished: store.isWished(this.data.m.id) });
    this.loadReviews();
  },
  loadShows() {
    const cinemas = D.CINEMAS.map(c => Object.assign({}, c, { shows: D.shows(this.data.m, c, this.data.day) }));
    this.setData({ cinemas });
  },
  // 我写的影评排在前面，后面是热门短评
  loadReviews() {
    const mid = this.data.m.id;
    const mine = store.reviews().filter(r => r.mid === mid).map(r => ({ id: r.id, name: '我', glyph: '我', mine: true, star: r.star, text: r.text, tags: r.tags || [], likes: r.likes }));
    const seed = this.data.m.status === 'now' ? D.SEED_REVIEWS.map((r, i) => Object.assign({ id: 'seed' + i, tags: [], glyph: r.name.charAt(0), bg: BGS[(i + 1) % BGS.length] }, r)) : [];
    this.setData({ reviews: mine.concat(seed), myCount: mine.length });
  },
  onDay(e) { this.setData({ day: Number(e.currentTarget.dataset.idx) }); this.loadShows(); },
  onExpand() { this.setData({ expand: !this.data.expand }); },
  onWish() {
    const on = store.toggleWish(this.data.m.id);
    this.setData({ wished: on });
    wx.showToast({ title: on ? '已标记想看' : '已取消', icon: 'none' });
  },
  // 「看过」/「写影评」：上映了才能写；写过的进去是编辑
  onRate() {
    if (this.data.m.status !== 'now') {
      if (!this.data.wished) this.onWish();
      else wx.showToast({ title: '已预约，开售后提醒你', icon: 'none' });
      return;
    }
    const mine = store.reviews().find(r => r.mid === this.data.m.id);
    wx.navigateTo({ url: '/pages/review/review?mid=' + this.data.m.id + (mine ? '&id=' + mine.id : '') });
  },
  onEditReview(e) { wx.navigateTo({ url: '/pages/review/review?mid=' + this.data.m.id + '&id=' + e.currentTarget.dataset.id }); },
  onLike(e) {
    const i = Number(e.currentTarget.dataset.i);
    const r = this.data.reviews[i];
    this.setData({ [`reviews[${i}].liked`]: !r.liked, [`reviews[${i}].likes`]: r.likes + (r.liked ? -1 : 1) });
  },
  onCinema(e) { wx.navigateTo({ url: '/pages/cinema/cinema?id=' + e.currentTarget.dataset.cid + '&mid=' + this.data.m.id }); },
  onPickShow(e) {
    const { cid, time } = e.currentTarget.dataset;
    const c = this.data.cinemas.find(x => x.id === cid);
    const show = c.shows.find(x => x.time === time);
    if (show.stop) { wx.showToast({ title: '该场次已停止售票', icon: 'none' }); return; }
    wx.setStorageSync('cinema_pick', { mid: this.data.m.id, cid, day: this.data.days[this.data.day].label, time: show.time, end: show.end, lang: show.lang, hall: show.hall, price: show.price, ts: show.ts });
    wx.navigateTo({ url: '/pages/seat/seat?mid=' + this.data.m.id + '&cid=' + cid + '&time=' + show.time });
  }
});
