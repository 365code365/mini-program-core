// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画。
// 引擎命中自定义 tabBar 时直接读节点的 data-index + data-path（不带前导 /）切页，所以 wxml 两个都要写
const tab = (name, text, icon) => ({
  pagePath: '/pages/' + name + '/' + name,
  path: 'pages/' + name + '/' + name,
  text,
  icon: '/images/tab/' + icon + '.png',
  iconOn: '/images/tab/' + icon + '-on.png'
});

Component({
  data: {
    selected: 0,
    color: '#a3a9b3',
    selectedColor: '#3a7d6c',
    list: [
      tab('index', '首页', 'home'),
      tab('designers', '设计师', 'designer'),
      tab('budget', '算预算', 'budget'),
      tab('diary', '装修日记', 'diary'),
      tab('mine', '我的', 'user')
    ]
  },
  methods: {
    switchTab(e) {
      const i = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[i].pagePath });
    }
  }
});
