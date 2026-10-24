const { CITIES } = require('../../utils/data.js');
const T = require('../../utils/store.js');

const TAGS = ['全部', '古城', '海滨', '自然', '美食', '都市'];

Page({
  data: { tags: TAGS, tag: '全部', list: [], planned: {} },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    const planned = {};
    T.all().forEach(t => { planned[t.city] = true; });
    this.setData({ planned });
    this.filter();
  },
  filter() {
    const tag = this.data.tag;
    this.setData({ list: CITIES.filter(c => tag === '全部' || c.tag === tag) });
  },
  onTag(e) { this.setData({ tag: e.currentTarget.dataset.t }); this.filter(); },
  // 带参数进新建页：目的地直接填好
  plan(e) { wx.navigateTo({ url: '/pages/create/create?city=' + encodeURIComponent(e.currentTarget.dataset.name) }); }
});
