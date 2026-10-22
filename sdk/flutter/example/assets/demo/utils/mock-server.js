// 模拟后端：`mock://api/...` 的请求在这里按路由造数据，带可调的网络延迟与失败率。
// 演示「请求 → 加载态 → 渲染 / 失败重试」整条链路，不依赖外网，每次打开结果一样（按页码 / id 取数）。
// 换成真接口时，页面代码不用改：utils/http.js 只在 url 以 mock:// 开头时走这里，其余照常 wx.request。

const options = { latency: 600, failRate: 0 };

const TITLES = [
  '用 Rust 写一个小程序渲染引擎是什么体验', '增量重绘：只画变了的那几行', '为什么滑动卡顿要看最慢的一帧',
  '布局缓存与 setData 的失效分析', 'GPU 合成与 CPU 光栅化各管什么', '大小核调度对渲染线程的影响',
  '一次 setData 到上屏经过了哪些步骤', '从 WXSS 到像素：样式是怎么算出来的', '离屏缓冲与 transform 的代价',
  '小程序生命周期的那些顺序问题', '字体：系统字体与内置字体的取舍', '真机上量帧率的正确姿势'
];
const AUTHORS = ['林小满', '周一', '陈默', '王二', '赵晴', '孙悟', '李雷', '韩梅梅'];
const COLORS = ['#07c160', '#10aeff', '#fa9d3b', '#e64340', '#8a5cf5', '#576b95', '#ff6a00', '#1aad19'];
const SUMMARY = '这是一段由模拟服务返回的摘要文字，用来演示多行文本的排版、省略与列表渲染。真实项目里它来自后端接口，字段结构保持一致即可。';
const TOTAL_FEED = 46;
const liked = {};

function feedItem(id) {
  const n = id - 1;
  return {
    id,
    title: TITLES[n % TITLES.length] + (n >= TITLES.length ? '（续 ' + Math.floor(n / TITLES.length) + '）' : ''),
    author: AUTHORS[n % AUTHORS.length],
    initial: AUTHORS[n % AUTHORS.length].slice(0, 1),
    color: COLORS[n % COLORS.length],
    summary: SUMMARY.length > 30 + (n * 7) % 40 ? SUMMARY.slice(0, 30 + (n * 7) % 40) + '…' : SUMMARY,
    minutesAgo: 3 + n * 17,
    likes: 10 + (n * 37) % 300 + (liked[id] ? 1 : 0),
    liked: !!liked[id]
  };
}

function query(url) {
  const q = {};
  const i = url.indexOf('?');
  if (i < 0) return q;
  url.slice(i + 1).split('&').forEach((kv) => {
    const [k, v] = kv.split('=');
    if (k) q[decodeURIComponent(k)] = decodeURIComponent(v || '');
  });
  return q;
}

// 路由表：返回 [statusCode, body]
const routes = {
  'GET /feed'(q) {
    const page = Math.max(1, parseInt(q.page || '1', 10));
    const size = Math.min(20, Math.max(1, parseInt(q.size || '10', 10)));
    const start = (page - 1) * size + 1;
    const list = [];
    for (let id = start; id < start + size && id <= TOTAL_FEED; id++) list.push(feedItem(id));
    return [200, { page, size, total: TOTAL_FEED, hasMore: start + size <= TOTAL_FEED, list }];
  },
  'POST /like'(q, data) {
    const id = data && data.id;
    if (!id || id > TOTAL_FEED) return [404, { message: '没有这条' }];
    liked[id] = !!data.liked;
    return [200, { id, liked: liked[id], likes: feedItem(id).likes }];
  },
  'GET /profile'() {
    return [200, { name: '林小满', level: 'Lv.6', city: '杭州', joinedDays: 412, color: '#10aeff' }];
  },
  'GET /stats'() {
    return [200, { orders: 128, amount: 23840.5, coupons: 6, points: 3260 }];
  },
  'GET /trend'(q) {
    const days = Math.min(14, parseInt(q.days || '7', 10));
    const points = [];
    for (let d = 0; d < days; d++) points.push({ day: '周' + '一二三四五六日'[d % 7], value: 30 + ((d * 53) % 70) });
    return [200, { days, points }];
  },
  'GET /orders'(q) {
    const limit = Math.min(10, parseInt(q.limit || '5', 10));
    const states = ['待发货', '运输中', '已签收', '已完成'];
    const goods = ['无线耳机', '机械手表', '咖啡豆 500g', '运动鞋', '生日蛋糕', '手机壳'];
    const list = [];
    for (let i = 0; i < limit; i++) {
      list.push({ no: 'NO.2026' + (1017 - i), name: goods[i % goods.length], price: (59 + i * 37).toFixed(2), state: states[i % states.length] });
    }
    return [200, { list }];
  }
};

/** 发一个模拟请求。回调形状与 wx.request 一致：success({ statusCode, data, header }) / fail({ errMsg }) */
function request(opts) {
  const method = (opts.method || 'GET').toUpperCase();
  const url = opts.url || '';
  const path = url.replace(/^mock:\/\/api/, '').split('?')[0];
  const handler = routes[method + ' ' + path];
  // 延迟上下浮动 ±30%，像真网络那样每次不一样
  const delay = Math.round(options.latency * (0.7 + Math.random() * 0.6));
  const timer = setTimeout(() => {
    if (Math.random() < options.failRate) {
      opts.fail && opts.fail({ errMsg: 'request:fail 模拟网络错误（失败率 ' + Math.round(options.failRate * 100) + '%）' });
    } else if (!handler) {
      opts.success && opts.success({ statusCode: 404, data: { message: '没有这个接口：' + method + ' ' + path }, header: {} });
    } else {
      const [statusCode, data] = handler(query(url), opts.data);
      // 深拷贝一份：页面改了拿到的对象，不该改到「服务端」
      opts.success && opts.success({ statusCode, data: JSON.parse(JSON.stringify(data)), header: { 'content-type': 'application/json' } });
    }
    opts.complete && opts.complete();
  }, delay);
  return { abort() { clearTimeout(timer); opts.fail && opts.fail({ errMsg: 'request:fail abort' }); } };
}

function setOptions(o) {
  Object.assign(options, o);
}

function getOptions() {
  return Object.assign({}, options);
}

module.exports = { request, setOptions, getOptions };
