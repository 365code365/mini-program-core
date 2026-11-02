// 已购课程、学习进度、收藏、打卡、订单、优惠券、积分、笔记、评价：都存在本地 storage
// 学习进度 zx_learn：{ 课程 id: { doneIds: [学完的课时 id], last: 下次要学的课时 id, at: 最近学习日期 } }
const data = require('./data.js');
function pad(v) { return v < 10 ? '0' + v : '' + v; }
function dayKey(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function daysAgo(n) { const t = new Date(); return dayKey(new Date(t.getFullYear(), t.getMonth(), t.getDate() - n)); }
function today() { return daysAgo(0); }
function stamp(d) { return dayKey(d) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes()); }
function money(n) { return (Math.round(n * 100) / 100).toFixed(2).replace(/\.00$/, ''); }
function lessonIds(c) { return data.flatLessons(c).map(l => l.id); }
// 首次打开：两门已购课程带一点进度，一门免费课已学完（学习中心、证书不是空的）
function defaultLearn() {
  const pick = (id, n, at) => {
    const ids = lessonIds(data.courseOf(id));
    return { doneIds: ids.slice(0, n), last: ids[Math.min(n, ids.length - 1)], at };
  };
  return { k1: pick('k1', 5, daysAgo(0)), k3: pick('k3', 2, daysAgo(2)), k7: pick('k7', 3, daysAgo(6)) };
}
function getLearn() {
  const v = wx.getStorageSync('zx_learn');
  if (!(v && typeof v === 'object' && Object.keys(v).length)) return defaultLearn();
  // 旧版只记了学完的节数 done：换算成前 done 节
  Object.keys(v).forEach(id => {
    const c = data.courseOf(id);
    if (c && !Array.isArray(v[id].doneIds)) v[id].doneIds = lessonIds(c).slice(0, Math.min(Number(v[id].done) || 0, c.lessons));
  });
  return v;
}
function saveLearn(v) { wx.setStorageSync('zx_learn', v); }
function owned(id) { return !!getLearn()[id]; }
function progress(id) {
  const c = data.courseOf(id);
  const l = getLearn()[id];
  const done = l ? l.doneIds.length : 0;
  return { done, total: c.lessons, pct: Math.min(100, Math.round(done / c.lessons * 100)), doneIds: l ? l.doneIds : [], last: l ? l.last : lessonIds(c)[0] };
}
function myCourses() {
  const learn = getLearn();
  return Object.keys(learn).filter(id => data.courseOf(id)).map(id => {
    const c = data.courseOf(id);
    const p = progress(id);
    return { id, title: c.title, cover: c.cover, mark: c.mark, teacher: data.teacherOf(c.teacher).name, lessons: c.lessons,
      done: p.done, pct: p.pct, at: learn[id].at, last: p.last, finished: p.pct >= 100 };
  }).sort((a, b) => (a.at < b.at ? 1 : -1));
}
// 累计学习分钟：所有已购课程里学完的课时时长相加
function learnedMins() {
  const learn = getLearn();
  return Object.keys(learn).filter(id => data.courseOf(id)).reduce((s, id) =>
    s + data.flatLessons(data.courseOf(id)).filter(l => learn[id].doneIds.indexOf(l.id) >= 0).reduce((a, l) => a + l.mins, 0), 0);
}
// 学完一节：返回 { first: 是否第一次学完, finished: 是否整门学完 }
function completeLesson(cid, lid) {
  const learn = getLearn();
  const l = learn[cid];
  if (!l) return { first: false, finished: false };
  const ids = lessonIds(data.courseOf(cid));
  const first = l.doneIds.indexOf(lid) < 0;
  if (first) l.doneIds.push(lid);
  const next = ids.find(x => l.doneIds.indexOf(x) < 0);
  l.last = next || lid;
  l.at = today();
  saveLearn(learn);
  if (first) {
    const les = data.flatLessons(data.courseOf(cid)).find(x => x.id === lid);
    addTodayMins(les ? les.mins : 20);
    wx.setStorageSync('zx_learn_day', today());
  }
  return { first, finished: l.doneIds.length >= ids.length };
}
// 今日学习分钟数：{ date, mins }，跨天归零（首次打开当天记 46 分钟的演示数据）
function todayMins() {
  const v = wx.getStorageSync('zx_today');
  if (v && typeof v === 'object') return v.date === today() ? v.mins : 0;
  return v === '' ? 46 : 0;
}
function addTodayMins(n) { wx.setStorageSync('zx_today', { date: today(), mins: todayMins() + n }); }

function getFav() { const v = wx.getStorageSync('zx_fav'); return Array.isArray(v) ? v : []; }
function saveFav(v) { wx.setStorageSync('zx_fav', v); }
// 打卡：默认连续打了 6 天（不含今天），外加月初零星几天
function getCheckins() {
  const v = wx.getStorageSync('zx_checkin');
  if (Array.isArray(v)) return v;
  const list = [];
  for (let i = 1; i <= 6; i++) list.push(daysAgo(i));
  [9, 10, 12, 15, 16, 17].forEach(n => list.push(daysAgo(n)));
  return list;
}
function saveCheckins(v) { wx.setStorageSync('zx_checkin', v); }
function streak(list) {
  const set = {};
  list.forEach(k => { set[k] = true; });
  let n = 0;
  let i = set[daysAgo(0)] ? 0 : 1;
  while (set[daysAgo(i)]) { n++; i++; }
  return n;
}

// ---------- 积分 ----------
function getPoints() { const v = wx.getStorageSync('zx_points'); return typeof v === 'number' ? v : 180; }
function addPoints(n) { const v = getPoints() + n; wx.setStorageSync('zx_points', v); return v; }

// ---------- 优惠券：[{ id, name, cut, min, until, used }] ----------
function getCoupons() {
  const v = wx.getStorageSync('zx_coupons');
  return Array.isArray(v) ? v : data.couponSeeds.map(c => Object.assign({ used: false }, c));
}
function saveCoupons(v) { wx.setStorageSync('zx_coupons', v); }
function usableCoupons(price) { return getCoupons().filter(q => !q.used && price >= q.min && q.cut < price); }
function setCouponUsed(id, used) {
  const list = getCoupons();
  list.forEach(q => { if (q.id === id) q.used = used; });
  saveCoupons(list);
}

// ---------- 订单：[{ id, cid, title, origin, price, cut, pay, coupon, time, status: paid / free / refunded }] ----------
function getOrders() {
  const v = wx.getStorageSync('zx_orders');
  if (Array.isArray(v)) return v;
  return [
    { id: 'o20260918', cid: 'k1', title: data.courseOf('k1').title, origin: 399, price: 199, cut: 0, pay: 199, coupon: '', time: daysAgo(45) + ' 21:16', status: 'paid' },
    { id: 'o20260826', cid: 'k3', title: data.courseOf('k3').title, origin: 199, price: 99, cut: 0, pay: 99, coupon: '', time: daysAgo(68) + ' 12:40', status: 'paid' },
    { id: 'o20260803', cid: 'k7', title: data.courseOf('k7').title, origin: 69, price: 0, cut: 0, pay: 0, coupon: '', time: daysAgo(90) + ' 08:05', status: 'free' }
  ];
}
function saveOrders(v) { wx.setStorageSync('zx_orders', v); }
// 下单（已付款 / 免费领取）：开通课程、核销券
function placeOrder(c, coupon) {
  const cut = coupon ? coupon.cut : 0;
  const o = { id: 'o' + Date.now(), cid: c.id, title: c.title, origin: c.origin, price: c.price, cut, pay: Math.max(0, c.price - cut), coupon: coupon ? coupon.id : '', time: stamp(new Date()), status: c.price === 0 ? 'free' : 'paid' };
  saveOrders([o].concat(getOrders()));
  if (coupon) setCouponUsed(coupon.id, true);
  const learn = getLearn();
  const ids = lessonIds(c);
  learn[c.id] = { doneIds: [], last: ids[0], at: today() };
  saveLearn(learn);
  return o;
}
// 退款规则：付费订单、7 天内、付费课时学了不到 3 节
function refundable(o) {
  if (o.status !== 'paid') return false;
  if ((Date.now() - new Date(o.time.replace(' ', 'T')).getTime()) > 7 * 86400000) return false;
  const c = data.courseOf(o.cid);
  const free = data.flatLessons(c).filter(l => l.free).map(l => l.id);
  return progress(o.cid).doneIds.filter(id => free.indexOf(id) < 0).length < 3;
}
// 退款：课程收回、券退回
function refund(oid) {
  const list = getOrders();
  const o = list.find(x => x.id === oid);
  if (!o) return null;
  o.status = 'refunded';
  o.refundAt = stamp(new Date());
  saveOrders(list);
  if (o.coupon) setCouponUsed(o.coupon, false);
  const learn = getLearn();
  delete learn[o.cid];
  saveLearn(learn);
  return o;
}

// ---------- 笔记：[{ id, cid, lid, text, pos, time }] ----------
function getNotes() { const v = wx.getStorageSync('zx_notes'); return Array.isArray(v) ? v : []; }
function saveNotes(v) { wx.setStorageSync('zx_notes', v); }

// ---------- 我的评价：{ 课程 id: { score, text, time } } ----------
function getMyReviews() { const v = wx.getStorageSync('zx_reviews'); return v && typeof v === 'object' ? v : {}; }
function saveMyReview(cid, r) { const v = getMyReviews(); v[cid] = r; wx.setStorageSync('zx_reviews', v); }

// ---------- 关注的讲师 ----------
function getFollow() { const v = wx.getStorageSync('zx_follow'); return Array.isArray(v) ? v : []; }
function saveFollow(v) { wx.setStorageSync('zx_follow', v); }

module.exports = {
  pad, dayKey, daysAgo, today, stamp, money, lessonIds,
  getLearn, saveLearn, owned, progress, myCourses, learnedMins, completeLesson, todayMins, addTodayMins,
  getFav, saveFav, getCheckins, saveCheckins, streak, getPoints, addPoints,
  getCoupons, saveCoupons, usableCoupons, setCouponUsed,
  getOrders, saveOrders, placeOrder, refundable, refund,
  getNotes, saveNotes, getMyReviews, saveMyReview, getFollow, saveFollow
};
