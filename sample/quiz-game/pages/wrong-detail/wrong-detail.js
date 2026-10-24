const G = require('../../utils/game.js');
const QS = require('../../utils/questions.js');

const L = ['A', 'B', 'C', 'D'];

Page({
  data: { pos: 0, count: 0, q: {}, opts: [], times: 1, catName: '', c: '', bg: '', showExp: true },

  onLoad(options) {
    const all = G.wrongs().map(w => w.id);
    this.ids = options.ids ? options.ids.split(',').filter(id => all.indexOf(id) >= 0) : all;
    if (!this.ids.length && options.id) this.ids = [options.id];
    const pos = Math.max(0, this.ids.indexOf(options.id));
    this.render(pos);
  },

  render(pos) {
    const id = this.ids[pos];
    const q = QS.find(id) || QS.Q[0];
    const w = G.wrongs().find(x => x.id === id) || { picked: -1, times: 1 };
    const c = QS.catOf(q.cat);
    this.setData({
      pos, count: this.ids.length, q, times: w.times || 1, catName: c.t, c: c.c, bg: QS.tint(c.c, 0.12),
      opts: q.opts.map((t, i) => ({ l: L[i], t, cls: i === q.a ? 'right' : i === w.picked ? 'wrong' : '', tag: i === q.a ? '正确答案' : i === w.picked ? '我的选择' : '' }))
    });
  },

  toggleExp() { this.setData({ showExp: !this.data.showExp }); },
  prev() { if (this.data.pos > 0) this.render(this.data.pos - 1); },
  next() { if (this.data.pos + 1 < this.ids.length) this.render(this.data.pos + 1); },

  remove() {
    const id = this.ids[this.data.pos];
    G.removeWrong(id);
    this.ids.splice(this.data.pos, 1);
    wx.showToast({ title: '已移出错题本', icon: 'none' });
    if (!this.ids.length) return setTimeout(() => wx.navigateBack(), 600);
    this.render(Math.min(this.data.pos, this.ids.length - 1));
  }
});
