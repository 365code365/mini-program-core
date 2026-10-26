// 光影放映厅：所有数据都从演示接口拉（utils/api.js），本地只存「想看」清单
const WISH_KEY = 'movie_wish';

App({
  globalData: {
    // 云听音乐的 AppID：详情页「听原声」用 wx.navigateToMiniProgram 跳过去
    musicAppId: 'wxc0bec341b9d3e883'
  },
  getWish() {
    return wx.getStorageSync(WISH_KEY) || [];
  },
  isWished(id) {
    return this.getWish().some((m) => m.id === id);
  },
  // 存卡片要的几个字段而不是只存 id：「我的」页离线也能画出来，不用再逐个请求详情
  toggleWish(movie) {
    let list = this.getWish();
    const on = !list.some((m) => m.id === movie.id);
    if (on) {
      list = [{ id: movie.id, title: movie.title, poster: movie.poster, score: movie.score, genreText: movie.genreText,
        releaseDate: movie.releaseDate, status: movie.status, addedAt: Date.now() }].concat(list);
    } else {
      list = list.filter((m) => m.id !== movie.id);
    }
    wx.setStorageSync(WISH_KEY, list);
    return on;
  },
  clearWish() {
    wx.removeStorageSync(WISH_KEY);
  }
});
