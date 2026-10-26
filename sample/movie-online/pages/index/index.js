// 首页：轮播 + 三个榜单 tab，上拉分页、下拉刷新。数据全部来自 /api/demo/movies
const { request } = require('../../utils/api.js');
const app = getApp();
const SIZE = 10;
const TABS = [{ key: 'now', name: '正在热映' }, { key: 'coming', name: '即将上映' }, { key: 'top', name: '高分榜' }];

function decorate(m, i, tab) {
  const d = m.releaseDate.split('-');
  return Object.assign({}, m, {
    scoreText: m.score ? m.score.toFixed(1) : '',
    starWidth: m.score * 10,
    releaseText: Number(d[1]) + '月' + Number(d[2]) + '日',
    year: d[0],
    rank: tab === 'top' ? i + 1 : 0,
    wished: app.isWished(m.id)
  });
}

Page({
  data: {
    tabs: TABS, tab: 'now', banners: [], list: [],
    // status：loading 首屏加载 / error 首屏失败 / empty / ok；more：idle / loading / error / end
    status: 'loading', more: 'idle', total: 0
  },
  onLoad() {
    this.page = 0;
    this.seq = 0;
    this.loadBanners();
    this.reload();
  },
  onShow() {
    // 从详情页回来，想看状态可能变了：只刷本地字段，不重新请求
    if (this.data.list.length) {
      this.setData({ list: this.data.list.map((m) => Object.assign({}, m, { wished: app.isWished(m.id) })) });
    }
  },
  onPullDownRefresh() {
    const done = () => wx.stopPullDownRefresh();
    Promise.all([this.loadBanners(), this.reload(true)]).then(done, done);
  },
  onReachBottom() {
    if (this.data.more === 'idle') this.fetch(this.page + 1);
  },
  loadBanners() {
    return request('/api/demo/banners', { app: 'movie' }, { silent: true })
      .then((r) => this.setData({ banners: r.list }))
      .catch(() => {});
  },
  reload(keepList) {
    this.page = 0;
    if (!keepList) this.setData({ status: 'loading', list: [], more: 'idle' });
    return this.fetch(1);
  },
  // seq：切 tab 时上一个 tab 的请求可能还没回来，回来了也要丢掉，否则列表会串台
  fetch(page) {
    const seq = ++this.seq;
    const tab = this.data.tab;
    if (page > 1) this.setData({ more: 'loading' });
    return request('/api/demo/movies', { type: tab, page, size: SIZE }, { silent: page === 1 })
      .then((r) => {
        if (seq !== this.seq) return;
        this.page = r.page;
        const base = page === 1 ? [] : this.data.list;
        const list = base.concat(r.list.map((m, i) => decorate(m, base.length + i, tab)));
        this.setData({ list, total: r.total, status: list.length ? 'ok' : 'empty', more: r.hasMore ? 'idle' : 'end' });
      })
      .catch(() => {
        if (seq !== this.seq) return;
        if (page === 1 && !this.data.list.length) this.setData({ status: 'error' });
        else this.setData({ more: 'error' });
      });
  },
  onRetry() { this.reload(); },
  onRetryMore() { this.fetch(this.page + 1); },
  onTab(e) {
    const tab = e.currentTarget.dataset.key;
    if (tab === this.data.tab) return;
    this.setData({ tab });
    this.reload();
    wx.pageScrollTo({ scrollTop: 0, duration: 0 });
  },
  onSearch() { wx.navigateTo({ url: '/pages/search/search' }); },
  onBanner(e) {
    const d = e.currentTarget.dataset;
    if (d.type === 'movie') wx.navigateTo({ url: '/pages/detail/detail?id=' + d.id });
  },
  onMovie(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onWish(e) {
    const idx = e.currentTarget.dataset.index;
    const m = this.data.list[idx];
    const on = app.toggleWish(m);
    this.setData({ ['list[' + idx + '].wished']: on });
    wx.showToast({ title: on ? '已加入想看' : '已取消想看', icon: 'none' });
  }
});
