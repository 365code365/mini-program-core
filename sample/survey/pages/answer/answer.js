const data = require('../../utils/data.js');

const RATE_WORDS = ['完全不会', '很不满意', '不满意', '不太满意', '一般偏低', '一般', '还可以', '比较满意', '满意', '很满意', '非常满意'];

Page({
  data: { sv: null, step: 0, total: 1, q: {}, cur: null, optList: [], multiCount: 0, rateWord: '拖动滑块打分', err: '', typeText: { single: '单选', multi: '多选', rate: '评分', text: '填空' } },
  onLoad(q) {
    const sv = data.getSurvey(q.id || 'q1');
    this.answers = {};
    wx.setNavigationBarTitle({ title: sv.kind === '投票' ? '参与投票' : '填写问卷' });
    this.setData({ sv, total: sv.questions.length });
    // 已截止：不让作答，直接去看结果
    if (data.isEnded(sv)) {
      this.show(0);
      wx.showModal({
        title: '已截止', content: '「' + sv.title + '」已于 ' + sv.deadline + ' 截止，不能再作答', showCancel: false, confirmText: '看结果', confirmColor: '#6C5CE7',
        success: () => wx.redirectTo({ url: '/pages/result/result?id=' + sv.id })
      });
      return;
    }
    // 上次没答完：接着答
    const draft = data.getDraft(sv.id);
    if (draft && draft.answers) {
      this.answers = draft.answers;
      this.show(Math.min(draft.step || 0, sv.questions.length - 1));
      wx.showToast({ title: '已恢复上次的作答', icon: 'none' });
      return;
    }
    this.show(0);
  },
  // 每次改答案 / 翻题都落一份草稿（提交成功后清掉）
  keep() {
    if (this.submitted || data.isEnded(this.data.sv)) return;
    data.saveDraft(this.data.sv.id, { step: this.data.step, answers: this.answers });
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
    if (Object.keys(this.answers).length) this.keep();
  },
  set(v) { this.answers[this.data.q.id] = v; this.keep(); },
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
    this.submitted = true;
    data.saveDraft(this.data.sv.id, null);
    wx.showToast({ title: '提交成功', icon: 'success' });
    setTimeout(() => wx.redirectTo({ url: '/pages/result/result?id=' + this.data.sv.id + '&fresh=1' }), 700);
  }
});
