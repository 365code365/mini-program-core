const S = require('../../utils/store.js');

const COLORS = ['#F0534A', '#3E7BFA', '#22B07D', '#F7A23B', '#8B5CF6', '#06B6D4', '#EC4899'];

Page({
  data: { listH: 600, kw: '', groups: [], letters: [], intoView: '', bigLetter: '' },

  onLoad(options) {
    this.mode = options.mode || 'go';
    const sys = wx.getSystemInfoSync();
    this.setData({ listH: Math.floor(sys.windowHeight - 105 * sys.windowWidth / 750) });
    wx.setNavigationBarTitle({ title: this.mode === 'pick' ? '选择收款人' : '收款人' });
    this.build('');
  },

  build(kw) {
    const k = kw.trim();
    const map = {};
    S.PAYEES.forEach((p, idx) => {
      if (k && p.name.indexOf(k) < 0 && p.bank.indexOf(k) < 0 && p.card.slice(-4).indexOf(k) < 0) return;
      (map[p.letter] = map[p.letter] || []).push({
        idx, name: p.name, bank: p.bank, card: p.card, tail: p.card.slice(-4), first: p.name.slice(0, 1), color: COLORS[idx % COLORS.length]
      });
    });
    const letters = Object.keys(map).sort();
    const groups = letters.map(l => ({ letter: l, items: map[l] }));
    this.setData({ kw, groups, letters: k ? this.data.letters : letters });
  },

  onSearch(e) { this.build(e.detail.value); },
  clearKw() { this.build(''); },

  jump(e) {
    const l = e.currentTarget.dataset.l;
    // 已知引擎问题：scroll-into-view（或 scroll-top）程序滚动之后，后面的点击全部不触发；
    // 微信里这样写是对的，所以保留标准写法，问题记在交付说明里
    this.setData({ intoView: 'g-' + l, bigLetter: l });
    clearTimeout(this.lt);
    this.lt = setTimeout(() => this.setData({ bigLetter: '' }), 600);
  },

  pick(e) {
    const i = e.currentTarget.dataset.i;
    const p = S.PAYEES[i];
    if (this.mode !== 'pick') return wx.redirectTo({ url: '/pages/transfer/transfer?pi=' + i });
    const ch = typeof this.getOpenerEventChannel === 'function' ? this.getOpenerEventChannel() : null;
    if (ch && ch.emit) ch.emit('pick', p);
    S.set('picked', p);
    wx.navigateBack();
  },

  onUnload() { clearTimeout(this.lt); }
});
