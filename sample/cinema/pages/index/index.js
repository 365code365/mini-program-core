const app = getApp();
function wan(n) { return n >= 10000 ? (n / 10000).toFixed(1) + '万' : '' + n; }
Page({
  data: {
    city: '上海', tab: 'now', keyword: '', list: [],
    banners: [
      { id: 'm3', poster: '/images/m3.jpg', title: '星际回响', sub: 'IMAX 同步上映 · 观众评分 9.0', tag: '本周票房冠军', bg: 'linear-gradient(120deg, #2B1055, #7A3A9A)' },
      { id: 'm1', poster: '/images/m1.jpg', title: '长河落日', sub: '国庆档口碑之作 · 累计 18.6 亿', tag: '口碑推荐', bg: 'linear-gradient(120deg, #1B2A6B, #B64E6E)' },
      { id: 'm5', poster: '/images/m5.jpg', title: '深海营救', sub: '11 月 8 日上映 · 预售开启', tag: '预售', bg: 'linear-gradient(120deg, #0D2B4A, #2F7DBF)' }
    ]
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    this.refresh();
  },
  refresh() {
    const wished = wx.getStorageSync('cinema_wish') || [];
    const kw = this.data.keyword;
    const list = app.MOVIES.filter(m => m.status === this.data.tab && (!kw || m.title.indexOf(kw) >= 0 || m.actors.some(a => a[0].indexOf(kw) >= 0)))
      .map(m => Object.assign({}, m, {
        imax: m.genre.indexOf('科幻') >= 0,
        scoreText: m.score.toFixed(1),
        actorText: m.actors.slice(0, 3).map(a => a[0]).join(' / '),
        wishText: wan(m.wish + (wished.indexOf(m.id) >= 0 ? 1 : 0)),
        wished: wished.indexOf(m.id) >= 0,
        releaseText: m.release.slice(5).replace('-', '月') + '日'
      }));
    this.setData({ list });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.tab }); this.refresh(); },
  onInput(e) { this.setData({ keyword: e.detail.value.trim() }); this.refresh(); },
  onCity() {
    const cities = ['上海', '北京', '广州', '深圳', '杭州', '成都'];
    wx.showActionSheet({ itemList: cities, success: (r) => this.setData({ city: cities[r.tapIndex] }) });
  },
  onMovie(e) { wx.navigateTo({ url: '/pages/movie/movie?id=' + e.currentTarget.dataset.id }); },
  onBuy(e) { wx.navigateTo({ url: '/pages/movie/movie?id=' + e.currentTarget.dataset.id + '&buy=1' }); },
  onWish(e) {
    const id = e.currentTarget.dataset.id;
    let w = wx.getStorageSync('cinema_wish') || [];
    const on = w.indexOf(id) < 0;
    w = on ? w.concat([id]) : w.filter(x => x !== id);
    wx.setStorageSync('cinema_wish', w);
    wx.showToast({ title: on ? '已标记想看，上映提醒你' : '已取消想看', icon: 'none' });
    this.refresh();
  }
});
