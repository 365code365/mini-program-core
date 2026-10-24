// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath（只画文字首字），所以用图片图标自己画。
// 引擎命中自定义 tabBar 时不跑下面的 switchTab，而是读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    color: '#9aa0a8',
    selectedColor: '#ff8a3d',
    list: [
      {
            "pagePath": "/pages/index/index",
            "path": "pages/index/index",
            "text": "首页",
            "icon": "/images/tab/home.png",
            "iconOn": "/images/tab/home-on.png"
      },
      {
            "pagePath": "/pages/pet/pet",
            "path": "pages/pet/pet",
            "text": "宠物档案",
            "icon": "/images/tab/paw.png",
            "iconOn": "/images/tab/paw-on.png"
      },
      {
            "pagePath": "/pages/mall/mall",
            "path": "pages/mall/mall",
            "text": "商城",
            "icon": "/images/tab/bag.png",
            "iconOn": "/images/tab/bag-on.png"
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
