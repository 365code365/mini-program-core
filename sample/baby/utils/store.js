// 本地数据：所有记录都落 storage，退出重进还在。首次启动按「今天」生成一份示例数据（宝宝约 6 个半月）。
const T = require('./time.js');

const K = {
  seeded: 'baby_seeded',
  profile: 'baby_profile',
  feeds: 'baby_feeds',
  sleeps: 'baby_sleeps',
  diapers: 'baby_diapers',
  growth: 'baby_growth',
  vaccines: 'baby_vaccines',
  moments: 'baby_moments',
  favs: 'baby_favs',
  feedTimer: 'baby_timer_feed',
  sleepTimer: 'baby_timer_sleep'
};

const RECORD_KEYS = [K.feeds, K.sleeps, K.diapers, K.growth, K.vaccines, K.moments, K.feedTimer, K.sleepTimer];

function uid(p) { return (p || 'r') + Date.now().toString(36) + Math.floor(Math.random() * 1e4).toString(36); }

function get(key, def) {
  const v = wx.getStorageSync(key);
  return v === '' || v === undefined || v === null ? def : v;
}
function set(key, v) { wx.setStorageSync(key, v); }

function seed(force) {
  if (!force && get(K.seeded, 0)) return;
  const now = Date.now();
  const today = T.startOfDay(now);
  const birthday = T.ymd(today - 198 * T.DAY);
  const min = 60000, hour = 3600000;
  const profile = {
    name: '小米粒', fullName: '陈思远', sex: 'boy', birthday, birthTime: '08:26',
    birthWeight: '3.35', birthHeight: '50', blood: 'O型', avatar: '/images/avatar.jpg'
  };
  const feeds = [
    { id: uid('f'), type: 'breast', ts: now - 50 * min, left: 9 * 60 + 40, right: 7 * 60 + 15 },
    { id: uid('f'), type: 'formula', ts: now - 4 * hour, ml: 150, note: '飞鹤星飞帆 2 段' },
    { id: uid('f'), type: 'breast', ts: now - 7 * hour - 20 * min, left: 11 * 60, right: 8 * 60 + 30 },
    { id: uid('f'), type: 'formula', ts: now - 11 * hour, ml: 160, note: '' },
    { id: uid('f'), type: 'breast', ts: now - 26 * hour, left: 10 * 60, right: 10 * 60 }
  ];
  const sleeps = [
    { id: uid('s'), start: now - 3 * hour - 10 * min, end: now - 1 * hour - 35 * min },
    { id: uid('s'), start: now - 9 * hour, end: now - 7 * hour - 50 * min },
    { id: uid('s'), start: now - 15 * hour, end: now - 9 * hour - 30 * min }
  ];
  for (let d = 1; d <= 6; d++) {
    const base = today - d * T.DAY;
    sleeps.push({ id: uid('s'), start: base + 13 * hour, end: base + 13 * hour + (80 + d * 7) * min });
    sleeps.push({ id: uid('s'), start: base - 3 * hour + d * 5 * min, end: base + 6 * hour + (d % 3) * 20 * min });
  }
  const diapers = [
    { id: uid('d'), ts: now - 70 * min, kind: 'wet' },
    { id: uid('d'), ts: now - 5 * hour, kind: 'both' },
    { id: uid('d'), ts: now - 9 * hour, kind: 'wet' }
  ];
  const at = (m) => T.addMonths(birthday, m);
  const growth = [
    { id: uid('g'), date: birthday, weight: '3.35', height: '50.0', head: '34.2' },
    { id: uid('g'), date: at(1), weight: '4.62', height: '54.8', head: '37.1' },
    { id: uid('g'), date: at(2), weight: '5.71', height: '58.6', head: '39.0' },
    { id: uid('g'), date: at(4), weight: '7.10', height: '64.2', head: '41.6' },
    { id: uid('g'), date: at(6), weight: '8.05', height: '67.9', head: '43.4' }
  ];
  const done = (m, code) => ({ code, date: at(m) });
  // 6 月龄及以前的都已按时接种（下一针是 8 月龄的麻腮风 / 乙脑）
  const vaccines = {};
  require('./vaccines.js').LIST.filter((v) => v.month <= 6).forEach((v) => { vaccines[v.code] = done(v.month, v.code); });
  const moments = [
    { id: uid('m'), date: T.ymd(today - 3 * T.DAY), photo: '/images/photos/p5.jpg', caption: '第一次在大澡盆里玩小黄鸭，拍水拍得特别起劲', tag: '第一次', voice: null },
    { id: uid('m'), date: at(6), photo: '/images/photos/p4.jpg', caption: '半岁啦！外婆做了南瓜小蛋糕，米粒只能闻闻', tag: '节日', voice: { path: '/audio/lullaby.wav', duration: 6 } },
    { id: uid('m'), date: at(5), photo: '/images/photos/p3.jpg', caption: '天气转暖，第一次坐推车去公园晒太阳', tag: '出游', voice: null },
    { id: uid('m'), date: at(3), photo: '/images/photos/p2.jpg', caption: '抱着小熊睡得香，口水打湿了小熊的耳朵', tag: '日常', voice: null },
    { id: uid('m'), date: at(1), photo: '/images/photos/p6.jpg', caption: '满月留个小脚丫印，比妈妈的手指还短', tag: '第一次', voice: null }
  ];
  // 一次写入多个键（基础库 2.25.0 的批量接口）
  wx.batchSetStorageSync([
    { key: K.profile, value: profile },
    { key: K.feeds, value: feeds },
    { key: K.sleeps, value: sleeps },
    { key: K.diapers, value: diapers },
    { key: K.growth, value: growth },
    { key: K.vaccines, value: vaccines },
    { key: K.moments, value: moments },
    { key: K.favs, value: ['k6m-food'] },
    { key: K.seeded, value: 1 }
  ]);
}

function profile() { return get(K.profile, null) || (seed(true), get(K.profile, {})); }

function list(key) { return get(key, []); }

function add(key, rec) {
  const arr = list(key);
  rec.id = rec.id || uid(key.slice(5, 6));
  arr.unshift(rec);
  set(key, arr);
  return rec;
}

function remove(key, id) { set(key, list(key).filter((r) => r.id !== id)); }

// 首页「今日汇总」与时间轴
function today() {
  const feeds = list(K.feeds).filter((r) => T.isToday(r.ts));
  const sleeps = list(K.sleeps).filter((r) => T.isToday(r.end));
  const diapers = list(K.diapers).filter((r) => T.isToday(r.ts));
  const ml = feeds.reduce((s, r) => s + (r.type === 'formula' ? Number(r.ml) || 0 : 0), 0);
  const breastMin = Math.round(feeds.reduce((s, r) => s + (r.type === 'breast' ? (r.left || 0) + (r.right || 0) : 0), 0) / 60);
  const sleepMin = sleeps.reduce((s, r) => s + Math.max(0, r.end - Math.max(r.start, T.startOfDay(Date.now()))) / 60000, 0);
  return { feeds, sleeps, diapers, ml, breastMin, sleepMin };
}

module.exports = { K, RECORD_KEYS, uid, get, set, seed, profile, list, add, remove, today };
