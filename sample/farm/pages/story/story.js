const { stories, farmers, goodsById } = require('../../utils/data.js');

Page({
  data: { s: {}, farmer: {}, goods: [], followed: false },
  onLoad(q) {
    const s = stories.find((x) => x.id === q.id) || stories[0];
    const followed = (wx.getStorageSync('farm_follow') || []).indexOf(s.farmer) >= 0;
    wx.setNavigationBarTitle({ title: s.title.split('：')[0] });
    this.setData({ s, farmer: farmers[s.farmer], goods: s.goods.map(goodsById), followed });
  },
  toggleFollow() {
    const list = wx.getStorageSync('farm_follow') || [];
    const id = this.data.s.farmer;
    const i = list.indexOf(id);
    if (i >= 0) list.splice(i, 1); else list.push(id);
    wx.setStorageSync('farm_follow', list);
    this.setData({ followed: i < 0 });
    wx.showToast({ title: i < 0 ? '已关注，开播会提醒你' : '已取消关注', icon: i < 0 ? 'success' : 'none' });
  },
  openGoods(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onShareAppMessage() { return { title: this.data.s.title, path: '/pages/story/story?id=' + this.data.s.id }; }
});
