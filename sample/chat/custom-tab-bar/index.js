Component({
  data: {
    selected: 0,
    badge: 0,
    badgeText: '',
    color: '#9aa0a6',
    selectedColor: '#4F6BFF',
    list: [
      { pagePath: '/pages/sessions/sessions', path: 'pages/sessions/sessions', text: '消息', iconPath: '/images/tab-chats.png', selectedIconPath: '/images/tab-chats-on.png' },
      { pagePath: '/pages/contacts/contacts', path: 'pages/contacts/contacts', text: '通讯录', iconPath: '/images/tab-contacts.png', selectedIconPath: '/images/tab-contacts-on.png' },
      { pagePath: '/pages/me/me', path: 'pages/me/me', text: '我', iconPath: '/images/tab-me.png', selectedIconPath: '/images/tab-me-on.png' }
    ]
  },
  // 角标：页面 onShow 里 store.syncTabBadge 经 getTabBar().setData 推过来（官方写法）；
  // 自己 attached / 所在页 show 时也从 storage 算一遍兜底。
  // 已知引擎问题：本引擎里 tabBar 首次渲染之后，这两条路都改不动角标（见交付说明）
  lifetimes: { attached() { this.refreshBadge(); } },
  pageLifetimes: { show() { this.refreshBadge(); } },
  methods: {
    refreshBadge() {
      const list = wx.getStorageSync('chat_sessions') || [];
      const n = list.reduce((a, s) => a + (s.unread || 0), 0);
      if (n !== this.data.badge) this.setData({ badge: n, badgeText: n > 99 ? '99+' : '' + n });
    },
    switchTab(e) {
      const idx = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[idx].pagePath });
    }
  }
});
