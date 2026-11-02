const { getItem, themeOf } = require('../../utils/items');
const guide = require('../../utils/guide');

Page({
  data: { item: null, theme: null, fav: false, tab: 'cond' },
  onLoad(q) {
    const item = getItem(q.id) || getItem('G0301');
    const favs = wx.getStorageSync('gov_favs') || [];
    this.setData({ item, theme: themeOf(item.theme), fav: favs.indexOf(item.id) >= 0 });
    wx.setNavigationBarTitle({ title: item.name });
  },
  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); },
  toggleFav() {
    const id = this.data.item.id;
    let favs = wx.getStorageSync('gov_favs') || [];
    const fav = favs.indexOf(id) < 0;
    favs = fav ? favs.concat(id) : favs.filter(x => x !== id);
    wx.setStorageSync('gov_favs', favs);
    this.setData({ fav });
    wx.showToast({ title: fav ? '已收藏，可在「我的」查看' : '已取消收藏', icon: 'none' });
  },
  download() { guide.open(this.data.item.id); },
  appoint() { wx.navigateTo({ url: '/pages/appoint/appoint?item=' + this.data.item.id }); },
  apply() {
    const it = this.data.item;
    if (!it.online) {
      wx.showModal({
        title: '需到窗口办理', content: `「${it.name}」需本人到场采集信息，可先预约取号，到场免排队。`,
        confirmText: '去预约', success: (r) => { if (r.confirm) this.appoint(); }
      });
      return;
    }
    wx.navigateTo({ url: '/pages/apply/apply?id=' + it.id });
  }
});
