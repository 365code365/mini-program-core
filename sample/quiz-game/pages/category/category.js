const G = require('../../utils/game.js');
const QS = require('../../utils/questions.js');

Page({
  data: { cats: [], selCount: 0, qCount: 0, allOn: false, energy: 0 },

  onLoad() { this.sel = G.cats().slice(); },
  onShow() {
    const wrongIds = G.wrongs().map(w => w.id);
    this.base = QS.CATS.map(c => {
      const qs = QS.byCats([c.k]);
      const wrong = qs.filter(q => wrongIds.indexOf(q.id) >= 0).length;
      return { k: c.k, t: c.t, g: c.g, c: c.c, n: qs.length, wrong, acc: Math.round((qs.length - wrong) / qs.length * 100) };
    });
    this.setData({ energy: G.energy().energy });
    this.render();
  },

  render() {
    const sel = this.sel;
    this.setData({
      cats: this.base.map(c => Object.assign({}, c, { on: sel.indexOf(c.k) >= 0 })),
      selCount: sel.length,
      qCount: QS.byCats(sel).length,
      allOn: sel.length === QS.CATS.length
    });
  },

  onChange(e) {
    // 按题库原顺序保存，免得勾选顺序影响显示
    const v = e.detail.value;
    this.sel = QS.CATS.map(c => c.k).filter(k => v.indexOf(k) >= 0);
    this.render();
  },
  toggleAll() {
    this.sel = this.data.allOn ? [] : QS.CATS.map(c => c.k);
    this.render();
  },

  start() {
    if (!this.sel.length) return wx.showToast({ title: '请至少选择 1 个题库', icon: 'none' });
    G.set('cats', this.sel);
    if (!G.useEnergy()) return wx.showModal({ title: '体力不足', content: '题库已保存。体力每 5 分钟恢复 1 点，回首页可补充体力', showCancel: false });
    wx.redirectTo({ url: '/pages/quiz/quiz' });
  },

  onUnload() {
    // 没开局直接返回也记住选择
    if (this.sel.length) G.set('cats', this.sel);
  }
});
