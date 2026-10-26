// 拾光图库：图片列表都从演示接口拉；点赞 / 收藏存 storage
const LIKE_KEY = 'photo_likes';
const FAV_KEY = 'photo_favs';

App({
  // previewUrls：发现页当前列表的大图地址，详情页点大图时用它做 wx.previewImage 的 urls（能左右翻整列）
  // pendingTag：详情页点标签 → 回发现页并切到这个标签（switchTab 不能带 query）
  globalData: { previewUrls: [], pendingTag: null },
  likes() { return wx.getStorageSync(LIKE_KEY) || {}; },
  isLiked(id) { return !!this.likes()[id]; },
  toggleLike(id) {
    const l = this.likes();
    if (l[id]) delete l[id];
    else l[id] = Date.now();
    wx.setStorageSync(LIKE_KEY, l);
    return !!l[id];
  },
  favs() { return wx.getStorageSync(FAV_KEY) || []; },
  isFav(id) { return this.favs().some((p) => p.id === id); },
  toggleFav(p) {
    let list = this.favs();
    const on = !list.some((x) => x.id === p.id);
    list = on ? [{ id: p.id, title: p.title, url: p.url, width: p.width, height: p.height, ratio: p.ratio, color: p.color,
      author: p.author, likes: p.likes, savedAt: Date.now() }].concat(list) : list.filter((x) => x.id !== p.id);
    wx.setStorageSync(FAV_KEY, list);
    return on;
  },
  removeFav(id) { wx.setStorageSync(FAV_KEY, this.favs().filter((x) => x.id !== id)); }
});
