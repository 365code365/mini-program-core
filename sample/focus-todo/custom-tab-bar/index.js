Component({
  data: {
    selected: 0,
    color: '#a3a8b3',
    selectedColor: '#ff6250',
    list: [
      { pagePath: '/pages/todo/todo', text: '清单', iconPath: '/images/tab-todo.png', selectedIconPath: '/images/tab-todo-on.png' },
      { pagePath: '/pages/timer/timer', text: '专注', iconPath: '/images/tab-timer.png', selectedIconPath: '/images/tab-timer-on.png' },
      { pagePath: '/pages/stats/stats', text: '统计', iconPath: '/images/tab-stats.png', selectedIconPath: '/images/tab-stats-on.png' },
      { pagePath: '/pages/settings/settings', text: '设置', iconPath: '/images/tab-settings.png', selectedIconPath: '/images/tab-settings-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      wx.switchTab({ url: e.currentTarget.dataset.path })
      this.setData({ selected: Number(e.currentTarget.dataset.index) })
    }
  }
})
