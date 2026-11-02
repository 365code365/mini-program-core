const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const theme = require('../../utils/theme.js');

Page({
  data: { theme: 'light', c: null, chapters: [], doneN: 0, fav: false, startText: '开始学习', learnersText: '', teacherInitial: '' },

  onLoad(q) {
    const c = data.course(q.id) || data.COURSES[0];
    this.cid = c.id;
    wx.setNavigationBarTitle({ title: c.title });
    this.setData({
      c,
      learnersText: c.learners >= 10000 ? (c.learners / 10000).toFixed(1) + '万' : String(c.learners),
      teacherInitial: c.teacher.substring(0, 1)
    });
  },

  onShow() {
    theme.apply(this, -1);
    this.refresh();
  },

  applyTheme() { theme.apply(this, -1); },

  refresh() {
    const c = this.data.c;
    const done = store.doneMap()[c.id] || [];
    const chapters = c.chapters.map((x, i) => Object.assign({}, x, { done: done.indexOf(i) >= 0, locked: !store.unlocked(c.id, i) }));
    const next = chapters.findIndex(x => !x.done);
    const last = store.last();
    let startText = '开始学习';
    if (next < 0) startText = '全部完成 · 再听一遍';
    else if (done.length > 0 || (last && last.courseId === c.id)) startText = '继续学习 · 第 ' + (next + 1) + ' 节';
    this.next = next < 0 ? 0 : next;
    this.setData({ chapters, doneN: done.length, fav: store.favs().indexOf(c.id) >= 0, startText });
  },

  toggleFav() {
    const on = store.toggleFav(this.cid);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
  },

  onChapter(e) {
    const idx = e.currentTarget.dataset.idx;
    const ch = this.data.chapters[idx];
    if (ch.locked) {
      wx.showModal({
        title: '本节尚未解锁',
        content: '完成「' + this.data.chapters[idx - 1].title + '」后自动解锁，循序渐进效果更好。',
        confirmText: '去听上一节',
        success: res => { if (res.confirm) this.play(idx - 1); }
      });
      return;
    }
    this.play(idx);
  },

  start() { this.play(this.next); },

  play(idx) {
    // 上一节可能也没解锁：顺着往前找第一节能听的
    while (idx > 0 && this.data.chapters[idx].locked) idx--;
    wx.navigateTo({ url: '/pages/player/player?id=' + this.cid + '&idx=' + idx });
  }
});
