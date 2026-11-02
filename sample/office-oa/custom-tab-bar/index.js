// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，用图片图标自己画。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以两个都要写。
// 「审批」上的红点 = 待我审批的单数，由各 tab 页 onShow 时 setData({ badge })。
Component({
  data: {
    selected: 0,
    badge: 0,
    color: '#9aa0a8',
    selectedColor: '#1677ff',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '工作台', icon: '/images/tab/work.png', iconOn: '/images/tab/work-on.png' },
      { pagePath: '/pages/approvals/approvals', path: 'pages/approvals/approvals', text: '审批', icon: '/images/tab/approve.png', iconOn: '/images/tab/approve-on.png' },
      { pagePath: '/pages/contacts/contacts', path: 'pages/contacts/contacts', text: '通讯录', icon: '/images/tab/contacts.png', iconOn: '/images/tab/contacts-on.png' },
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
