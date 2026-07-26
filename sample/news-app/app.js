// 头条新闻 · 应用入口
//
// 阅读历史、收藏、关注、设置（夜间模式/自动播放/正文字号）、搜索历史全部落 storage，
// 这样跨页面与编译到 H5 后都能保持一致。
App({
  globalData: {
    appName: '头条新闻'
  },

  onLaunch: function () {
    console.log('📰 头条新闻 启动');
    this.settings = this.getSettings();
  },

  // ==================== 设置 ====================

  getSettings: function () {
    var s = wx.getStorageSync('settings');
    if (!s || typeof s !== 'object') {
      s = { nightMode: false, autoPlay: true, fontLevel: 3 };
    }
    return s;
  },

  saveSettings: function (patch) {
    var s = this.getSettings();
    for (var k in patch) {
      if (patch.hasOwnProperty(k)) { s[k] = patch[k]; }
    }
    wx.setStorageSync('settings', s);
    return s;
  },

  /// 正文字号：1~5 档 → px
  fontSizeOf: function (level) {
    var map = { 1: 14, 2: 15, 3: 17, 4: 19, 5: 21 };
    return map[level] || 17;
  },

  // ==================== 阅读历史 ====================

  getHistory: function () {
    return wx.getStorageSync('history') || [];
  },

  pushHistory: function (article) {
    if (!article || !article.id) { return []; }
    var list = this.getHistory().filter(function (a) { return a.id !== article.id; });
    list.unshift({ id: article.id, title: article.title, source: article.source, time: this.now() });
    if (list.length > 30) { list = list.slice(0, 30); }
    wx.setStorageSync('history', list);
    return list;
  },

  // ==================== 收藏 / 关注 / 点赞 ====================

  getFavorites: function () {
    return wx.getStorageSync('favorites') || [];
  },

  isFavorite: function (id) {
    return this.getFavorites().some(function (a) { return a.id === id; });
  },

  toggleFavorite: function (article) {
    var list = this.getFavorites();
    var exists = list.some(function (a) { return a.id === article.id; });
    if (exists) {
      list = list.filter(function (a) { return a.id !== article.id; });
    } else {
      list.unshift({ id: article.id, title: article.title, source: article.source });
    }
    wx.setStorageSync('favorites', list);
    return !exists;
  },

  getFollows: function () {
    return wx.getStorageSync('follows') || [];
  },

  isFollowed: function (author) {
    return this.getFollows().indexOf(author) >= 0;
  },

  toggleFollow: function (author) {
    var list = this.getFollows();
    var idx = list.indexOf(author);
    if (idx >= 0) { list.splice(idx, 1); } else { list.push(author); }
    wx.setStorageSync('follows', list);
    return idx < 0;
  },

  // ==================== 评论 ====================

  getComments: function (articleId) {
    var all = wx.getStorageSync('comments') || {};
    return all['a' + articleId] || [];
  },

  addComment: function (articleId, text) {
    var all = wx.getStorageSync('comments') || {};
    var key = 'a' + articleId;
    var list = all[key] || [];
    list.unshift({ id: Date.now(), user: '我', avatar: '/assets/avatar1.jpg', text: text, time: '刚刚', likes: 0, mine: true });
    all[key] = list;
    wx.setStorageSync('comments', all);
    return list;
  },

  // ==================== 搜索历史 ====================

  getSearchHistory: function () {
    return wx.getStorageSync('searchHistory') || [];
  },

  pushSearchHistory: function (kw) {
    if (!kw) { return this.getSearchHistory(); }
    var list = this.getSearchHistory().filter(function (k) { return k !== kw; });
    list.unshift(kw);
    if (list.length > 10) { list = list.slice(0, 10); }
    wx.setStorageSync('searchHistory', list);
    return list;
  },

  clearSearchHistory: function () {
    wx.setStorageSync('searchHistory', []);
    return [];
  },

  // ==================== 签到 ====================

  getCheckin: function () {
    return wx.getStorageSync('checkin') || { days: [], streak: 0 };
  },

  doCheckin: function (day) {
    var c = this.getCheckin();
    if (c.days.indexOf(day) < 0) {
      c.days.push(day);
      c.streak = c.days.length;
      wx.setStorageSync('checkin', c);
      return true;
    }
    return false;
  },

  now: function () {
    var d = new Date();
    var p = function (n) { return n < 10 ? '0' + n : '' + n; };
    return p(d.getHours()) + ':' + p(d.getMinutes());
  }
});
