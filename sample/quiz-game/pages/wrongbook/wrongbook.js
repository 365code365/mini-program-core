const G = require('../../utils/game.js');
const QS = require('../../utils/questions.js');

const L = ['A', 'B', 'C', 'D'];

// 第一次打开时放几道示例错题，页面不至于空
function seed() {
  if (G.get('wrongSeeded', false)) return;
  G.set('wrongSeeded', true);
  if (G.wrongs().length) return;
  const now = Date.now();
  G.set('wrongs', [
    { id: 'q6', picked: 1, time: now - 3600e3, times: 2 },
    { id: 'q12', picked: 0, time: now - 7200e3, times: 1 },
    { id: 'q18', picked: -1, time: now - 86400e3, times: 1 },
    { id: 'q22', picked: 0, time: now - 2 * 86400e3, times: 3 },
    { id: 'q34', picked: 1, time: now - 3 * 86400e3, times: 1 }
  ]);
}

function when(t) {
  const d = Date.now() - t;
  if (d < 3600e3) return '刚刚';
  if (d < 86400e3) return Math.floor(d / 3600e3) + ' 小时前';
  return Math.floor(d / 86400e3) + ' 天前';
}

Page({
  data: { list: [], all: 0, filters: [], cat: 'all' },

  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    seed();
    this.open = this.open || {};
    this.load();
  },

  load() {
    const ws = G.wrongs();
    const counts = {};
    ws.forEach(w => { const q = QS.find(w.id); if (q) counts[q.cat] = (counts[q.cat] || 0) + 1; });
    const filters = [{ k: 'all', t: '全部', n: ws.length }].concat(QS.CATS.filter(c => counts[c.k]).map(c => ({ k: c.k, t: c.t, n: counts[c.k] })));
    let cat = this.data.cat;
    if (cat !== 'all' && !counts[cat]) cat = 'all';
    const list = ws.map(w => {
      const q = QS.find(w.id);
      if (!q || (cat !== 'all' && q.cat !== cat)) return null;
      const c = QS.catOf(q.cat);
      return {
        id: q.id, q: q.q, exp: q.exp, catName: c.t, c: c.c, bg: QS.tint(c.c, 0.12), times: w.times || 1, when: when(w.time),
        right: L[q.a] + '. ' + q.opts[q.a], mine: w.picked >= 0 ? L[w.picked] + '. ' + q.opts[w.picked] : '超时未作答',
        open: !!this.open[q.id]
      };
    }).filter(Boolean);
    this.setData({ list, all: ws.length, filters, cat });
  },

  onFilter(e) { this.setData({ cat: e.currentTarget.dataset.k }); this.load(); },

  toggle(e) {
    const id = e.currentTarget.dataset.id;
    this.open[id] = !this.open[id];
    const i = this.data.list.findIndex(x => x.id === id);
    this.setData({ ['list[' + i + '].open']: this.open[id] });
  },

  remove(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '移出错题本',
      content: '确认已经掌握这道题了吗？',
      confirmText: '移除',
      success: r => {
        if (!r.confirm) return;
        G.removeWrong(id);
        delete this.open[id];
        this.load();
        wx.showToast({ title: '已移除', icon: 'success' });
      }
    });
  },

  clearAll() {
    wx.showModal({
      title: '清空错题本',
      content: '将删除全部 ' + this.data.all + ' 道错题，不可恢复',
      confirmText: '清空',
      confirmColor: '#F43F5E',
      success: r => {
        if (!r.confirm) return;
        G.set('wrongs', []);
        this.open = {};
        this.load();
      }
    });
  },

  detail(e) {
    const ids = this.data.list.map(x => x.id).join(',');
    wx.navigateTo({ url: '/pages/wrong-detail/wrong-detail?id=' + e.currentTarget.dataset.id + '&ids=' + ids });
  },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
