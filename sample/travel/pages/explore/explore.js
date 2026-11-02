const { CITIES } = require('../../utils/data.js');
const T = require('../../utils/store.js');
const G = require('../../utils/guide.js');

const TAGS = ['全部', '想去', '古城', '海滨', '自然', '美食', '都市'];

Page({
  data: { tags: TAGS, tag: '全部', list: [], planned: {}, wish: {} },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    const planned = {};
    T.all().forEach(t => { planned[t.city] = true; });
    const wish = {};
    G.wishes().forEach(n => { wish[n] = true; });
    const g = getApp().globalData || {};
    if (g.exploreTag) { this.setData({ tag: g.exploreTag }); g.exploreTag = ''; }
    this.setData({ planned, wish });
    this.filter();
  },
  filter() {
    const tag = this.data.tag;
    const wish = this.data.wish;
    this.setData({ list: CITIES.filter(c => tag === '全部' || (tag === '想去' ? wish[c.name] : c.tag === tag)) });
  },
  onTag(e) { this.setData({ tag: e.currentTarget.dataset.t }); this.filter(); },
  // 点城市先看攻略，攻略页底部再按建议天数新建行程
  plan(e) { wx.navigateTo({ url: '/pages/guide/guide?city=' + encodeURIComponent(e.currentTarget.dataset.name) }); }
});
