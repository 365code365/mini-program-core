const app = getApp();
const u = app.util;
const PRICE = [[0, 1e9], [0, 300], [300, 600], [600, 1000], [1000, 1e9]];
const STAR = [null, 2, 3, 4, 5, 0];

Page({
  data: {
    sorts: ['推荐排序', '低价优先', '好评优先', '高价优先'], sort: 0,
    stars: ['不限', '经济型', '舒适型', '高档型', '豪华型', '特色民宿'],
    prices: ['不限', '¥300以下', '¥300-600', '¥600-1000', '¥1000以上'],
    star: 0, price: 0, keyword: '', list: []
  },
  onLoad() {
    const s = app.getSearch();
    this.setData({ star: s.star, price: s.price, keyword: s.keyword, inShort: u.short(s.checkin).replace('月', '-').replace('日', ''), outShort: u.short(s.checkout).replace('月', '-').replace('日', '') });
    wx.setNavigationBarTitle({ title: s.city + '酒店 · ' + u.nights(s.checkin, s.checkout) + '晚' });
  },
  onShow() { this.filter(); },
  filter() {
    const { star, price, keyword, sort } = this.data;
    const favs = wx.getStorageSync('hotel_favs') || [];
    const [lo, hi] = PRICE[price];
    let list = app.HOTELS.filter(h => {
      if (h.price < lo || h.price >= hi) return false;
      if (STAR[star] !== null && h.star !== STAR[star]) return false;
      if (keyword) {
        const hay = h.name + h.area + h.addr + h.tags.join('');
        if (hay.indexOf(keyword) < 0) return false;
      }
      return true;
    }).map(h => Object.assign({}, h, { fav: favs.indexOf(h.id) >= 0 }));
    if (sort === 1) list.sort((a, b) => a.price - b.price);
    if (sort === 2) list.sort((a, b) => b.score - a.score || b.reviews - a.reviews);
    if (sort === 3) list.sort((a, b) => b.price - a.price);
    this.setData({ list });
  },
  persist() {
    const s = app.getSearch();
    s.star = this.data.star; s.price = this.data.price; s.keyword = this.data.keyword;
    app.saveSearch(s);
  },
  onSort(e) { this.setData({ sort: e.currentTarget.dataset.idx }); this.filter(); },
  onStar(e) { const i = e.currentTarget.dataset.idx; this.setData({ star: this.data.star === i ? 0 : i }); this.persist(); this.filter(); },
  onPrice(e) { const i = e.currentTarget.dataset.idx; this.setData({ price: this.data.price === i ? 0 : i }); this.persist(); this.filter(); },
  onInput(e) { this.setData({ keyword: e.detail.value.trim() }); this.persist(); this.filter(); },
  onClear() { this.setData({ keyword: '' }); this.persist(); this.filter(); },
  onReset() { this.setData({ keyword: '', star: 0, price: 0 }); this.persist(); this.filter(); },
  onOpen(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); }
});
