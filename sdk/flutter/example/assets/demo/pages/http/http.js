// 信息流：请求 → 骨架屏 → 渲染 → 触底分页 → 点赞乐观更新 → 失败重试。数据来自 utils/mock-server.js
const http = require('../../utils/http.js');

const SIZE = 10;

function ago(min) {
  if (min < 60) return min + ' 分钟前';
  if (min < 60 * 24) return Math.floor(min / 60) + ' 小时前';
  return Math.floor(min / 60 / 24) + ' 天前';
}

Page({
  data: {
    list: [],
    page: 0,
    total: 0,
    // idle | loading | done | error（底部那一行的状态）
    footer: 'idle',
    firstLoading: true,
    error: '',
    latency: 600,
    failRate: 0,
    logs: []
  },
  onLoad() {
    const o = http.mock.getOptions();
    this.setData({ latency: o.latency, failRate: o.failRate });
    this.offLog = http.onLog((logs) => this.setData({ logs: logs.map((l) => Object.assign({}, l, { short: l.url.replace('mock://api', '') })) }));
    this.refresh();
  },
  onUnload() {
    this.offLog && this.offLog();
  },
  onReachBottom() {
    this.loadMore();
  },
  refresh() {
    this.setData({ list: [], page: 0, total: 0, footer: 'idle', firstLoading: true, error: '' });
    this.loadMore();
  },
  loadMore() {
    if (this.data.footer === 'loading' || this.data.footer === 'done') return;
    const next = this.data.page + 1;
    this.setData({ footer: 'loading', error: '' });
    http.request({ url: 'mock://api/feed?page=' + next + '&size=' + SIZE })
      .then((res) => {
        const items = res.list.map((it) => Object.assign(it, { time: ago(it.minutesAgo) }));
        // 只追加新的那一截：setData 用路径写法，已经渲染的条目不用重新传
        const patch = { page: res.page, total: res.total, footer: res.hasMore ? 'idle' : 'done', firstLoading: false };
        const base = this.data.list.length;
        items.forEach((it, i) => { patch['list[' + (base + i) + ']'] = it; });
        this.setData(patch);
      })
      .catch((e) => this.setData({ footer: 'error', error: e.message, firstLoading: false }));
  },
  retry() {
    this.setData({ footer: 'idle' });
    this.loadMore();
  },
  // 乐观更新：先改界面，请求失败再改回去
  like(e) {
    const idx = e.currentTarget.dataset.idx;
    const it = this.data.list[idx];
    const liked = !it.liked;
    const key = 'list[' + idx + ']';
    this.setData({ [key + '.liked']: liked, [key + '.likes']: it.likes + (liked ? 1 : -1) });
    http.request({ url: 'mock://api/like', method: 'POST', data: { id: it.id, liked } })
      .then((res) => this.setData({ [key + '.likes']: res.likes, [key + '.liked']: res.liked }))
      .catch((err) => {
        this.setData({ [key + '.liked']: !liked, [key + '.likes']: it.likes });
        wx.showToast({ title: '点赞失败，已撤回', icon: 'none' });
        console.log('like failed: ' + err.message);
      });
  },
  setLatency(e) {
    const latency = Number(e.currentTarget.dataset.v);
    http.mock.setOptions({ latency });
    this.setData({ latency });
  },
  setFail(e) {
    const failRate = Number(e.currentTarget.dataset.v);
    http.mock.setOptions({ failRate });
    this.setData({ failRate });
  },
  open(e) {
    wx.showModal({ title: '第 ' + e.currentTarget.dataset.id + ' 条', content: e.currentTarget.dataset.title, showCancel: false });
  }
});
