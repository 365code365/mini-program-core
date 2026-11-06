// 全部短评：最新 / 最热两种排序，onReachBottom 分页；自己写的那条（本地）排在第一页最前面
const { request } = require('../../utils/api.js');
const store = require('../../utils/store.js');
const SIZE = 10;

Page({
  data: { sort: 'hot', list: [], total: 0, status: 'loading', more: 'idle', title: '' },
  onLoad(q) {
    this.id = Number(q.id) || 1;
    this.seq = 0;
    this.page = 0;
    const title = decodeURIComponent(q.title || '');
    this.setData({ title });
    if (title) wx.setNavigationBarTitle({ title: '《' + title + '》短评' });
    this.fetch(1);
  },
  onReachBottom() {
    if (this.data.more === 'idle') this.fetch(this.page + 1);
  },
  onPullDownRefresh() {
    this.fetch(1).then(() => wx.stopPullDownRefresh());
  },
  onSort(e) {
    const sort = e.currentTarget.dataset.sort;
    if (sort === this.data.sort) return;
    this.setData({ sort, list: [], status: 'loading' });
    this.fetch(1);
  },
  fetch(page) {
    const seq = ++this.seq;
    if (page > 1) this.setData({ more: 'loading' });
    return request('/api/demo/movies/' + this.id + '/comments', { sort: this.data.sort === 'new' ? 'new' : 'hot', page, size: SIZE }, { silent: true })
      .then((r) => {
        if (seq !== this.seq) return;
        this.page = r.page;
        const items = r.list.map((c) => Object.assign({}, c, { starWidth: c.star * 20, liked: false }));
        const list = page === 1 ? this.withMine(items) : this.data.list.concat(items);
        this.apiTotal = r.total;
        this.setData({ list, total: r.total + (store.myCommentOf(this.id) ? 1 : 0), status: list.length ? 'ok' : 'empty', more: r.hasMore ? 'idle' : 'end' });
      })
      .catch(() => {
        if (seq !== this.seq) return;
        this.setData(page === 1 ? { status: 'error' } : { more: 'error' });
      });
  },
  withMine(items) {
    return store.mergeComments(this.id, items.filter((c) => !c.mine)).map((c) => Object.assign({ starWidth: c.star * 20, liked: false }, c));
  },
  // 从写短评回来：换上新的那条（或删掉）
  onShow() {
    if (this.apiTotal === undefined) return;
    const list = this.withMine(this.data.list);
    this.setData({ list, total: this.apiTotal + (store.myCommentOf(this.id) ? 1 : 0), status: list.length ? 'ok' : 'empty' });
  },
  onWrite() { wx.navigateTo({ url: '/pages/write/write?id=' + this.id }); },
  onRetry() { this.setData({ status: 'loading' }); this.fetch(1); },
  onRetryMore() { this.fetch(this.page + 1); },
  // 点赞只在本页生效（演示接口只读）
  onLike(e) {
    const i = Number(e.currentTarget.dataset.index);
    const c = this.data.list[i];
    this.setData({ ['list[' + i + '].liked']: !c.liked, ['list[' + i + '].likes']: c.likes + (c.liked ? -1 : 1) });
  }
});
