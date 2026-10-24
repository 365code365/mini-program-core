const data = require('../../utils/data.js');

const INTENTS = ['前端开发工程师', 'Java 后端', '产品经理', 'UI/UX 设计师', '数据分析师', '电商运营'];

Page({
  data: {
    intent: '前端开发工程师',
    cities: ['杭州', '上海', '北京', '深圳', '全国'],
    cityIdx: 0,
    keyword: '',
    tabs: ['推荐', '最新', '附近'],
    tabIdx: 0,
    list: []
  },
  onLoad() {
    const saved = wx.getStorageSync('jobs_intent');
    if (saved) this.setData({ intent: saved.intent, cityIdx: saved.cityIdx });
    this.refresh();
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
  },
  refresh() {
    const city = this.data.cities[this.data.cityIdx];
    const kw = this.data.keyword.trim().toLowerCase();
    let list = data.jobs.map(data.decorate);
    if (city !== '全国') list = list.filter(j => j.city === city);
    if (kw) {
      list = list.filter(j => (j.title + j.co.name + j.tags.join(' ')).toLowerCase().indexOf(kw) >= 0);
    }
    const t = this.data.tabIdx;
    if (t === 0) list.sort((a, b) => b.hot - a.hot);
    else if (t === 1) list.sort((a, b) => a.posted - b.posted);
    else list.sort((a, b) => a.dist - b.dist);
    this.setData({ list });
  },
  saveIntent() { wx.setStorageSync('jobs_intent', { intent: this.data.intent, cityIdx: this.data.cityIdx }); },
  onIntent() {
    wx.showActionSheet({
      itemList: INTENTS,
      success: (r) => { this.setData({ intent: INTENTS[r.tapIndex] }); this.saveIntent(); }
    });
  },
  onCity(e) { this.setData({ cityIdx: Number(e.detail.value) }); this.saveIntent(); this.refresh(); },
  onSearch(e) { this.setData({ keyword: e.detail.value }); this.refresh(); },
  onClear() { this.setData({ keyword: '' }); this.refresh(); },
  onTab(e) { this.setData({ tabIdx: e.currentTarget.dataset.idx }); this.refresh(); },
  onJob(e) { wx.navigateTo({ url: '/pages/job/job?id=' + e.currentTarget.dataset.id }); }
});
