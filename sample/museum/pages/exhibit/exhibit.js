// 展品详情：大图（点开 root-portal 全屏看图层，长按 show-menu-by-longpress，原图走 wx.previewImage）、
// rich-text 介绍、可选中复制的释文（text selectable / user-select）、InnerAudioContext 讲解（进度、倍速）、收藏。
// page-meta 的 scroll-top 只在值变化时滚动，所以「回到顶部」在 '0px' 与 '0rpx' 之间交替赋值
const { HALLS } = require('../../utils/data.js');
const U = require('../../utils/util.js');

function fmt(s) { s = Math.max(0, Math.floor(s || 0)); return U.pad2(Math.floor(s / 60)) + ':' + U.pad2(s % 60); }

Page({
  data: {
    ex: null, faved: false, related: [], gallery: [],
    viewer: false, vIdx: 0, scrollTop: '', showTop: false,
    playing: false, pct: 0, cur: '00:00', dur: '00:05', rate: 1, rates: [0.75, 1, 1.25, 1.5]
  },
  onLoad(q) {
    const ex = U.exhibit(q.id) || U.exhibit('e1');
    const hall = HALLS.find(h => h.id === ex.hall);
    const gallery = hall.exhibits.map(id => U.exhibit(id));
    this.setData({
      ex, gallery, vIdx: Math.max(0, hall.exhibits.indexOf(ex.id)),
      related: HALLS.filter(h => h.id !== ex.hall).slice(0, 3).map(h => U.exhibit(h.exhibits[0]))
    });
    U.pushHistory(ex.id);
    this.initAudio();
  },
  onShow() { this.setData({ faved: U.load('fav', []).indexOf(this.data.ex.id) >= 0 }); },
  onHide() { if (this.audio && this.data.playing) this.audio.pause(); },
  onUnload() { if (this.audio) this.audio.destroy(); },
  onPageScroll(e) {
    const show = e.scrollTop > 300;
    if (show !== this.data.showTop) this.setData({ showTop: show });
  },
  initAudio() {
    const a = this.audio = wx.createInnerAudioContext();
    a.src = '/audio/guide.wav';
    a.onPlay(() => this.setData({ playing: true }));
    a.onPause(() => this.setData({ playing: false }));
    a.onStop(() => this.setData({ playing: false, pct: 0, cur: '00:00' }));
    a.onEnded(() => this.setData({ playing: false, pct: 100 }));
    a.onCanplay(() => { if (a.duration > 0) this.setData({ dur: fmt(a.duration) }); });
    a.onTimeUpdate(() => {
      const d = a.duration || 5;
      this.setData({ pct: Math.min(100, Math.round(a.currentTime / d * 100)), cur: fmt(a.currentTime), dur: fmt(d) });
    });
    a.onError(e => {
      this.setData({ playing: false });
      wx.showToast({ title: '讲解音频加载失败（' + (e.errCode || e.errMsg || '') + '）', icon: 'none' });
    });
  },
  togglePlay() {
    if (this.data.playing) this.audio.pause();
    else { if (this.data.pct >= 100) this.audio.seek(0); this.audio.play(); }
  },
  setRate(e) {
    const rate = +e.currentTarget.dataset.r;
    this.audio.playbackRate = rate;
    this.setData({ rate });
  },
  toggleFav() {
    const on = U.toggleFav(this.data.ex.id);
    this.setData({ faved: on });
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: on ? 'success' : 'none' });
  },
  openViewer() { this.setData({ viewer: true }); },
  closeViewer() { this.setData({ viewer: false }); },
  prevImg() { this.setData({ vIdx: (this.data.vIdx + this.data.gallery.length - 1) % this.data.gallery.length }); },
  nextImg() { this.setData({ vIdx: (this.data.vIdx + 1) % this.data.gallery.length }); },
  preview() {
    const urls = this.data.gallery.map(g => g.img);
    wx.previewImage({ urls, current: urls[this.data.vIdx] || urls[0] });
  },
  copyIns() {
    wx.setClipboardData({ data: this.data.ex.name + '：' + this.data.ex.inscription });
  },
  backTop() { this.setData({ scrollTop: this.data.scrollTop === '0px' ? '0rpx' : '0px' }); },
  openExhibit(e) { wx.redirectTo({ url: '/pages/exhibit/exhibit?id=' + e.currentTarget.dataset.id }); },
  onShareAppMessage() { return { title: '云川博物院 · ' + this.data.ex.name, path: '/pages/exhibit/exhibit?id=' + this.data.ex.id }; }
});
