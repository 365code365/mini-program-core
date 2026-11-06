// 写短评：打星（必填）+ 正文（5~300 字），存在本地（演示接口只读），详情页与全部短评里排在最前面。每部影片一条，再进来是修改
const { request } = require('../../utils/api.js');
const store = require('../../utils/store.js');
const STAR_TEXT = ['', '很差', '较差', '还行', '推荐', '力荐'];
Page({
  data: { status: 'loading', m: null, starList: [1, 2, 3, 4, 5], star: 0, starText: '点击星星评分', content: '', editing: false },
  onLoad(q) {
    this.id = Number(q.id) || 1;
    const old = store.myCommentOf(this.id);
    if (old) this.setData({ editing: true, star: old.star, starText: STAR_TEXT[old.star], content: old.content });
    wx.setNavigationBarTitle({ title: old ? '修改短评' : '写短评' });
    this.load();
  },
  load() {
    this.setData({ status: 'loading' });
    request('/api/demo/movies/' + this.id, null, { silent: true }).then((m) => {
      if (m.status === 'coming' && !this.data.editing) { this.setData({ status: 'coming', m: { id: m.id, title: m.title, releaseDate: m.releaseDate } }); return; }
      this.setData({ status: 'ok', m: { id: m.id, title: m.title, poster: m.poster, genreText: m.genreText, releaseDate: m.releaseDate, status: m.status } });
    }).catch(() => this.setData({ status: 'error' }));
  },
  onRetry() { this.load(); },
  onBack() { wx.navigateBack(); },
  onStar(e) {
    const star = Number(e.currentTarget.dataset.n);
    this.setData({ star, starText: STAR_TEXT[star] });
  },
  onInput(e) { this.setData({ content: e.detail.value }); },
  onSubmit() {
    const content = this.data.content.trim();
    if (!this.data.star) { wx.showToast({ title: '请先打个分', icon: 'none' }); return; }
    if (content.length < 5) { wx.showToast({ title: '至少写 5 个字', icon: 'none' }); return; }
    store.saveComment(this.data.m, this.data.star, content);
    wx.showToast({ title: this.data.editing ? '已修改' : '发表成功', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 600);
  },
  onDelete() {
    const c = store.myCommentOf(this.id);
    if (!c) return;
    wx.showModal({
      title: '删除短评', content: '删除后不能恢复', confirmText: '删除', confirmColor: '#ff4e6a',
      success: (r) => {
        if (!r.confirm) return;
        store.removeComment(c.id);
        wx.showToast({ title: '已删除', icon: 'none' });
        setTimeout(() => wx.navigateBack(), 600);
      }
    });
  }
});
