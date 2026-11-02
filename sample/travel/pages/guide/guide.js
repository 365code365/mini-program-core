const { CITIES, spotsOf } = require('../../utils/data.js');
const T = require('../../utils/store.js');
const G = require('../../utils/guide.js');

// 城市攻略：灵感页点城市进来。看季节 / 天数 / 花费 / 必去景点，标「想去」，或直接按建议天数新建行程
Page({
  data: { city: null, g: {}, spots: [], trips: [], wish: false, budgetText: '' },
  onLoad(q) {
    const name = q.city ? decodeURIComponent(q.city) : '成都';
    const city = CITIES.find(c => c.name === name) || CITIES[0];
    const g = G.guideOf(city.name);
    wx.setNavigationBarTitle({ title: city.name + '攻略' });
    this.setData({ city, g, spots: spotsOf(city.name).slice(0, 6), budgetText: String(g.cost * g.days) });
  },
  onShow() {
    if (!this.data.city) return;
    const name = this.data.city.name;
    const trips = T.all().filter(t => t.city === name).map(t => ({
      id: t.id, title: t.title, range: T.short(t.start) + ' - ' + T.short(t.end), days: t.plan.length
    }));
    this.setData({ trips, wish: G.isWish(name) });
  },
  onWish() {
    const on = G.toggleWish(this.data.city.name);
    this.setData({ wish: on });
    wx.showToast({ title: on ? '已加入想去' : '已移出想去', icon: 'none' });
  },
  openTrip(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  // 新建页按「建议天数」预填返回日期
  plan() {
    wx.navigateTo({ url: '/pages/create/create?city=' + encodeURIComponent(this.data.city.name) + '&days=' + this.data.g.days });
  }
});
