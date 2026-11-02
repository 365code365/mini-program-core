// 景区详情：宣传片（video + VideoContext 播放 / 暂停 / 跳章节）、开放时间、门票、须知、评价
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const app = getApp();
const CHAPTERS = [{ t: 0, name: '开场' }, { t: 2, name: '全景' }, { t: 4, name: '片尾' }];

function mmss(s) { s = Math.max(0, Math.floor(s || 0)); return (s < 600 ? '0' : '') + Math.floor(s / 60) + ':' + (s % 60 < 10 ? '0' : '') + (s % 60); }

Page({
  data: { s: null, fav: false, playing: false, ended: false, cur: '00:00', dur: '00:05', pct: 0, chapters: CHAPTERS, chapter: 0, from: 0, reviews: [] },
  onLoad(q) {
    const s = data.find(q && q.id);
    app.setScenic(s.id);
    wx.setNavigationBarTitle({ title: s.short });
    const reviews = s.reviews.map((r) => Object.assign({}, r, { stars: '★★★★★'.slice(0, r.score) + '☆☆☆☆☆'.slice(0, 5 - r.score), av: r.user.slice(0, 1) }));
    this.setData({ s, fav: store.isFav(s.id), from: Math.min(...s.tickets.map((t) => t.price)), reviews });
  },
  onReady() { this.video = wx.createVideoContext('promo', this); },
  onHide() { if (this.video && this.data.playing) this.video.pause(); },
  onPlayToggle() {
    if (!this.video) return;
    if (this.data.ended) { this.video.seek(0); this.video.play(); return; }
    if (this.data.playing) this.video.pause(); else this.video.play();
  },
  onChapter(e) {
    const c = CHAPTERS[+e.currentTarget.dataset.i];
    this.video.seek(c.t);
    this.video.play();
    this.setData({ chapter: +e.currentTarget.dataset.i, ended: false });
  },
  onPlay() { this.setData({ playing: true, ended: false }); },
  onPause() { this.setData({ playing: false }); },
  onTime(e) {
    const { currentTime, duration } = e.detail;
    let ch = 0;
    CHAPTERS.forEach((c, i) => { if (currentTime >= c.t) ch = i; });
    // timeupdate 一秒好几次：秒数没变就不 setData
    const cur = mmss(currentTime);
    if (cur === this.data.cur && ch === this.data.chapter) return;
    this.setData({ cur, dur: mmss(duration), pct: duration ? Math.round(currentTime * 100 / duration) : 0, chapter: ch });
  },
  onEnded() { this.setData({ playing: false, ended: true, pct: 100 }); },
  onVideoError(e) { wx.showToast({ title: '宣传片加载失败', icon: 'none' }); console.log('video error', e.detail && e.detail.errMsg); },
  onFav() {
    const on = store.toggleFav(this.data.s.id);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
  },
  onAddr() {
    const s = this.data.s, p = s.routes[0].stops[0];
    wx.openLocation({
      latitude: p[1], longitude: p[2], name: s.name, address: s.addr,
      fail: () => wx.setClipboardData({ data: s.addr, success: () => wx.showToast({ title: '地址已复制，可粘贴到地图 App', icon: 'none' }) })
    });
  },
  onShareAppMessage() { return { title: this.data.s.name + ' · ' + this.data.s.slogan, path: '/pages/detail/detail?id=' + this.data.s.id }; },
  goBook(e) {
    const t = e.currentTarget.dataset.t || '';
    wx.navigateTo({ url: '/pages/booking/booking?id=' + this.data.s.id + (t ? '&t=' + t : '') });
  },
  goGuide() { app.setScenic(this.data.s.id); wx.switchTab({ url: '/pages/guide/guide' }); },
  goRoute() { wx.navigateTo({ url: '/pages/route/route?id=' + this.data.s.id }); }
});
