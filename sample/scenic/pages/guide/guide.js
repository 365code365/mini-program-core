// 语音导览（tab）：景点卡片竖向 swiper，上下滑切换讲解点。
// 单段试听用 InnerAudioContext（离开页面就停）；「全程连播」用 BackgroundAudioManager（切到别的页 / 锁屏继续播，一段放完自动下一段）
const data = require('../../utils/data.js');
const narr = require('../../utils/narr.js');
const store = require('../../utils/store.js');
const tab = require('../../utils/tab.js');
const app = getApp();

function mmss(s) { s = Math.max(0, Math.floor(s || 0)); return '0' + Math.floor(s / 60) + ':' + (s % 60 < 10 ? '0' : '') + (s % 60); }

Page({
  data: { scenics: [], sid: '', s: null, spots: [], cur: 0, mode: '', playing: false, pos: '00:00', dur: '00:00', pct: 0, chime: true },
  onLoad() {
    this.setData({ scenics: data.SCENIC.map((x) => ({ id: x.id, short: x.short })) });
    this.inner = wx.createInnerAudioContext();
    this.inner.onPlay(() => this.setData({ playing: true }));
    this.inner.onPause(() => this.setData({ playing: false }));
    this.inner.onStop(() => this.setData({ playing: false }));
    this.inner.onEnded(() => this.setData({ playing: false, pct: 100 }));
    this.inner.onTimeUpdate(() => this.tick(this.inner.currentTime, this.inner.duration));
    this.inner.onError((e) => { this.setData({ playing: false }); wx.showToast({ title: '讲解音频加载失败', icon: 'none' }); console.log('inner audio error', e && e.errMsg); });
    this.chimeCtx = wx.createInnerAudioContext();
    this.chimeCtx.src = '/audio/chime.wav';
    this.bindBgm();
  },
  onShow() {
    tab.sync(this, 1);
    const sid = app.globalData.scenic;
    if (sid !== this.data.sid) this.load(sid);
    this.setData({ chime: store.settings().chime });
  },
  onHide() { if (this.data.mode === 'inner') this.inner.pause(); },
  onUnload() { this.inner.destroy(); this.chimeCtx.destroy(); },
  load(sid, keepIdx) {
    const s = data.find(sid);
    const spots = s.spots.map((p) => Object.assign({}, p, { img: '/images/spot/' + p.id + '.jpg', text: narr[p.id], src: '/audio/' + p.id + '.mp3', durText: mmss(p.dur) }));
    this.setData({ sid: s.id, s: { id: s.id, name: s.name, short: s.short }, spots, cur: keepIdx || 0, pct: 0, pos: '00:00', dur: spots[keepIdx || 0].durText });
  },
  onScenic(e) {
    const id = e.currentTarget.dataset.id;
    if (id === this.data.sid) return;
    this.stopAll();
    app.setScenic(id);
    this.load(id);
  },
  bindBgm() {
    // 本引擎当前版本 wx.getBackgroundAudioManager() 会抛「property is not configurable」（见报告「引擎问题」）：
    // 拿不到就没有「全程连播」，单段讲解照常用 InnerAudioContext
    let bg = null;
    try { bg = wx.getBackgroundAudioManager(); } catch (e) { console.log('BackgroundAudioManager 不可用:', e && e.message); }
    this.bg = bg;
    if (!bg) return;
    bg.onPlay(() => { if (this.data.mode === 'bg') this.setData({ playing: true }); });
    bg.onPause(() => { if (this.data.mode === 'bg') this.setData({ playing: false }); });
    bg.onStop(() => { if (this.data.mode === 'bg') this.setData({ playing: false, mode: '' }); });
    bg.onTimeUpdate(() => { if (this.data.mode === 'bg') this.tick(bg.currentTime, bg.duration); });
    bg.onEnded(() => {
      if (this.data.mode !== 'bg') return;
      const next = this.data.cur + 1;
      if (next >= this.data.spots.length || !store.settings().autoNext) {
        this.setData({ playing: false, mode: '', pct: 100 });
        wx.showToast({ title: this.data.s.short + '全程讲解已播完', icon: 'none' });
        return;
      }
      this.setData({ cur: next, dur: this.data.spots[next].durText });
      this.playBg(next);
    });
    bg.onError((e) => { this.setData({ playing: false, mode: '' }); console.log('bgm error', e && e.errMsg); wx.showToast({ title: '后台播放不可用，已改为单段播放', icon: 'none' }); });
  },
  tick(t, d) {
    const pos = mmss(t);
    if (pos === this.data.pos) return;
    this.setData({ pos, dur: d ? mmss(d) : this.data.dur, pct: d ? Math.round(t * 100 / d) : 0 });
  },
  playInner(i) {
    if (this.data.mode === 'bg') this.bg.stop();
    this.inner.stop();
    this.inner.src = this.data.spots[i].src;
    this.inner.play();
    this.setData({ mode: 'inner', pct: 0, pos: '00:00' });
  },
  playBg(i) {
    const p = this.data.spots[i], s = this.data.s;
    this.inner.stop();
    // 背景音频要给 title；设置 src 后会自动开播
    this.bg.title = p.name + ' · 语音讲解';
    this.bg.epname = s.name;
    this.bg.singer = '游山河讲解员';
    this.bg.coverImgUrl = p.img;
    this.bg.src = p.src;
    this.setData({ mode: 'bg', pct: 0, pos: '00:00', playing: true });
  },
  stopAll() {
    this.inner.stop();
    if (this.data.mode === 'bg') this.bg.stop();
    this.setData({ mode: '', playing: false, pct: 0, pos: '00:00' });
  },
  // 程序改 current（连播自动下一段）时 change 带回的就是当前下标，直接忽略
  onSwipe(e) { this.go(e.detail.current); },
  go(i) {
    if (i === this.data.cur || i < 0 || i >= this.data.spots.length) return;
    this.setData({ cur: i, dur: this.data.spots[i].durText });
    if (this.data.chime) { this.chimeCtx.stop(); this.chimeCtx.play(); }
    // 正在播：跟着切到新景点
    if (this.data.playing) { if (this.data.mode === 'bg') this.playBg(i); else this.playInner(i); }
    else this.setData({ pct: 0, pos: '00:00', mode: '' });
  },
  onToggle() {
    if (this.data.playing) { if (this.data.mode === 'bg') this.bg.pause(); else this.inner.pause(); return; }
    if (this.data.mode === 'bg') { this.bg.play(); return; }
    if (this.data.mode === 'inner' && this.data.pct > 0 && this.data.pct < 100) { this.inner.play(); return; }
    this.playInner(this.data.cur);
  },
  onAll() {
    if (this.data.mode === 'bg') { this.stopAll(); return; }
    if (!this.bg) { wx.showToast({ title: '当前环境不支持后台连播，已改为单段播放', icon: 'none' }); this.playInner(this.data.cur); return; }
    this.playBg(this.data.cur);
    wx.showToast({ title: '全程连播已开启，切到其他页面也会继续', icon: 'none' });
  },
  onPrev() { this.go(this.data.cur - 1); },
  onNext() { this.go(this.data.cur + 1); },
  goRoute() { wx.navigateTo({ url: '/pages/route/route?id=' + this.data.sid }); },
  goDetail() { wx.navigateTo({ url: '/pages/detail/detail?id=' + this.data.sid }); }
});
