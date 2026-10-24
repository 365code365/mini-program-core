// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath（只画文字首字），所以用图片图标自己画。
// 引擎命中自定义 tabBar 时不跑下面的 switchTab，而是读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    color: '#9aa0a8',
    selectedColor: '#1f7cf0',
    list: [
      {
            "pagePath": "/pages/index/index",
            "path": "pages/index/index",
            "text": "首页",
            "icon": "/images/tab/home.png",
            "iconOn": "/images/tab/home-on.png"
      },
      {
            "pagePath": "/pages/dept/dept",
            "path": "pages/dept/dept",
            "text": "科室",
            "icon": "/images/tab/grid.png",
            "iconOn": "/images/tab/grid-on.png"
      },
      {
            "pagePath": "/pages/orders/orders",
            "path": "pages/orders/orders",
            "text": "挂号单",
            "icon": "/images/tab/doc.png",
            "iconOn": "/images/tab/doc-on.png"
      },
      {
            "pagePath": "/pages/mine/mine",
            "path": "pages/mine/mine",
            "text": "我的",
            "icon": "/images/tab/user.png",
            "iconOn": "/images/tab/user-on.png"
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
