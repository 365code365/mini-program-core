const data = require('../../utils/data.js');

const RATE_WORDS = ['完全不会', '很不满意', '不满意', '不太满意', '一般偏低', '一般', '还可以', '比较满意', '满意', '很满意', '非常满意'];

Page({
  data: { sv: null, step: 0, total: 1, q: {}, cur: null, optList: [], multiCount: 0, rateWord: '拖动滑块打分', err: '', typeText: { single: '单选', multi: '多选', rate: '评分', text: '填空' } },
  onLoad(q) {
    const sv = data.getSurvey(q.id || 'q1');
    this.answers = {};
    wx.setNavigationBarTitle({ title: sv.kind === '投票' ? '参与投票' : '填写问卷' });
    this.setData({ sv, total: sv.questions.length });
    this.show(0);
  },
  show(step) {
    const q = this.data.sv.questions[step];
    const cur = this.answers[q.id];
    const sel = Array.isArray(cur) ? cur : [];
    this.setData({
      step, q, cur: cur === undefined ? null : cur, err: '',
      optList: q.type === 'multi' ? q.options.map((t, i) => ({ t, on: sel.indexOf(i) >= 0 })) : [],
      multiCount: sel.length,
      rateWord: q.type === 'rate' && typeof cur === 'number' ? RATE_WORDS[cur] : '拖动滑块打分'
    });
  },
  set(v) { this.answers[this.data.q.id] = v; },
  onSingle(e) { const i = e.currentTarget.dataset.idx; this.set(i); this.setData({ cur: i, err: '' }); },
  onMulti(e) {
    const i = e.currentTarget.dataset.idx;
    let sel = (this.answers[this.data.q.id] || []).slice();
    sel = sel.indexOf(i) >= 0 ? sel.filter(x => x !== i) : sel.concat([i]).sort();
    this.set(sel);
    this.setData({ ['optList[' + i + '].on']: sel.indexOf(i) >= 0, multiCount: sel.length, err: '' });
  },
  onRate(e) { const v = Number(e.detail.value); this.set(v); this.setData({ cur: v, rateWord: RATE_WORDS[v] || '', err: '' }); },
  onText(e) { this.set(e.detail.value); this.setData({ cur: e.detail.value }); },
  valid() {
    const q = this.data.q;
    const v = this.answers[q.id];
    if (!q.required) return true;
    const empty = v === undefined || v === null || (Array.isArray(v) && v.length === 0) || (typeof v === 'string' && !v.trim());
    if (empty) {
      this.setData({ err: q.type === 'multi' ? '此题为必答题，请至少选择一项' : q.type === 'rate' ? '此题为必答题，请拖动滑块打分' : '此题为必答题' });
      wx.showToast({ title: '第 ' + (this.data.step + 1) + ' 题未作答', icon: 'none' });
      return false;
    }
    return true;
  },
  onPrev() { if (this.data.step > 0) this.show(this.data.step - 1); },
  onJump(e) {
    const i = e.currentTarget.dataset.idx;
    if (i <= this.data.step || this.valid()) this.show(i > this.data.step ? this.data.step + 1 : i);
  },
  onNext() {
    if (!this.valid()) return;
    if (this.data.step < this.data.total - 1) { this.show(this.data.step + 1); return; }
    // 提交前再整体校验一遍必答题
    const miss = this.data.sv.questions.findIndex(q => q.required && (this.answers[q.id] === undefined || (Array.isArray(this.answers[q.id]) && !this.answers[q.id].length)));
    if (miss >= 0) { this.show(miss); this.valid(); return; }
    const all = data.getAnswers();
    all[this.data.sv.id] = this.answers;
    wx.setStorageSync('sv_answers', all);
    wx.showToast({ title: '提交成功', icon: 'success' });
    setTimeout(() => wx.redirectTo({ url: '/pages/result/result?id=' + this.data.sv.id + '&fresh=1' }), 700);
  }
});
