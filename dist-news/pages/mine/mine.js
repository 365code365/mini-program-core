// 头条新闻 · 我的
Page({
  data: {
    nightMode: false,
    autoPlay: true,
    fontLevel: 3,
    user: {
      name: '头条读者',
      desc: '已连续阅读 128 天',
      avatar: '/assets/avatar1.jpg'
    },
    stats: [
      { label: '关注', value: 42 },
      { label: '收藏', value: 186 },
      { label: '历史', value: '1.2万' },
      { label: '评论', value: 73 }
    ],
    menus: [
      { id: 1, name: '我的收藏', icon: 'success', color: '#D43C33', badge: '' },
      { id: 2, name: '阅读历史', icon: 'waiting', color: '#F0A020', badge: '' },
      { id: 3, name: '消息通知', icon: 'info', color: '#3A7BD5', badge: '9' },
      { id: 4, name: '意见反馈', icon: 'warn', color: '#7B61FF', badge: '' }
    ]
  },

  onLoad: function () {
    console.log('👤 我的页面加载');
  },

  onLoginTap: function () {
    wx.showToast({ title: '进入账号页', icon: 'none' });
  },

  onStatTap: function (e) {
    wx.showToast({ title: e.currentTarget.dataset.label, icon: 'none' });
  },

  onMenuTap: function (e) {
    wx.showToast({ title: '菜单 ' + e.currentTarget.dataset.id, icon: 'none' });
  },

  onNightModeChange: function (e) {
    this.setData({ nightMode: e.detail.value });
  },

  onAutoPlayChange: function (e) {
    this.setData({ autoPlay: e.detail.value });
  },

  onFontLevelChange: function (e) {
    this.setData({ fontLevel: e.detail.value });
  },

  onLogout: function () {
    wx.showToast({ title: '已退出登录', icon: 'none' });
  }
});
