// 自定义 tabBar：原生 tabBar 在本引擎里只画文字首字，不加载 iconPath，所以用组件自己画图标
Component({
  data: {
    selected: 0,
    color: '#9aa0a6',
    selectedColor: '#F0384D',
    list: [
      {
        "pagePath": "/pages/index/index",
        "text": "电影",
        "iconPath": "/images/tab-home.png",
        "selectedIconPath": "/images/tab-home-on.png"
      },
      {
        "pagePath": "/pages/tickets/tickets",
        "text": "我的票",
        "iconPath": "/images/tab-tickets.png",
        "selectedIconPath": "/images/tab-tickets-on.png"
      },
      {
        "pagePath": "/pages/mine/mine",
        "text": "我的",
        "iconPath": "/images/tab-mine.png",
        "selectedIconPath": "/images/tab-mine-on.png"
      }
    ]
  },
  methods: {
    switchTab(e) {
      const idx = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[idx].pagePath });
    }
  }
});
