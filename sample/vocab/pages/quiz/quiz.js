// 测验：四选一（看词选义）/ 拼写（看义写词）。题目与干扰项用 wx.getRandomValues 洗牌；答错进错题本、短振动。
// src=book 当前词书已学的词、star 生词本、wrong 错题本（错题模式下答对即移出错题本）
const store = require('../../utils/store.js');
const books = require('../../utils/books.js');
const rand = require('../../utils/rand.js');
const app = getApp();
const N = 10;
const SRC_NAME = { book: '已学单词', star: '生词本', wrong: '错题本' };

Page({
  data: {
    src: 'book', srcName: '', mode: 'choice', loading: true, qs: [], i: 0, q: null,
    picked: -1, spell: '', spellState: '', score: 0, wrongList: [], done: false, empty: false
  },
  onLoad(q) {
    const src = (q && q.src) || 'book';
    this.setData({ src, srcName: SRC_NAME[src] || SRC_NAME.book });
    wx.setNavigationBarTitle({ title: '测验 · ' + this.data.srcName });
    this.build();
  },
  pool() {
    const src = this.data.src;
    if (src === 'star') return store.stars().map(books.find).filter(Boolean);
    if (src === 'wrong') return store.wrongs().map((x) => books.find(x.w)).filter(Boolean);
    const b = store.settings().book;
    const l = store.learnedOf(b);
    return l.length >= 4 ? l : books.words(b);
  },
  async build() {
    this.setData({ loading: true, done: false });
    const pool = this.pool();
    if (!pool.length) { this.setData({ loading: false, empty: true }); return; }
    const picked = (await rand.shuffle(pool)).slice(0, N);
    // 干扰项从同一本书里取，词性相近更有迷惑性；不够再从全部词书补
    const qs = [];
    for (const it of picked) {
      let others = books.words(it.book).filter((x) => x.w !== it.w);
      if (others.length < 3) others = others.concat(books.words('cet4').filter((x) => x.w !== it.w));
      const ds = (await rand.shuffle(others)).slice(0, 3);
      const opts = await rand.shuffle([it].concat(ds));
      qs.push({ item: it, opts: opts.map((o, k) => ({ w: o.w, mean: o.mean, k: 'ABCD'.charAt(k) })), answer: opts.indexOf(it), len: it.w.length, hint: it.w.charAt(0) + ' _ '.repeat(it.w.length - 1).trim() });
    }
    this.setData({ qs, i: 0, q: qs[0], picked: -1, spell: '', spellState: '', score: 0, wrongList: [], loading: false, empty: false });
  },
  onMode(e) {
    const m = e.currentTarget.dataset.m;
    if (m === this.data.mode) return;
    if (this.data.i > 0 && !this.data.done) {
      wx.showModal({ title: '切换题型', content: '切换后本轮重新开始，确定吗？', success: (r) => { if (r.confirm) { this.setData({ mode: m }); this.build(); } } });
      return;
    }
    this.setData({ mode: m });
    this.build();
  },
  onSpeak() { app.speak(this.data.q.item.w); },
  judge(ok) {
    const it = this.data.q.item;
    if (ok) {
      app.play('/audio/right.wav');
      if (this.data.src === 'wrong') store.removeWrong(it.w);
      this.setData({ score: this.data.score + 1 });
    } else {
      app.play('/audio/wrong.wav');
      wx.vibrateShort({ type: 'heavy' });
      store.addWrong(it.w);
      this.setData({ wrongList: this.data.wrongList.concat([{ w: it.w, mean: it.mean }]) });
    }
  },
  onPick(e) {
    if (this.data.picked >= 0) return;
    const k = e.currentTarget.dataset.k;
    this.setData({ picked: k });
    this.judge(k === this.data.q.answer);
    this._t = setTimeout(() => this.next(), k === this.data.q.answer ? 700 : 1400);
  },
  onSpellInput(e) { this.setData({ spell: e.detail.value }); },
  onSpellSubmit() {
    if (this.data.spellState) { this.next(); return; }
    const v = this.data.spell.trim().toLowerCase();
    if (!v) { wx.showToast({ title: '先把单词拼出来', icon: 'none' }); return; }
    const ok = v === this.data.q.item.w.toLowerCase();
    this.setData({ spellState: ok ? 'ok' : 'bad' });
    this.judge(ok);
  },
  onSkip() {
    if (this.data.spellState) { this.next(); return; }
    this.setData({ spellState: 'bad' });
    this.judge(false);
  },
  next() {
    clearTimeout(this._t);
    const i = this.data.i + 1;
    if (i >= this.data.qs.length) {
      store.bump('quiz', this.data.qs.length);
      this.setData({ done: true });
      return;
    }
    this.setData({ i, q: this.data.qs[i], picked: -1, spell: '', spellState: '' });
  },
  onUnload() { clearTimeout(this._t); },
  onAgain() { this.build(); },
  goWrong() { wx.redirectTo({ url: '/pages/wrong/wrong' }); },
  goWord(e) { wx.navigateTo({ url: '/pages/word/word?w=' + e.currentTarget.dataset.w }); },
  goLearn() { wx.redirectTo({ url: '/pages/learn/learn' }); }
});
