// 我的：想看清单（按 全部 / 待上映 / 可观看 筛）、我写的短评（修改 / 删除）、演示接口地址
const api = require('../../utils/api.js');
const store = require('../../utils/store.js');
Page({
  data: { all: [], list: [], coming: 0, filter: 'all', comments: [], starList: [1, 2, 3, 4, 5], base: api.BASE },
  onShow() {
    const all = store.getWish().map((m) => Object.assign({}, m, {
      scoreText: m.score ? m.score.toFixed(1) : '',
      statusText: m.status === 'coming' ? m.releaseDate.slice(5).replace('-', '月') + '日上映' : (m.status === 'now' ? '热映中' : '经典')
    }));
    const comments = store.myComments();
    this.setData({ all, coming: all.filter((m) => m.status === 'coming').length, comments });
    this.applyFilter();
  },
  applyFilter() {
    const f = this.data.filter;
    const list = this.data.all.filter((m) => f === 'all' || (f === 'coming' ? m.status === 'coming' : m.status !== 'coming'));
    this.setData({ list });
  },
  onFilter(e) {
    this.setData({ filter: e.currentTarget.dataset.f });
    this.applyFilter();
  },
  onMyComments() {
    if (!this.data.comments.length) { wx.showToast({ title: '还没写过短评，去影片详情页写一条吧', icon: 'none' }); return; }
    wx.pageScrollTo({ selector: '.cmt-card', duration: 200 });
  },
  onMovie(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onEditComment(e) { wx.navigateTo({ url: '/pages/write/write?id=' + e.currentTarget.dataset.id }); },
  onDelComment(e) {
    const c = this.data.comments[Number(e.currentTarget.dataset.index)];
    wx.showModal({
      title: '删除短评', content: '删除《' + c.movieTitle + '》的短评？', confirmText: '删除', confirmColor: '#ff4e6a',
      success: (r) => {
        if (!r.confirm) return;
        store.removeComment(c.id);
        wx.showToast({ title: '已删除', icon: 'none' });
        this.onShow();
      }
    });
  },
  onRemove(e) {
    const m = this.data.list[Number(e.currentTarget.dataset.index)];
    wx.showModal({
      title: '取消想看',
      content: '把《' + m.title + '》移出想看清单？',
      success: (r) => {
        if (!r.confirm) return;
        store.toggleWish(m);
        this.onShow();
      }
    });
  },
  onClear() {
    if (!this.data.all.length) return;
    wx.showModal({
      title: '清空想看',
      content: '清空全部 ' + this.data.all.length + ' 部？',
      confirmColor: '#ff4e6a',
      success: (r) => {
        if (r.confirm) { store.clearWish(); this.onShow(); }
      }
    });
  },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
