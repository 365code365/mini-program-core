const store = require('../../utils/store.js');
const KN = require('../../utils/knowledge.js');

Page({
  data: { a: null, fav: false, stageName: '', related: [] },

  onLoad(q) {
    const a = KN.byId(q.id) || KN.ARTICLES[0];
    const stage = KN.STAGES.find((s) => s.key === a.stage);
    const related = KN.ARTICLES.filter((x) => x.stage === a.stage && x.id !== a.id);
    this.setData({ a, stageName: stage ? stage.name : '', related, fav: store.get(store.K.favs, []).indexOf(a.id) >= 0 });
    wx.setNavigationBarTitle({ title: a.tag + ' · ' + (stage ? stage.name : '') });
  },

  toggleFav() {
    const id = this.data.a.id;
    let favs = store.get(store.K.favs, []);
    const fav = favs.indexOf(id) < 0;
    favs = fav ? [id].concat(favs) : favs.filter((x) => x !== id);
    store.set(store.K.favs, favs);
    this.setData({ fav });
    wx.vibrateShort({ type: 'light' });
    wx.showToast({ title: fav ? '已收藏' : '已取消收藏', icon: 'none' });
  },

  open(e) { wx.redirectTo({ url: '/pages/article/article?id=' + e.currentTarget.dataset.id }); },

  onShareAppMessage() {
    return { title: this.data.a.title, path: '/pages/article/article?id=' + this.data.a.id };
  }
});
