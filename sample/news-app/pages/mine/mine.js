// 头条新闻 · 我的（设置持久化 / 入口跳转）
Page({
  data: {
    nightMode: false,
    autoPlay: true,
    fontLevel: 3,
    fontSample: 17,
    user: {
      name: '头条读者',
      desc: '已连续阅读 128 天',
      avatar: '/assets/avatar1.jpg'
    },
    stats: [],
    menus: []
  },

  onShow: function () {
    var app = getApp();
    var s = app.getSettings();
    var favorites = app.getFavorites();
    var history = app.getHistory();
    var follows = app.getFollows();
    var checkin = app.getCheckin();
    this.setData({
      nightMode: !!s.nightMode,
      autoPlay: !!s.autoPlay,
      fontLevel: s.fontLevel || 3,
      fontSample: app.fontSizeOf(s.fontLevel || 3),
      user: {
        name: '头条读者',
        desc: '已连续签到 ' + (checkin.streak || 0) + ' 天 · 阅读 ' + history.length + ' 篇',
        avatar: '/assets/avatar1.jpg'
      },
      stats: [
        { key: 'follow', label: '关注', value: follows.length },
        { key: 'fav', label: '收藏', value: favorites.length },
        { key: 'history', label: '历史', value: history.length },
        { key: 'hot', label: '热榜', value: '榜' }
      ],
      menus: [
        { id: 1, key: 'fav', name: '我的收藏', icon: 'star', color: '#D43C33', badge: favorites.length ? '' + favorites.length : '' },
        { id: 2, key: 'history', name: '阅读历史', icon: 'clock', color: '#F0A020', badge: '' },
        { id: 3, key: 'search', name: '搜索发现', icon: 'search', color: '#3A7BD5', badge: '' },
        { id: 4, key: 'hot', name: '热榜与签到', icon: 'arrow_up', color: '#7B61FF', badge: checkin.days && checkin.days.length ? '已签' : '未签' }
      ]
    });
  },

  onLoginTap: function () {
    wx.showToast({ title: '当前为演示账号', icon: 'none' });
  },

  onStatTap: function (e) {
    this.route(e.currentTarget.dataset.key);
  },

  onMenuTap: function (e) {
    this.route(e.currentTarget.dataset.key);
  },

  route: function (key) {
    if (key === 'fav' || key === 'hot' || key === 'history') {
      wx.navigateTo({ url: '/pages/hot/hot' });
    } else if (key === 'search') {
      wx.navigateTo({ url: '/pages/search/search' });
    } else if (key === 'follow') {
      var follows = getApp().getFollows();
      wx.showToast({ title: follows.length ? '已关注：' + follows.join('、') : '还没有关注作者', icon: 'none' });
    }
  },

  onNightModeChange: function (e) {
    getApp().saveSettings({ nightMode: e.detail.value });
    this.setData({ nightMode: e.detail.value });
    wx.showToast({ title: e.detail.value ? '正文将使用夜间配色' : '已关闭夜间模式', icon: 'none' });
  },

  onAutoPlayChange: function (e) {
    getApp().saveSettings({ autoPlay: e.detail.value });
    this.setData({ autoPlay: e.detail.value });
  },

  onFontLevelChange: function (e) {
    var app = getApp();
    var level = Number(e.detail.value) || 3;
    app.saveSettings({ fontLevel: level });
    this.setData({ fontLevel: level, fontSample: app.fontSizeOf(level) });
  },

  onLogout: function () {
    var self = this;
    wx.showModal({
      title: '清除本地数据',
      content: '将清空收藏、历史、评论与设置，确定继续？',
      success: function (res) {
        if (res.confirm) {
          ['favorites', 'history', 'follows', 'comments', 'settings', 'searchHistory', 'checkin'].forEach(function (k) {
            wx.removeStorageSync(k);
          });
          self.onShow();
          wx.showToast({ title: '已清除', icon: 'success' });
        }
      }
    });
  }
});
