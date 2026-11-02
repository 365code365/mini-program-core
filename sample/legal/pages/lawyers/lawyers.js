const D = require('../../utils/data.js');
const S = require('../../utils/store.js');
const app = getApp();

const CITIES = [{ id: '', name: '全部城市' }].concat(['北京', '上海', '杭州', '深圳', '成都', '南京'].map(c => ({ id: c, name: c })));
const SORTS = ['综合', '评分最高', '价格最低', '执业年限'];

Page({
  data: {
    cats: [{ id: '', name: '全部' }].concat(D.categories),
    cat: '',
    cities: CITIES,
    cityIdx: 0,
    sorts: SORTS,
    sort: 0,
    favOnly: false,
    loading: true,
    list: []
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1 });
    // 首页分类入口经 globalData 带过来（switchTab 不能带 query）
    if (app.globalData.lawyerCat !== undefined) {
      this.setData({ cat: app.globalData.lawyerCat });
      app.globalData.lawyerCat = undefined;
    }
    if (app.globalData.lawyerFav) {
      this.setData({ favOnly: true, cityIdx: 0 });
      app.globalData.lawyerFav = false;
    }
    this.load();
  },
  load() {
    this.setData({ loading: true });
    setTimeout(() => this.apply(), 300);
  },
  apply() {
    const { cat, cities, cityIdx, sort, favOnly } = this.data;
    const city = cities[cityIdx].id;
    const favs = S.favs();
    let list = D.lawyers.filter(l => (!cat || l.fields.indexOf(cat) >= 0) && (!city || l.city === city) && (!favOnly || favs.indexOf(l.id) >= 0));
    const by = [
      (a, b) => b.rating * 1000 + b.consults / 10 - (a.rating * 1000 + a.consults / 10),
      (a, b) => b.rating - a.rating,
      (a, b) => a.price.text - b.price.text,
      (a, b) => b.years - a.years
    ][sort];
    list = list.slice().sort(by).map(l => Object.assign(D.decorate(l), { fav: favs.indexOf(l.id) >= 0 }));
    this.setData({ list, loading: false });
  },
  pickCat(e) { this.setData({ cat: e.currentTarget.dataset.id }); this.load(); },
  onCity(e) { this.setData({ cityIdx: Number(e.detail.value) }); this.load(); },
  pickSort(e) { this.setData({ sort: e.currentTarget.dataset.i }); this.apply(); },
  toggleFavOnly() { this.setData({ favOnly: !this.data.favOnly }); this.apply(); },
  reset() { this.setData({ cat: '', cityIdx: 0, sort: 0, favOnly: false }); this.load(); },
  goLawyer(e) { wx.navigateTo({ url: '/pages/lawyer/lawyer?id=' + e.currentTarget.dataset.id }); },
  ask(e) { wx.navigateTo({ url: '/pages/order/order?lawyer=' + e.currentTarget.dataset.id }); }
});
