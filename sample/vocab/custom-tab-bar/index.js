// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画；「复习」tab 带到期词数角标（各 tab 页 onShow 里由 utils/tab.js 写入）。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    badge: '',
    color: '#a0a6b0',
    selectedColor: '#4f6bed',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '学习', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/review/review', path: 'pages/review/review', text: '复习', icon: '/images/tab/review.png', iconOn: '/images/tab/review-on.png' },
      { pagePath: '/pages/notebook/notebook', path: 'pages/notebook/notebook', text: '生词本', icon: '/images/tab/book.png', iconOn: '/images/tab/book-on.png' },
      { pagePath: '/pages/mine/mine', path: 'pages/mine/mine', text: '我的', icon: '/images/tab/user.png', iconOn: '/images/tab/user-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const i = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[i].pagePath });
    }
  }
});
