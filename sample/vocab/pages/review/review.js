// 复习（tab）：到期词逐个过，记得升一档（1/2/4/7/15 天），忘了退回明天
const store = require('../../utils/store.js');
const tab = require('../../utils/tab.js');
const app = getApp();

Page({
  data: {
    list: [], running: false, i: 0, cur: null, revealed: false, ok: 0, bad: 0, finished: false,
    upcoming: [], intervals: store.INTERVALS, stageText: ['第 1 轮', '第 2 轮', '第 3 轮', '第 4 轮', '第 5 轮'], maxUp: 1
  },
  onShow() {
    tab.sync(this, 1);
    if (!this.data.running) this.load();
  },
  load() {
    const up = store.upcoming(7);
    this.setData({
      list: store.dueList(), upcoming: up, maxUp: Math.max(1, ...up.map((x) => x.count)), finished: false
    });
  },
  onStart() {
    if (!this.data.list.length) return;
    this.setData({ running: true, i: 0, cur: this.data.list[0], revealed: false, ok: 0, bad: 0 });
    if (store.settings().autoPlay) app.speak(this.data.list[0].w);
  },
  onReveal() { this.setData({ revealed: true }); app.play('/audio/flip.wav'); },
  onSpeak() { app.speak(this.data.cur.w); },
  onRemember() { this.answer(true); },
  onForget() { this.answer(false); },
  answer(ok) {
    const r = store.markReview(this.data.cur.w, ok);
    if (!ok) { store.addStar(this.data.cur.w); wx.vibrateShort({ type: 'light' }); }
    const patch = ok ? { ok: this.data.ok + 1 } : { bad: this.data.bad + 1 };
    if (ok && r && !r.due) wx.showToast({ title: this.data.cur.w + ' 已掌握', icon: 'none' });
    const i = this.data.i + 1;
    if (i >= this.data.list.length) {
      this.setData(Object.assign(patch, { running: false, finished: true, cur: null }));
      tab.sync(this, 1);
      const up = store.upcoming(7);
      this.setData({ list: store.dueList(), upcoming: up, maxUp: Math.max(1, ...up.map((x) => x.count)) });
      app.play('/audio/right.wav');
      return;
    }
    this.setData(Object.assign(patch, { i, cur: this.data.list[i], revealed: false }));
    if (store.settings().autoPlay) app.speak(this.data.list[i].w);
  },
  onQuit() {
    wx.showModal({ title: '结束本次复习？', content: '已复习的词会保存，剩下的仍在待复习里。', success: (r) => {
      if (r.confirm) { this.setData({ running: false }); this.load(); tab.sync(this, 1); }
    } });
  },
  goWord(e) { wx.navigateTo({ url: '/pages/word/word?w=' + e.currentTarget.dataset.w }); },
  goLearn() { wx.navigateTo({ url: '/pages/learn/learn' }); },
  goQuiz() { wx.navigateTo({ url: '/pages/quiz/quiz?src=book' }); }
});
