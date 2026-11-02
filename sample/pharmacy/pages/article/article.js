// 健康资讯详情：正文、文中提到的药品（可直接看详情）、「有用」反馈（存 storage，同一篇只能点一次）
const { ARTICLES, BY_ID, card } = require('../../utils/drugs.js');
const S = require('../../utils/store.js');

Page({
  data: { a: null, drugs: [], liked: false },
  onLoad(q) {
    const a = ARTICLES.find(x => x.id === q.id) || ARTICLES[0];
    wx.setNavigationBarTitle({ title: a.tag });
    this.setData({ a, drugs: a.drugs.map(id => card(BY_ID[id])), liked: S.get('ph_liked', []).indexOf(a.id) >= 0 });
  },
  like() {
    if (this.data.liked) { wx.showToast({ title: '已经反馈过了', icon: 'none' }); return; }
    S.set('ph_liked', S.get('ph_liked', []).concat(this.data.a.id));
    this.setData({ liked: true });
    wx.showToast({ title: '感谢反馈', icon: 'success' });
  },
  openDrug(e) { wx.navigateTo({ url: '/pages/drug/drug?id=' + e.currentTarget.dataset.id }); },
  remind() { wx.navigateTo({ url: '/pages/remind-edit/remind-edit' }); },
  onShareAppMessage() { return { title: this.data.a.title, path: '/pages/article/article?id=' + this.data.a.id }; }
});
