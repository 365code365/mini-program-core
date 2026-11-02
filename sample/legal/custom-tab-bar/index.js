// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，用图片图标自己画。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以两个都要写。
Component({
  data: {
    selected: 0,
    color: '#9aa0a8',
    selectedColor: '#1f4fb4',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/lawyers/lawyers', path: 'pages/lawyers/lawyers', text: '找律师', icon: '/images/tab/lawyer.png', iconOn: '/images/tab/lawyer-on.png' },
      { pagePath: '/pages/contracts/contracts', path: 'pages/contracts/contracts', text: '合同', icon: '/images/tab/doc.png', iconOn: '/images/tab/doc-on.png' },
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
