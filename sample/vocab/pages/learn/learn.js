// 学习页：一张张卡片过新词。左右滑走 horizontal-drag-gesture-handler（Skyline 手势组件）+ worklet 共享值驱动卡片位移，
// 右滑 = 认识、左滑 = 不认识；也可以点下面三个按钮。
const store = require('../../utils/store.js');
const books = require('../../utils/books.js');
const app = getApp();
const THRESHOLD = 90;

Page({
  data: {
    queue: [], idx: 0, cur: null, revealed: false, starred: false,
    done: false, counts: { known: 0, fuzzy: 0, unknown: 0 }, pct: 0, total: 0,
    bookName: '', extra: false, canCheck: false, checked: false
  },
  onLoad(q) {
    const s = store.settings();
    const left = Math.max(0, s.plan - store.todayLog().learn);
    // extra=1：今天计划已完成后「再学一组」
    const n = q && q.extra ? 5 : left;
    const queue = store.newWords(s.book, n);
    this._autoPlay = s.autoPlay;
    this.setData({ queue, total: queue.length, bookName: books.book(s.book).name, extra: !!(q && q.extra) });
    // 共享值 x：手指拖动时直接改它，applyAnimatedStyle 每帧把它映射成卡片的 transform 与两侧提示的透明度
    const x = wx.worklet.shared(0);
    this._x = x;
    this.applyAnimatedStyle('#card', () => {
      'worklet';
      return { transform: 'translateX(' + x.value + 'px) rotate(' + (x.value / 24) + 'deg)' };
    });
    this.applyAnimatedStyle('#hint-r', () => { 'worklet'; return { opacity: Math.max(0, Math.min(1, x.value / THRESHOLD)) }; });
    this.applyAnimatedStyle('#hint-l', () => { 'worklet'; return { opacity: Math.max(0, Math.min(1, -x.value / THRESHOLD)) }; });
    if (queue.length) this.show(0); else this.finish();
  },
  onUnload() { const a = app.globalData.audio; if (a) a.stop(); },
  show(i) {
    const cur = this.data.queue[i];
    this.setData({ idx: i, cur, revealed: false, starred: store.isStar(cur.w), pct: Math.round(i * 100 / this.data.total) });
    // 等这一轮 setData 落到视图上再复位卡片位置、自动发音：避免新词还没换上，旧卡片先弹回中间闪一下
    wx.nextTick(() => {
      this._x.value = 0;
      if (this._autoPlay) app.speak(cur.w);
    });
  },
  onCardDrag(e) {
    if (this.data.done || !this.data.cur) return;
    if (e.state === 1 || e.state === 2) {
      this._x.value = e.translationX;
    } else if (e.state === 3) {
      const tx = e.translationX;
      if (tx > THRESHOLD) this.choose('known');
      else if (tx < -THRESHOLD) this.choose('unknown');
      else this._x.value = wx.worklet.spring(0, { damping: 18, stiffness: 220 });
    } else if (e.state === 4) {
      this._x.value = 0;
    }
  },
  onKnown() { this.choose('known'); },
  onFuzzy() { this.choose('fuzzy'); },
  onUnknown() { this.choose('unknown'); },
  choose(res) {
    if (this._busy) return;
    const cur = this.data.cur;
    store.markLearn(cur, res);
    const counts = Object.assign({}, this.data.counts);
    counts[res] += 1;
    this.setData({ counts });
    this._x.value = 0;
    if (res === 'known') { this.next(); return; }
    // 模糊 / 不认识：先把释义和例句亮出来看一眼，不认识的自动收进生词本
    if (res === 'unknown') {
      store.addStar(cur.w);
      wx.vibrateShort({ type: 'light' });
      this.setData({ starred: true });
      wx.showToast({ title: '已收进生词本', icon: 'none' });
    }
    app.play('/audio/flip.wav');
    this.setData({ revealed: true, pending: true });
  },
  onReveal() {
    if (this.data.revealed) return;
    app.play('/audio/flip.wav');
    this.setData({ revealed: true });
  },
  onNext() { this.setData({ pending: false }); this.next(); },
  next() {
    const i = this.data.idx + 1;
    if (i >= this.data.queue.length) { this.finish(); return; }
    this.show(i);
  },
  finish() {
    const s = store.settings();
    const tl = store.todayLog();
    const left = store.newWords(s.book, 1).length;
    const checked = store.checkedToday();
    this.setData({
      done: true, pct: 100, checked,
      canCheck: !checked && (tl.learn >= s.plan || left === 0) && store.dueList().length === 0,
      bookLeft: left, dueLeft: store.dueList().length
    });
    if (this.data.total) app.play('/audio/right.wav');
  },
  onSpeak() { app.speak(this.data.cur.w); },
  onStar() {
    const on = store.toggleStar(this.data.cur.w);
    this.setData({ starred: on });
    wx.showToast({ title: on ? '已加入生词本' : '已移出生词本', icon: 'none' });
  },
  onDetail() { wx.navigateTo({ url: '/pages/word/word?w=' + this.data.cur.w }); },
  onCheck() {
    const n = store.checkin();
    wx.vibrateShort({ type: 'medium' });
    wx.showToast({ title: '打卡成功，连续 ' + n + ' 天', icon: 'success' });
    this.setData({ checked: true, canCheck: false });
  },
  onMore() {
    if (!this.data.bookLeft) {
      wx.showModal({ title: '这本词书学完了', content: '可以换一本词书继续，已学的词照常按计划复习。', confirmText: '去换书', success: (r) => {
        if (r.confirm) wx.redirectTo({ url: '/pages/books/books' });
      } });
      return;
    }
    wx.showModal({ title: '再学一组？', content: '再学 5 个新词，明天的复习量会相应增加。', success: (r) => {
      if (r.confirm) wx.redirectTo({ url: '/pages/learn/learn?extra=1' });
    } });
  },
  goReview() { wx.switchTab({ url: '/pages/review/review' }); },
  goQuiz() { wx.redirectTo({ url: '/pages/quiz/quiz?src=book' }); },
  goHome() { wx.navigateBack({ fail: () => wx.switchTab({ url: '/pages/index/index' }) }); }
});
