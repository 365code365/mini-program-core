// 生词本（tab）：学习时点「不认识」、复习「忘记了」会自动收进来，也能手动加；可搜索、移出、拿来测验
const store = require('../../utils/store.js');
const books = require('../../utils/books.js');
const tab = require('../../utils/tab.js');
const app = getApp();

Page({
  data: { all: [], list: [], kw: '', sort: 'new' },
  onShow() {
    tab.sync(this, 2);
    this.load();
  },
  load() {
    const p = store.progress();
    const all = store.stars().map((w, i) => {
      const x = books.find(w) || { w, ph: '', mean: '' };
      const r = p[w];
      return Object.assign({}, x, { order: i, state: r ? (r.due ? '下次复习 ' + r.due.slice(5) : '已掌握') : '未学' });
    });
    this.setData({ all });
    this.filter();
  },
  filter() {
    const kw = this.data.kw.trim().toLowerCase();
    let list = this.data.all.filter((x) => !kw || x.w.indexOf(kw) >= 0 || x.mean.indexOf(kw) >= 0);
    if (this.data.sort === 'az') list = list.slice().sort((a, b) => (a.w < b.w ? -1 : 1));
    this.setData({ list });
  },
  onInput(e) { this.setData({ kw: e.detail.value }); this.filter(); },
  onClear() { this.setData({ kw: '' }); this.filter(); },
  onSort(e) { this.setData({ sort: e.currentTarget.dataset.s }); this.filter(); },
  onSpeak(e) { app.speak(e.currentTarget.dataset.w); },
  onRemove(e) {
    const w = e.currentTarget.dataset.w;
    wx.showModal({ title: '移出生词本', content: '确定把「' + w + '」移出生词本吗？学习进度不受影响。', success: (r) => {
      if (r.confirm) { store.toggleStar(w); this.load(); wx.showToast({ title: '已移出', icon: 'none' }); }
    } });
  },
  goWord(e) { wx.navigateTo({ url: '/pages/word/word?w=' + e.currentTarget.dataset.w }); },
  goQuiz() {
    if (this.data.all.length < 4) { wx.showToast({ title: '生词满 4 个才能出四选一题', icon: 'none' }); return; }
    wx.navigateTo({ url: '/pages/quiz/quiz?src=star' });
  },
  goLearn() { wx.navigateTo({ url: '/pages/learn/learn' }); }
});
