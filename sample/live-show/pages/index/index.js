const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');
const store = require('../../utils/store.js');

function brief(s, now) {
  const first = s.sessions[0], last = s.sessions[s.sessions.length - 1];
  const sale = D.onSale(s, now);
  const allOut = s.tiers.every((t) => t.soldOut);
  return {
    id: s.id, title: s.title, poster: s.poster, venue: s.city + ' · ' + s.venue, artist: s.artist,
    dates: first.date === last.date ? first.date + ' ' + first.time : first.date + ' 至 ' + last.date.slice(5),
    from: D.minPrice(s), want: F.wantText(s.want),
    badge: allOut ? '已售罄' : (sale ? '在售' : '即将开售'),
    badgeCls: allOut ? 'bd-out' : (sale ? 'bd-on' : 'bd-soon'),
    saleText: sale ? '' : F.monthDay(s.saleAt) + ' 开售'
  };
}

Page({
  data: {
    city: '上海', cats: D.CATS, cat: 'all', cur: 0,
    hot: [], list: [], days: [], dayIdx: 0, daySales: [], loading: true, updated: ''
  },
  onLoad() {
    this.setData({ city: getApp().globalData.city });
    // 首屏先给加载态，模拟请求
    setTimeout(() => this.reload(), 300);
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0, badge: store.orders().filter((o) => o.status === 'unpaid').length });
    if (!this.data.loading) this.buildDays();
  },
  onPullDownRefresh() {
    this.reload(() => {
      wx.stopPullDownRefresh();
      wx.showToast({ title: '已更新', icon: 'success' });
    });
  },
  refresh() { wx.startPullDownRefresh(); },
  reload(cb) {
    const now = Date.now();
    const city = this.data.city;
    const inCity = D.SHOWS.filter((s) => s.city === city);
    const hot = inCity.slice().sort((a, b) => b.want - a.want).slice(0, 5).map((s) => brief(s, now));
    const list = inCity.filter((s) => this.data.cat === 'all' || s.cat === this.data.cat).map((s) => brief(s, now));
    const d = new Date(now);
    this.setData({ hot, list, cur: 0, loading: false, updated: F.pad(d.getHours()) + ':' + F.pad(d.getMinutes()) });
    this.buildDays();
    cb && cb();
  },
  // 开售日历：今天起 7 天，每天列出当天开售的演出（全国）
  buildDays() {
    const now = Date.now();
    const base = new Date(now); base.setHours(0, 0, 0, 0);
    const reminds = store.reminds();
    const days = [];
    for (let i = 0; i < 7; i++) {
      const t0 = base.getTime() + i * D.DAY, t1 = t0 + D.DAY;
      const dd = new Date(t0);
      const sales = D.SHOWS.filter((s) => s.saleAt >= t0 && s.saleAt < t1).map((s) => ({
        id: s.id, title: s.title, city: s.city, time: F.pad(new Date(s.saleAt).getHours()) + ':' + F.pad(new Date(s.saleAt).getMinutes()),
        reminded: !!reminds[s.id], past: s.saleAt <= now
      }));
      days.push({ label: i === 0 ? '今天' : (i === 1 ? '明天' : F.WEEK[dd.getDay()]), md: (dd.getMonth() + 1) + '/' + dd.getDate(), n: sales.length, sales });
    }
    let idx = this.data.dayIdx;
    if (!this._dayPicked) { idx = days.findIndex((x) => x.n > 0); if (idx < 0) idx = 0; }
    this.setData({ days, dayIdx: idx, daySales: days[idx].sales });
  },
  pickDay(e) {
    const i = e.currentTarget.dataset.i;
    this._dayPicked = true;
    this.setData({ dayIdx: i, daySales: this.data.days[i].sales });
  },
  chooseCity() {
    wx.showActionSheet({
      itemList: D.CITIES,
      success: (r) => {
        const city = D.CITIES[r.tapIndex];
        getApp().globalData.city = city;
        this.setData({ city, cat: 'all', loading: true });
        wx.showLoading({ title: '切换中' });
        setTimeout(() => { wx.hideLoading(); this.reload(); }, 300);
      }
    });
  },
  pickCat(e) {
    this.setData({ cat: e.currentTarget.dataset.id });
    this.reload();
    wx.pageScrollTo({ selector: '#list-head', duration: 300 });
  },
  onSwiper(e) { this.setData({ cur: e.detail.current }); },
  open(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  toShanghai() {
    getApp().globalData.city = '上海';
    this.setData({ city: '上海', cat: 'all' });
    this.reload();
  },
  onShareAppMessage() { return { title: '现场：好演出不错过', path: '/pages/index/index' }; }
});
