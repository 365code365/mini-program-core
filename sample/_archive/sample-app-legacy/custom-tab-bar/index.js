// 自定义 TabBar 组件
Component({
  data: {
    selected: 0,
    list: [
      { pagePath: 'pages/index/index', text: '首页', iconType: 'success' },
      { pagePath: 'pages/category/category', text: '分类', iconType: 'info' },
      { pagePath: 'pages/cart/cart', text: '购物车', iconType: 'waiting' },
      { pagePath: 'pages/profile/profile', text: '我的', iconType: 'warn' }
    ]
  },
  methods: {
    switchTab: function(e) {
      var index = e.currentTarget.dataset.index;
      var path = e.currentTarget.dataset.path;
      this.setData({ selected: index });
      wx.switchTab({ url: '/' + path });
    }
  }
});
