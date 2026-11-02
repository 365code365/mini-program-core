const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const PRICE = [[0, 1e9], [0, 300], [300, 600], [600, 1000], [1000, 1e9]];
const STAR = [null, 2, 3, 4, 5, 0];
Page({
  data: {
    sorts: ['推荐排序', '低价优先', '好评优先', '高价优先'], sort: 0,
    stars: ['不限', '经济型', '舒适型', '高档型', '豪华型', '特色民宿'],
    prices: ['不限', '¥300以下', '¥300-600', '¥600-1000', '¥1000以上'],
    star: 0, price: 0, keyword: '', list: [], hourly: false, city: '', people: '', kwSeq: 0
  },
  onShow() {
    // 每次回到列表都按最新的搜索条件（首页改了城市 / 日期 / 人数）
    const s = store.getSearch();
    this.s = s;
    const hourly = s.kind === 2;
    this.setData({
      star: s.star, price: s.price, keyword: s.keyword, hourly, city: s.city, people: store.peopleText(s),
      inShort: store.short(s.checkin).replace('月', '-').replace('日', ''), outShort: hourly ? '4小时' : store.short(s.checkout).replace('月', '-').replace('日', '')
    });
    wx.setNavigationBarTitle({ title: s.city + (hourly ? '钟点房' : (s.kind === 1 ? '民宿' : '酒店')) + (hourly ? '' : ' · ' + store.nights(s.checkin, s.checkout) + '晚') });
    this.filter();
  },
  filter() {
    const { star, price, keyword, sort, hourly } = this.data;
    const s = this.s;
    const favs = store.favs();
    const [lo, hi] = PRICE[price];
    // 每间要住的人数（成人 + 儿童按间平分，向上取整）
    const per = Math.ceil((s.adults + s.children) / s.rooms);
    let list = data.HOTELS.filter(h => {
      if (h.city !== s.city) return false;
      if (hourly && !h.hourly) return false;
      const rooms = h.rooms.filter(r => !!r.hours === hourly);
      const from = Math.min.apply(null, rooms.map(r => r.price));
      if (from < lo || from >= hi) return false;
      if (STAR[star] !== null && h.star !== STAR[star]) return false;
      if (!rooms.some(r => r.max >= per)) return false;
      if (keyword) {
        const hay = h.name + h.area + h.addr + h.tags.join('');
        if (hay.indexOf(keyword) < 0) return false;
      }
      return true;
    }).map(h => {
      const rooms = h.rooms.filter(r => !!r.hours === hourly);
      return Object.assign({}, h, { fav: favs.indexOf(h.id) >= 0, from: Math.min.apply(null, rooms.map(r => r.price)) });
    });
    if (sort === 1) list.sort((a, b) => a.from - b.from);
    if (sort === 2) list.sort((a, b) => b.score - a.score || b.reviews - a.reviews);
    if (sort === 3) list.sort((a, b) => b.from - a.from);
    this.setData({ list });
  },
  persist() {
    const s = store.getSearch();
    s.star = this.data.star; s.price = this.data.price; s.keyword = this.data.keyword;
    store.saveSearch(s);
    this.s = s;
  },
  onSort(e) { this.setData({ sort: Number(e.currentTarget.dataset.idx) }); this.filter(); },
  onStar(e) { const i = Number(e.currentTarget.dataset.idx); this.setData({ star: this.data.star === i ? 0 : i }); this.persist(); this.filter(); },
  onPrice(e) { const i = Number(e.currentTarget.dataset.idx); this.setData({ price: this.data.price === i ? 0 : i }); this.persist(); this.filter(); },
  onInput(e) { this.setData({ keyword: e.detail.value.trim() }); this.persist(); this.filter(); },
  onClear() { this.setData({ keyword: '', kwSeq: this.data.kwSeq + 1 }); this.persist(); this.filter(); },
  onReset() { this.setData({ keyword: '', star: 0, price: 0, kwSeq: this.data.kwSeq + 1 }); this.persist(); this.filter(); },
  // 改城市、日期、人数回搜索页
  onDates() { wx.navigateBack(); },
  onOpen(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); }
});
