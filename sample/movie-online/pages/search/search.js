// 搜索：输入防抖 300ms 再请求（每敲一个字就打一次接口，慢网下结果会乱序回来）；结果分页
const { request } = require('../../utils/api.js');
const HISTORY_KEY = 'movie_search_history';
const HOT = ['星际', '长安', '侦探', '深海', '夏日', '陈', '林', '动画'];

Page({
  data: { kw: '', hot: HOT, history: [], list: [], total: 0, status: 'idle', more: 'idle' },
  onLoad() {
    this.timer = 0;
    this.seq = 0;
    this.page = 0;
    this.setData({ history: wx.getStorageSync(HISTORY_KEY) || [] });
  },
  onUnload() { clearTimeout(this.timer); },
  onReachBottom() {
    if (this.data.more === 'idle' && this.data.kw) this.fetch(this.page + 1);
  },
  onInput(e) {
    const kw = e.detail.value.trim();
    this.setData({ kw });
    clearTimeout(this.timer);
    if (!kw) {
      this.seq++;
      this.setData({ status: 'idle', list: [] });
      return;
    }
    this.timer = setTimeout(() => this.fetch(1), 300);
  },
  onConfirm() {
    clearTimeout(this.timer);
    if (this.data.kw) { this.saveHistory(this.data.kw); this.fetch(1); }
  },
  onClear() {
    this.seq++;
    this.setData({ kw: '', list: [], status: 'idle' });
  },
  onPick(e) {
    const kw = e.currentTarget.dataset.kw;
    this.setData({ kw });
    this.saveHistory(kw);
    this.fetch(1);
  },
  onClearHistory() {
    wx.removeStorageSync(HISTORY_KEY);
    this.setData({ history: [] });
  },
  saveHistory(kw) {
    const h = [kw].concat(this.data.history.filter((x) => x !== kw)).slice(0, 10);
    wx.setStorageSync(HISTORY_KEY, h);
    this.setData({ history: h });
  },
  fetch(page) {
    const seq = ++this.seq;
    const kw = this.data.kw;
    this.setData(page === 1 ? { status: 'loading' } : { more: 'loading' });
    request('/api/demo/movies', { q: kw, page, size: 10 }, { silent: true }).then((r) => {
      if (seq !== this.seq) return;
      this.page = r.page;
      const items = r.list.map((m) => Object.assign({}, m, { scoreText: m.score ? m.score.toFixed(1) : '暂无评分' }));
      const list = page === 1 ? items : this.data.list.concat(items);
      this.setData({ list, total: r.total, status: list.length ? 'ok' : 'empty', more: r.hasMore ? 'idle' : 'end' });
    }).catch(() => {
      if (seq !== this.seq) return;
      if (page === 1) this.setData({ status: 'error' });
      else this.setData({ more: 'error' });
    });
  },
  onRetry() { this.fetch(1); },
  onRetryMore() { this.fetch(this.page + 1); },
  onMovie(e) {
    if (this.data.kw) this.saveHistory(this.data.kw);
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  }
});
