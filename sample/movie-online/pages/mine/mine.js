// 我的：想看清单（storage）+ 统计 + 演示接口地址
const api = require('../../utils/api.js');
const app = getApp();

Page({
  data: { list: [], coming: 0, base: api.BASE },
  onShow() {
    const list = app.getWish().map((m) => Object.assign({}, m, {
      scoreText: m.score ? m.score.toFixed(1) : '',
      statusText: m.status === 'coming' ? m.releaseDate.slice(5).replace('-', '月') + '日上映' : (m.status === 'now' ? '热映中' : '经典')
    }));
    this.setData({ list, coming: list.filter((m) => m.status === 'coming').length });
  },
  onMovie(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onRemove(e) {
    const m = this.data.list[e.currentTarget.dataset.index];
    wx.showModal({
      title: '取消想看',
      content: '把《' + m.title + '》移出想看清单？',
      success: (r) => {
        if (!r.confirm) return;
        app.toggleWish(m);
        this.onShow();
      }
    });
  },
  onClear() {
    if (!this.data.list.length) return;
    wx.showModal({
      title: '清空想看',
      content: '清空全部 ' + this.data.list.length + ' 部？',
      confirmColor: '#ff4e6a',
      success: (r) => {
        if (r.confirm) { app.clearWish(); this.onShow(); }
      }
    });
  },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
