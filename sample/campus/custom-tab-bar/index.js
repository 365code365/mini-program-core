// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，所以用图片图标自己画。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，wxml 两个都要写。
// 角标 / 红点：页面里 getTabBar().setData({ 'list[i].badge': n }) / { 'list[i].dot': true }（等价于 wx.setTabBarBadge / showTabBarRedDot）
Component({
  data: {
    selected: 0,
    color: '#9aa0ab',
    selectedColor: '#14937f',
    list: [
      {
        "pagePath": "/pages/index/index",
        "path": "pages/index/index",
        "text": "首页",
        "icon": "/images/tab/home.png",
        "iconOn": "/images/tab/home-on.png",
        "badge": 0,
        "dot": false
      },
      {
        "pagePath": "/pages/timetable/timetable",
        "path": "pages/timetable/timetable",
        "text": "课表",
        "icon": "/images/tab/calendar.png",
        "iconOn": "/images/tab/calendar-on.png",
        "badge": 0,
        "dot": false
      },
      {
        "pagePath": "/pages/notices/notices",
        "path": "pages/notices/notices",
        "text": "通知",
        "icon": "/images/tab/bell.png",
        "iconOn": "/images/tab/bell-on.png",
        "badge": 0,
        "dot": false
      },
      {
        "pagePath": "/pages/mine/mine",
        "path": "pages/mine/mine",
        "text": "我的",
        "icon": "/images/tab/user.png",
        "iconOn": "/images/tab/user-on.png",
        "badge": 0,
        "dot": false
      }
    ]
  },
  methods: {
    switchTab(e) {
      const i = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[i].pagePath });
    }
  }
});
