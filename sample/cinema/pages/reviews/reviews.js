// 我的影评：列表、编辑、删除；空态引导去「已观影」的票写一条
const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { list: [], starList: [1, 2, 3, 4, 5] },
  onShow() { this.load(); },
  load() {
    const list = store.reviews().map(r => {
      const m = D.getMovie(r.mid);
      return Object.assign({}, r, { title: m.title, poster: m.poster, starText: D.STAR_TEXT[r.star], date: store.dateTime(r.ts).slice(0, 10) });
    });
    this.setData({ list });
  },
  onEdit(e) { wx.navigateTo({ url: '/pages/review/review?id=' + e.currentTarget.dataset.id }); },
  onMovie(e) { wx.navigateTo({ url: '/pages/movie/movie?id=' + e.currentTarget.dataset.mid }); },
  onDelete(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '删除影评', content: '删除后不能恢复', confirmText: '删除', confirmColor: '#E8243F',
      success: r => {
        if (!r.confirm) return;
        store.removeReview(id);
        wx.showToast({ title: '已删除', icon: 'none' });
        this.load();
      }
    });
  },
  onGo() {
    wx.setStorageSync(store.K.ticketTab, 'used');
    wx.switchTab({ url: '/pages/tickets/tickets' });
  }
});
