// 仪表盘：四个接口并发请求（Promise.all），各块独立的加载 / 失败态；再加一个真实网络请求做对照
const http = require('../../utils/http.js');

// 统计格子窄：上万按「万」显示，不然一个金额就把四格挤成两行
function money(n) {
  return n >= 10000 ? '¥' + (n / 10000).toFixed(2) + '万' : '¥' + n.toFixed(2);
}

Page({
  data: {
    loading: true,
    costMs: 0,
    profile: null,
    stats: null,
    trend: [],
    trendMax: 1,
    orders: [],
    errors: {},
    real: '点按钮从 httpbin.org 取一份真实 JSON（要联网）'
  },
  onLoad() {
    this.loadAll();
  },
  loadAll() {
    const started = Date.now();
    this.setData({ loading: true, errors: {} });
    // 每一块单独 catch：一个接口挂了不影响别的块渲染
    const part = (key, url, apply) =>
      http.request({ url }).then(apply).catch((e) => this.setData({ ['errors.' + key]: e.message }));
    Promise.all([
      part('profile', 'mock://api/profile', (d) => this.setData({ profile: d })),
      part('stats', 'mock://api/stats', (d) => this.setData({ stats: Object.assign(d, { amountText: money(d.amount) }) })),
      part('trend', 'mock://api/trend?days=7', (d) => {
        const max = Math.max.apply(null, d.points.map((p) => p.value));
        this.setData({ trend: d.points, trendMax: max });
      }),
      part('orders', 'mock://api/orders?limit=5', (d) => this.setData({ orders: d.list }))
    ]).then(() => this.setData({ loading: false, costMs: Date.now() - started }));
  },
  realRequest() {
    this.setData({ real: '请求中…' });
    const started = Date.now();
    http.request({ url: 'https://httpbin.org/json' })
      .then((d) => {
        const s = d && d.slideshow;
        const titles = s && s.slides ? s.slides.map((x) => x.title).join(' / ') : JSON.stringify(d).slice(0, 80);
        this.setData({ real: (Date.now() - started) + 'ms · ' + (s ? s.title + '：' : '') + titles });
      })
      .catch((e) => this.setData({ real: '失败：' + e.message }));
  }
});
