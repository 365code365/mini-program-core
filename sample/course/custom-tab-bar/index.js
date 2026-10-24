// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，用图片图标自己画
Component({
  data: {
    selected: 0,
    color: '#9aa0ab',
    selectedColor: '#3d6cf6',
    list: [
      { pagePath: '/pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/learn/learn', text: '学习', icon: '/images/tab/book.png', iconOn: '/images/tab/book-on.png' },
      { pagePath: '/pages/checkin/checkin', text: '打卡', icon: '/images/tab/cal.png', iconOn: '/images/tab/cal-on.png' },
      { pagePath: '/pages/mine/mine', text: '我的', icon: '/images/tab/user.png', iconOn: '/images/tab/user-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const i = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[i].pagePath });
    }
  }
});
