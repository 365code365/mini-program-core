// 发现页：标签横滑 + 双列瀑布流（按图片宽高比分到较矮的那一列）+ 上拉分页 + 下拉刷新
const { request } = require('../../utils/api.js');
const app = getApp();
const SIZE = 12;
const COL_W = 345;      // 一列的宽（rpx）
const INFO_H = 104;     // 卡片标题 + 作者那一栏的高（rpx），参与「哪一列更矮」的计算

Page({
  data: { tags: [], tag: '', left: [], right: [], status: 'loading', more: 'idle', total: 0, banners: [] },
  onLoad() {
    this.seq = 0;
    this.page = 0;
    this.items = [];
    request('/api/demo/tags', { app: 'gallery' }, { silent: true })
      .then((r) => this.setData({ tags: r.list }))
      .catch(() => {});
    request('/api/demo/banners', { app: 'gallery' }, { silent: true })
      .then((r) => this.setData({ banners: r.list }))
      .catch(() => {});
    this.reload();
  },
  onShow() {
    const t = app.globalData.pendingTag;
    if (t !== null && t !== undefined) {
      app.globalData.pendingTag = null;
      if (t !== this.data.tag) { this.setData({ tag: t }); this.reload(); return; }
    }
    // 详情页里点过赞：只刷本地的点赞态
    if (this.items.length) this.layout(this.items);
  },
  onPullDownRefresh() {
    this.reload().then(() => wx.stopPullDownRefresh());
  },
  onReachBottom() {
    if (this.data.more === 'idle') this.fetch(this.page + 1);
  },
  reload() {
    this.page = 0;
    this.setData({ status: this.items.length ? this.data.status : 'loading' });
    return this.fetch(1);
  },
  fetch(page) {
    const seq = ++this.seq;
    if (page > 1) this.setData({ more: 'loading' });
    return request('/api/demo/photos', { tag: this.data.tag, page, size: SIZE }, { silent: true }).then((r) => {
      if (seq !== this.seq) return;
      this.page = r.page;
      this.items = page === 1 ? r.list : this.items.concat(r.list);
      this.layout(this.items);
      app.globalData.previewUrls = this.items.map((p) => p.url);
      this.setData({ total: r.total, status: this.items.length ? 'ok' : 'empty', more: r.hasMore ? 'idle' : 'end' });
    }).catch(() => {
      if (seq !== this.seq) return;
      if (page === 1 && !this.items.length) this.setData({ status: 'error' });
      else this.setData({ more: 'error' });
    });
  },
  // 每次都从头按顺序分列：同一份列表永远同样的排布（翻页追加时前面的卡片不会跳列）
  layout(items) {
    const left = [];
    const right = [];
    let hl = 0;
    let hr = 0;
    items.forEach((p, i) => {
      const h = Math.round(COL_W * p.ratio);
      const card = { id: p.id, title: p.title, url: p.url, h, color: p.color, author: p.author, likes: p.likes, liked: app.isLiked(p.id), index: i };
      if (app.isLiked(p.id)) card.likes += 1;
      if (hl <= hr) { left.push(card); hl += h + INFO_H; } else { right.push(card); hr += h + INFO_H; }
    });
    this.setData({ left, right });
  },
  onTag(e) {
    const tag = e.currentTarget.dataset.name;
    if (tag === this.data.tag) return;
    this.items = [];
    this.setData({ tag, left: [], right: [], status: 'loading', more: 'idle' });
    wx.pageScrollTo({ scrollTop: 0, duration: 0 });
    this.fetch(1);
  },
  onBanner(e) {
    const tag = e.currentTarget.dataset.tag;
    if (tag) this.onTag({ currentTarget: { dataset: { name: tag } } });
  },
  onPhoto(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  // 长按：直接全屏预览，urls 是当前整个列表（可左右翻、双指缩放）
  onPreview(e) {
    const urls = this.items.map((p) => p.url);
    wx.previewImage({ urls, current: urls[e.currentTarget.dataset.index] });
  },
  onLike(e) {
    const id = e.currentTarget.dataset.id;
    const on = app.toggleLike(id);
    this.layout(this.items);
    wx.showToast({ title: on ? '已点赞' : '已取消点赞', icon: 'none' });
  },
  onRetry() { this.setData({ status: 'loading' }); this.fetch(1); },
  onRetryMore() { this.fetch(this.page + 1); }
});
