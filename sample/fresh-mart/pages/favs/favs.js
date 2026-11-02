const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: {
    list: []
  },
  onShow() {
    this.load();
  },
  load() {
    const list = store.getFavs().map(id => {
      const gd = data.findGoods(id);
      const f = data.flashOf(id);
      return { id, name: gd.name, spec: gd.spec, img: gd.img, price: f ? f.price : gd.price, origin: gd.origin, flash: !!f, tags: gd.tags || [] };
    });
    this.setData({ list });
  },
  onDetail(e) {
    wx.navigateTo({ url: `/pages/detail/detail?id=${e.currentTarget.dataset.id}` });
  },
  onAdd(e) {
    const r = store.addToCart(e.currentTarget.dataset.id, 1);
    wx.showToast({ title: r.ok ? r.msg || `已加入购物车（${r.count}）` : r.msg, icon: 'none' });
  },
  onRemove(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '取消收藏',
      content: '确定不再收藏这件商品吗？',
      confirmText: '取消收藏',
      success: r => {
        if (!r.confirm) return;
        if (store.isFav(id)) store.toggleFav(id);
        this.load();
        wx.showToast({ title: '已取消收藏', icon: 'none' });
      }
    });
  },
  onGo() {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
