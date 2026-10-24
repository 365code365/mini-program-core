// 自定义 tabBar：原生 tabBar 在本引擎里只画文字首字，不加载 iconPath，所以用组件自己画图标
Component({
  data: {
    selected: 0,
    color: '#9aa0a6',
    selectedColor: '#1E6FFF',
    list: [
      {
        "pagePath": "/pages/index/index",
        "text": "首页",
        "iconPath": "/images/tab-home.png",
        "selectedIconPath": "/images/tab-home-on.png"
      },
      {
        "pagePath": "/pages/orders/orders",
        "text": "订单",
        "iconPath": "/images/tab-orders.png",
        "selectedIconPath": "/images/tab-orders-on.png"
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
