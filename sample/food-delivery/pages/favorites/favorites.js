const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: {
    list: []
  },
  // 店铺页里取消 / 新加收藏，回到这里要跟上
  onShow() {
    this.load();
  },
  load() {
    const ids = store.getFavs();
    const list = ids.map(id => data.shops.find(s => s.id === id)).filter(Boolean);
    this.setData({ list });
  },
  onShop(e) {
    wx.navigateTo({ url: `/pages/shop/shop?id=${e.currentTarget.dataset.id}` });
  },
  onUnfav(e) {
    const id = String(e.currentTarget.dataset.id);
    const shop = data.findShop(id);
    wx.showModal({
      title: '取消收藏',
      content: `不再收藏「${shop.name}」？`,
      confirmText: '取消收藏',
      cancelText: '再想想',
      success: r => {
        if (!r.confirm) return;
        if (store.getFavs().indexOf(id) >= 0) store.toggleFav(id);
        this.load();
        wx.showToast({ title: '已取消收藏', icon: 'none' });
      }
    });
  },
  onGoHome() {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
