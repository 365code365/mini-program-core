// 头条新闻 · 自定义 tabBar
Component({
  data: {
    selected: 0,
    list: [
      { pagePath: 'pages/home/home', text: '首页', iconType: 'success' },
      { pagePath: 'pages/video/video', text: '视频', iconType: 'waiting' },
      { pagePath: 'pages/mine/mine', text: '我的', iconType: 'info' }
    ]
  },
  methods: {
    switchTab: function (e) {
      var index = e.currentTarget.dataset.index;
      var path = e.currentTarget.dataset.path;
      this.setData({ selected: index });
      wx.switchTab({ url: '/' + path });
    }
  }
});
