const G = require('../../utils/game.js');
const QS = require('../../utils/questions.js');

const TOTAL = 10;
const TIME = 10000;
const STEP = 100;
const LETTERS = ['A', 'B', 'C', 'D'];

function shuffle(a) {
  const r = a.slice();
  for (let i = r.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    const t = r[i]; r[i] = r[j]; r[j] = t;
  }
  return r;
}

Page({
  data: {
    idx: 0, total: TOTAL, progress: 0, score: 0, combo: 0, comboAnim: false, q: {}, opts: [], catName: '', catColor: '', catBg: '',
    sec: 10, rotR: 45, rotL: 45, showR: true, showL: true, ringColor: '#FFD25A', phase: 'answer', halfUsed: false, props: {}, verdict: '', verdictCls: '', gainText: '', gainAnim: false
  },

  onLoad() {
    const cats = G.cats();
    this.list = shuffle(QS.byCats(cats)).slice(0, TOTAL);
    this.stats = { correct: 0, answered: 0, skipped: 0, maxCombo: 0, ticks: 0, wrongIds: [] };
    this.setData({ total: this.list.length, props: G.profile().props });
    this.show(0);
  },
  onUnload() { this.stop(); clearTimeout(this.nextT); clearTimeout(this.fxT); this.quitting = true; },

  show(i) {
    const q = this.list[i];
    const c = QS.catOf(q.cat);
    this.left = TIME;
    this.setData({
      idx: i, q, progress: Math.round(i / this.list.length * 100), phase: 'answer', halfUsed: false, verdict: '', gainText: '',
      catName: c.t, catColor: c.c, catBg: QS.tint(c.c, 0.12),
      opts: q.opts.map((t, k) => ({ i: k, l: LETTERS[k], t, cls: '' }))
    });
    this.paintRing();
    this.stop();
    // 按节拍数扣时间，不读 Date.now()：无头模式下时钟是固定的
    this.timer = setInterval(() => {
      this.left -= STEP;
      this.stats.ticks += 1;
      if (this.left <= 0) { this.left = 0; this.paintRing(); this.timeout(); } else this.paintRing();
    }, STEP);
  },
  stop() { clearInterval(this.timer); this.timer = null; },

  paintRing() {
    const f = this.left / TIME;
    const rotR = f >= 0.5 ? 45 : f * 360 - 135;
    const rotL = f >= 0.5 ? -135 + (f - 0.5) * 360 : -135;
    const sec = Math.ceil(this.left / 1000);
    // 一半为空时整块不画：转到边界的弧在容器接缝处会留一条细线
    this.setData({ rotR, rotL, sec, showR: f > 0, showL: f > 0.5, ringColor: sec <= 3 ? '#FF5C7A' : '#FFD25A' });
  },

  choose(e) {
    if (this.data.phase !== 'answer') return;
    const pick = e.currentTarget.dataset.i;
    if (this.data.opts[pick].cls === 'gone') return;
    this.stop();
    const q = this.data.q;
    const ok = pick === q.a;
    this.stats.answered += 1;
    let { score, combo } = this.data;
    let gain = 0;
    if (ok) {
      combo += 1;
      this.stats.correct += 1;
      this.stats.maxCombo = Math.max(this.stats.maxCombo, combo);
      gain = 100 + Math.ceil(this.left / 1000) * 10 + (combo >= 2 ? (combo - 1) * 20 : 0);
      score += gain;
    } else {
      combo = 0;
      G.addWrong(q, pick);
      this.stats.wrongIds.push(q.id);
    }
    this.setData({
      phase: 'reveal', score, combo, comboAnim: ok && combo >= 2,
      opts: this.data.opts.map(o => Object.assign({}, o, { cls: o.i === q.a ? 'right' : o.i === pick ? 'wrong' : o.cls === 'gone' ? 'gone' : 'dim' })),
      verdict: ok ? (combo >= 2 ? '答对了！连对加成 +' + (combo - 1) * 20 : '答对了！') : '答错了，正确答案是 ' + LETTERS[q.a],
      verdictCls: ok ? 'v-ok' : 'v-no',
      gainText: ok ? '+' + gain : '', gainAnim: ok
    });
    if (ok) wx.vibrateShort && wx.vibrateShort({ type: 'light' });
    this.nextT = setTimeout(() => this.next(), 1200);
  },

  timeout() {
    if (this.data.phase !== 'answer') return;
    this.stop();
    const q = this.data.q;
    this.stats.answered += 1;
    G.addWrong(q, -1);
    this.stats.wrongIds.push(q.id);
    this.setData({
      phase: 'reveal', combo: 0,
      opts: this.data.opts.map(o => Object.assign({}, o, { cls: o.i === q.a ? 'right' : o.cls === 'gone' ? 'gone' : 'dim' })),
      verdict: '时间到！正确答案是 ' + LETTERS[q.a], verdictCls: 'v-no'
    });
    this.nextT = setTimeout(() => this.next(), 1000);
  },

  next() {
    if (this.quitting) return;
    const i = this.data.idx + 1;
    if (i >= this.list.length) return this.finish();
    this.show(i);
  },

  useHalf() {
    const props = this.data.props;
    if (this.data.phase !== 'answer') return;
    if (this.data.halfUsed) return wx.showToast({ title: '本题已使用过 50:50', icon: 'none' });
    if (!props.half) return wx.showToast({ title: '50:50 道具不足，签到可领', icon: 'none' });
    const q = this.data.q;
    const wrong = shuffle([0, 1, 2, 3].filter(k => k !== q.a)).slice(0, 2);
    const p = G.profile();
    p.props.half -= 1;
    G.saveProfile(p);
    this.setData({ halfUsed: true, props: p.props, opts: this.data.opts.map(o => Object.assign({}, o, { cls: wrong.indexOf(o.i) >= 0 ? 'gone' : o.cls })) });
  },

  useSkip() {
    if (this.data.phase !== 'answer') return;
    if (!this.data.props.skip) return wx.showToast({ title: '跳过道具不足，签到可领', icon: 'none' });
    this.stop();
    const p = G.profile();
    p.props.skip -= 1;
    G.saveProfile(p);
    this.stats.skipped += 1;
    // 跳过不算答错，也不打断连对
    this.setData({ props: p.props, phase: 'reveal', verdict: '已跳过本题', verdictCls: 'v-ok' });
    this.nextT = setTimeout(() => this.next(), 500);
  },

  quit() {
    const wasAnswer = this.data.phase === 'answer';
    this.stop();
    wx.showModal({
      title: '退出答题？',
      content: '中途退出本局不计成绩，已消耗的体力不返还',
      confirmText: '退出',
      cancelText: '继续答题',
      success: r => {
        if (r.confirm) { this.quitting = true; wx.navigateBack(); return; }
        // 继续：从暂停处接着倒计时
        if (wasAnswer && this.data.phase === 'answer') {
          this.timer = setInterval(() => {
            this.left -= STEP;
            this.stats.ticks += 1;
            if (this.left <= 0) { this.left = 0; this.paintRing(); this.timeout(); } else this.paintRing();
          }, STEP);
        }
      }
    });
  },

  finish() {
    const s = this.stats;
    const p = G.profile();
    const score = this.data.score;
    const newRecord = score > p.best;
    const result = {
      score, correct: s.correct, answered: s.answered, skipped: s.skipped, total: this.list.length, maxCombo: s.maxCombo,
      seconds: Math.round(s.ticks * STEP / 100) / 10, newRecord, prevBest: p.best, wrongIds: s.wrongIds, totalBefore: p.total
    };
    p.best = Math.max(p.best, score);
    p.games += 1;
    p.total += score;
    G.saveProfile(p);
    G.record(score);
    G.set('lastResult', result);
    wx.redirectTo({ url: '/pages/result/result' });
  }
});
