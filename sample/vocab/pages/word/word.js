// 单词详情：发音、释义、例句、学习状态（第几轮、下次复习）、加入 / 移出生词本、错题记录
const store = require('../../utils/store.js');
const books = require('../../utils/books.js');
const app = getApp();

Page({
  data: { it: null, star: false, state: '', next: '', wrongTimes: 0, bookName: '' },
  onLoad(q) {
    const it = books.find(q && q.w) || books.words('cet4')[0];
    wx.setNavigationBarTitle({ title: it.w });
    this.setData({ it, bookName: books.book(it.book).full });
    if (store.settings().autoPlay) app.speak(it.w);
  },
  onShow() {
    const it = this.data.it;
    if (!it) return;
    const r = store.progress()[it.w];
    const wr = store.wrongs().find((x) => x.w === it.w);
    let state = '还没学过', next = '';
    if (r) {
      state = r.due ? '复习中 · 第 ' + (r.stage + 1) + ' / ' + store.INTERVALS.length + ' 轮' : '已掌握';
      next = r.due ? '下次复习 ' + r.due : '五轮复习都记住了';
    }
    this.setData({ star: store.isStar(it.w), state, next, wrongTimes: wr ? wr.times : 0 });
  },
  onSpeak() { app.speak(this.data.it.w); },
  onStar() {
    const on = store.toggleStar(this.data.it.w);
    this.setData({ star: on });
    wx.showToast({ title: on ? '已加入生词本' : '已移出生词本', icon: 'none' });
  },
  onCopy() {
    const it = this.data.it;
    wx.setClipboardData({ data: it.w + ' ' + it.ph + ' ' + it.mean + '\n' + it.ex });
  }
});
