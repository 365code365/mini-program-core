// 收藏夹：storage 里的收藏，双列瀑布流；点开详情、长按取消收藏
const app = getApp();
const COL_W = 345;
const INFO_H = 104;

Page({
  data: { left: [], right: [], count: 0 },
  onShow() {
    const favs = app.favs();
    const left = [];
    const right = [];
    let hl = 0;
    let hr = 0;
    favs.forEach((p) => {
      const card = Object.assign({}, p, { h: Math.round(COL_W * p.ratio) });
      if (hl <= hr) { left.push(card); hl += card.h + INFO_H; } else { right.push(card); hr += card.h + INFO_H; }
    });
    // 预览时左右翻的是整个收藏夹
    this.urls = favs.map((p) => p.url);
    this.setData({ left, right, count: favs.length });
  },
  onPhoto(e) {
    app.globalData.previewUrls = this.urls;
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },
  onLong(e) {
    const id = e.currentTarget.dataset.id;
    wx.showActionSheet({
      itemList: ['全屏预览', '取消收藏'],
      success: (r) => {
        if (r.tapIndex === 0) {
          const p = app.favs().find((x) => x.id === id);
          if (p) wx.previewImage({ urls: this.urls, current: p.url });
        } else {
          app.removeFav(id);
          this.onShow();
          wx.showToast({ title: '已取消收藏', icon: 'none' });
        }
      }
    });
  },
  onClear() {
    wx.showModal({
      title: '清空收藏',
      content: '确定清空全部 ' + this.data.count + ' 张收藏？',
      success: (r) => {
        if (!r.confirm) return;
        wx.removeStorageSync('photo_favs');
        this.onShow();
      }
    });
  },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
