const data = require('../../utils/data.js');
const LETTERS = ['A', 'B', 'C', 'D'];
Page({
  data: { idx: 0, total: 0, q: null, picked: -1, answers: [], finished: false, score: 0, right: 0, pct: 0, review: [] },
  onLoad() { this.start(); },
  start() {
    const total = data.questions.length;
    this.setData({ idx: 0, total, answers: [], finished: false, score: 0, right: 0 });
    this.show(0);
  },
  show(i) {
    const src = data.questions[i];
    const q = { q: src.q, exp: src.exp, ans: src.ans, opts: src.opts.map((t, k) => ({ k, t, l: LETTERS[k], cls: '' })) };
    this.setData({ idx: i, q, picked: -1, isLast: i === this.data.total - 1, pct: Math.round(i / this.data.total * 100) });
  },
  pick(e) {
    if (this.data.picked >= 0) return;
    const k = e.currentTarget.dataset.k;
    const q = this.data.q;
    const opts = q.opts.map(o => Object.assign({}, o, { cls: o.k === q.ans ? 'right' : (o.k === k ? 'wrong' : 'dim') }));
    const answers = this.data.answers.concat([k]);
    this.setData({ picked: k, 'q.opts': opts, answers, pct: Math.round((this.data.idx + 1) / this.data.total * 100) });
  },
  next() {
    if (this.data.picked < 0) { wx.showToast({ title: '请先选择一个答案', icon: 'none' }); return; }
    if (this.data.idx + 1 < this.data.total) { this.show(this.data.idx + 1); return; }
    this.submit();
  },
  submit() {
    const qs = data.questions;
    const answers = this.data.answers;
    const right = answers.filter((a, i) => a === qs[i].ans).length;
    const score = Math.round(right / qs.length * 100);
    const review = qs.map((q, i) => ({ i: i + 1, ok: answers[i] === q.ans, mine: LETTERS[answers[i]], ans: LETTERS[q.ans] }));
    wx.showModal({ title: '确认交卷', content: '共 ' + qs.length + ' 题，已全部作答。交卷后将显示成绩。', confirmText: '交卷', success: r => {
      if (!r.confirm) return;
      wx.setStorageSync('zx_exam_today', true);
      const best = wx.getStorageSync('zx_exam_best') || 0;
      if (score > best) wx.setStorageSync('zx_exam_best', score);
      this.setData({ finished: true, score, right, review });
    } });
  },
  retry() { this.start(); },
  back() { wx.navigateBack(); }
});
