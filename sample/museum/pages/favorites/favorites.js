// 我的收藏 / 浏览足迹：两份都在 storage（mu_fav 是展品 id 列表，mu_history 带浏览时间）
const U = require('../../utils/util.js');

function ago(t) {
  const m = Math.floor((Date.now() - t) / 60000);
  if (m < 1) return '刚刚';
  if (m < 60) return m + ' 分钟前';
  if (m < 1440) return Math.floor(m / 60) + ' 小时前';
  return Math.floor(m / 1440) + ' 天前';
}

Page({
  data: { tab: 0, fav: [], his: [] },
  onLoad(q) { if (q.tab === '1') this.setData({ tab: 1 }); },
  onShow() { this.refresh(); },
  refresh() {
    const fav = U.load('fav', []).map(id => U.exhibit(id)).filter(Boolean);
    const his = U.load('history', []).map(h => Object.assign(U.exhibit(h.id) || {}, { ago: ago(h.at) })).filter(x => x.id);
    this.setData({ fav, his });
  },
  onTab(e) { this.setData({ tab: +e.currentTarget.dataset.t }); },
  open(e) { wx.navigateTo({ url: '/pages/exhibit/exhibit?id=' + e.currentTarget.dataset.id }); },
  unfav(e) {
    U.toggleFav(e.currentTarget.dataset.id);
    this.refresh();
    wx.showToast({ title: '已取消收藏', icon: 'none' });
  },
  clearHis() {
    wx.showModal({
      title: '清空足迹', content: '清空后无法恢复，确定清空全部浏览足迹吗？', confirmText: '清空', confirmColor: '#c0392b',
      success: r => { if (r.confirm) { U.save('history', []); this.refresh(); } }
    });
  },
  goHalls() { wx.switchTab({ url: '/pages/halls/halls' }); }
});
