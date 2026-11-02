// 燃动健身本地状态，全部存 storage：团课预约、打卡、训练记录、身体数据、会员卡与订单、训练计划、个人资料、门店。
// 预约按时间推进（课结束还没签到记爽约、候补没排上自动取消），页面 onShow 先 tick() 再读
const data = require('./data.js');
const K = {
  bookings: 'fit_bookings', checkins: 'fit_checkins', workouts: 'fit_workouts', body: 'fit_body', profile: 'fit_profile',
  card: 'fit_card', orders: 'fit_orders', store: 'fit_store', plan: 'fit_plan'
};
const MIN = 60000;
const DAY = 86400000;
const DAILY_LIMIT = 2; // 会员每天最多约 2 节团课
const FREE_CANCEL = 2 * 3600 * 1000; // 开课前 2 小时外取消不算爽约
const SIGN_EARLY = 30 * MIN; // 开课前 30 分钟开始可以签到
const STRIKE_LIMIT = 3; // 30 天内爽约 3 次，暂停预约 7 天

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function dateStr(ts) { const d = new Date(ts); return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function hm(ts) { const d = new Date(ts); return pad(d.getHours()) + ':' + pad(d.getMinutes()); }
function dateTime(ts) { return dateStr(ts) + ' ' + hm(ts); }
function today() { return dateStr(Date.now()); }
function parseDate(s) { const p = String(s).split('-').map(Number); return new Date(p[0], p[1] - 1, p[2]).getTime(); }
function addDays(s, n) { const p = String(s).split('-').map(Number); return dateStr(new Date(p[0], p[1] - 1, p[2] + n).getTime()); }
function daysBetween(a, b) { return Math.round((parseDate(b) - parseDate(a)) / DAY); }
// getStorageSync 没有这一项时给 ''：列表判 Array.isArray，数字判 typeof，不用 `|| 默认值`
function arr(key) { const v = wx.getStorageSync(key); return Array.isArray(v) ? v : null; }
let seq = 0;
function uid(p) { seq += 1; return p + Date.now().toString(36).toUpperCase() + seq; }

// ───────── 个人资料 / 门店 ─────────
function getProfile() {
  const v = wx.getStorageSync(K.profile);
  return v && v.name ? v : { name: '陈一鸣', height: 176, goal: '减脂塑形' };
}
function saveProfile(p) { wx.setStorageSync(K.profile, p); }
function storeIdx() { const v = wx.getStorageSync(K.store); return typeof v === 'number' ? v : 0; }
function setStore(i) { wx.setStorageSync(K.store, i); }
function currentStore() { return data.stores[storeIdx()] || data.stores[0]; }

// ───────── 打卡与训练记录 ─────────
// 默认历史：过去 40 天里按固定规律练过，离线可复现
function defaultCheckins() {
  const out = [];
  for (let i = 40; i >= 1; i--) if ((i * 7) % 5 !== 0) out.push(addDays(today(), -i));
  return out;
}
function getCheckins() { return arr(K.checkins) || defaultCheckins(); }
function checkIn(date) {
  const l = getCheckins();
  const d = date || today();
  if (l.indexOf(d) >= 0) return false;
  l.push(d);
  l.sort();
  wx.setStorageSync(K.checkins, l.slice(-400));
  return true;
}
// 连续打卡：从今天（今天还没打就从昨天）往前数
function streak() {
  const set = {};
  getCheckins().forEach(d => { set[d] = true; });
  let d = set[today()] ? today() : addDays(today(), -1);
  let n = 0;
  while (set[d]) { n += 1; d = addDays(d, -1); }
  return n;
}
const DEFAULT_WORKOUTS = [
  { ago: 1, name: '燃脂搏击 BodyCombat', type: '团课', time: '07:30', min: 45, kcal: 512, hr: 152 },
  { ago: 1, name: '跑步机 · 间歇跑', type: '跑步', time: '19:10', min: 35, kcal: 178, hr: 141 },
  { ago: 2, name: '杠铃力量 BodyPump', type: '团课', time: '19:30', min: 55, kcal: 460, hr: 138 },
  { ago: 3, name: '核心塑形 Pilates', type: '团课', time: '18:30', min: 30, kcal: 210, hr: 118 },
  { ago: 4, name: '器械 · 胸背训练', type: '力量', time: '20:00', min: 62, kcal: 540, hr: 132 }
];
function defaultWorkouts() {
  return DEFAULT_WORKOUTS.map((w, i) => ({ id: 'w' + i, date: addDays(today(), -w.ago), name: w.name, type: w.type, time: w.time, min: w.min, kcal: w.kcal, hr: w.hr }));
}
// 新的在前：先按训练日期时间，同一分钟的按记录先后（at）
function getWorkouts() {
  return (arr(K.workouts) || defaultWorkouts()).slice().sort((a, b) => {
    const ka = a.date + ' ' + a.time;
    const kb = b.date + ' ' + b.time;
    return ka === kb ? (b.at || 0) - (a.at || 0) : (ka < kb ? 1 : -1);
  });
}
function addWorkout(w) {
  const l = getWorkouts();
  const one = Object.assign({ id: uid('w'), date: today(), time: hm(Date.now()), at: Date.now() }, w);
  l.unshift(one);
  wx.setStorageSync(K.workouts, l.slice(0, 200));
  checkIn(one.date);
  return one;
}
function removeWorkout(id) { wx.setStorageSync(K.workouts, getWorkouts().filter(w => w.id !== id)); }
function statsOf(date) {
  const ws = getWorkouts().filter(w => w.date === date);
  return { count: ws.length, min: ws.reduce((n, w) => n + w.min, 0), kcal: ws.reduce((n, w) => n + w.kcal, 0) };
}
// 本周（周一到周日）每天的时长与消耗
function weekStats() {
  const t = new Date();
  const monday = addDays(today(), -((t.getDay() + 6) % 7));
  const names = ['一', '二', '三', '四', '五', '六', '日'];
  const days = names.map((d, i) => Object.assign({ d, date: addDays(monday, i) }, statsOf(addDays(monday, i))));
  return {
    days,
    trained: days.filter(x => x.count).length,
    totalMin: days.reduce((n, x) => n + x.min, 0),
    totalKcal: days.reduce((n, x) => n + x.kcal, 0),
    todayIdx: (t.getDay() + 6) % 7
  };
}

// ───────── 身体数据 ─────────
function defaultBody() {
  const ws = [74.2, 73.8, 73.5, 73.1, 72.9, 72.5];
  // 每周称一次，最近一次是 2 天前
  return ws.map((w, i) => ({ id: 'b' + i, date: addDays(today(), -(ws.length - 1 - i) * 7 - 2), weight: w, fat: Math.round((20.4 - i * 0.35) * 10) / 10 }));
}
function getBody() { return (arr(K.body) || defaultBody()).slice().sort((a, b) => (a.date < b.date ? -1 : 1)); }
function upsertBody(r) {
  const l = getBody().filter(x => x.date !== r.date);
  l.push(Object.assign({ id: uid('b') }, r));
  l.sort((a, b) => (a.date < b.date ? -1 : 1));
  wx.setStorageSync(K.body, l.slice(-60));
}
function removeBody(id) { wx.setStorageSync(K.body, getBody().filter(x => x.id !== id)); }
function latestBody() { const l = getBody(); return l[l.length - 1] || null; }
function bmiOf(w, h) {
  const v = w / Math.pow(h / 100, 2);
  return { v: v.toFixed(1), text: v < 18.5 ? '偏瘦' : (v < 24 ? '正常' : (v < 28 ? '超重' : '肥胖')) };
}

// ───────── 会员卡与订单 ─────────
function getCard() {
  const v = wx.getStorageSync(K.card);
  return v && v.expire ? v : { plan: '黑金年卡', expire: addDays(today(), 286) };
}
function cardValid(date) { return getCard().expire >= (date || today()); }
function cardLeftDays() { return Math.max(0, daysBetween(today(), getCard().expire)); }
function getOrders() { return (arr(K.orders) || []).filter(o => o && o.kind); }
function saveOrders(l) { wx.setStorageSync(K.orders, l); }
function addOrder(o) { const l = getOrders(); l.unshift(o); saveOrders(l.slice(0, 50)); return o; }
function updateOrder(id, patch) {
  const l = getOrders();
  const o = l.find(x => x.id === id);
  if (o) Object.assign(o, patch);
  saveOrders(l);
  return o;
}
// 买 / 续卡：在原到期日上顺延（已过期就从今天算）
function buyCard(plan) {
  const card = getCard();
  const valid = card.expire >= today();
  const from = valid ? card.expire : today();
  const to = addDays(from, plan.days);
  // 卡名：过期了重新开卡就叫新买的；有效期内续短卡，保留原来更高档的卡名
  wx.setStorageSync(K.card, { plan: !valid || plan.days >= 365 ? plan.name : card.plan, expire: to });
  return addOrder({ id: 'F' + Date.now().toString().slice(-10), kind: 'card', title: plan.name, price: plan.price, days: plan.days, oldExpire: card.expire, oldPlan: card.plan, newExpire: to, status: 'paid', createdAt: Date.now() });
}
// 7 天内、而且是最近一笔购卡订单，可以全额退（有效期退回原来的日子）
function canRefund(o) {
  if (o.kind !== 'card' || o.status !== 'paid' || Date.now() - o.createdAt > 7 * DAY) return false;
  const latest = getOrders().find(x => x.kind === 'card' && x.status === 'paid');
  return !!latest && latest.id === o.id;
}
function refundCard(id) {
  const o = getOrders().find(x => x.id === id);
  if (!o || !canRefund(o)) return null;
  wx.setStorageSync(K.card, { plan: o.oldPlan, expire: o.oldExpire });
  return updateOrder(id, { status: 'refund', refundAt: Date.now() });
}

// ───────── 团课预约 ─────────
// status：booked 已预约 / wait 候补 / done 已上课（签到过）/ miss 爽约 / cancel 已取消（late：开课前 2 小时内取消，算爽约）
function getBookings() { return (arr(K.bookings) || []).filter(b => b && b.bid && b.date); } // 老版本按 dayIdx 存，换了日期就对不上，丢掉
function saveBookings(l) { wx.setStorageSync(K.bookings, l); }
function startOf(b) { const t = b.time.split(':').map(Number); return parseDate(b.date) + (t[0] * 60 + t[1]) * MIN; }
function endOf(b) { return startOf(b) + b.dur * MIN; }
function bookingOf(courseId, date) { return getBookings().find(b => b.courseId === courseId && b.date === date && b.status !== 'cancel') || null; }
function strikes(now) {
  now = now || Date.now();
  return getBookings().filter(b => (b.status === 'miss' || b.late) && now - startOf(b) < 30 * DAY).sort((a, b) => startOf(a) - startOf(b));
}
// 预约前的检查：返回 { err } 或 { needPay }（没有有效会员卡就按单次课付费）
function checkBook(c, now) {
  now = now || Date.now();
  const s = strikes(now);
  if (s.length >= STRIKE_LIMIT) {
    const until = startOf(s[s.length - 1]) + 7 * DAY;
    if (now < until) return { err: '近 30 天爽约 ' + s.length + ' 次，' + dateStr(until).slice(5) + ' 前暂停预约' };
  }
  const b = { date: c.date, time: c.time, dur: c.dur };
  if (now >= startOf(b)) return { err: now >= endOf(b) ? '这节课已经结束了' : '已开课，下次早点来' };
  const mine = getBookings().filter(x => x.date === c.date && (x.status === 'booked' || x.status === 'done'));
  if (c.left > 0 && mine.length >= DAILY_LIMIT) return { err: '每天最多预约 ' + DAILY_LIMIT + ' 节团课' };
  const clash = mine.find(x => startOf(x) < endOf(b) && startOf(b) < endOf(x));
  if (clash) return { err: '和已约的「' + clash.name + '」时间冲突' };
  return { needPay: !cardValid(c.date) };
}
// 满员就排候补（不占每日名额）
function book(c, paid) {
  const l = getBookings();
  const b = {
    bid: uid('k'), courseId: c.id, date: c.date, name: c.name, type: c.type, time: c.time, dur: c.dur, room: c.room, coach: c.coachInfo.name,
    kcal: c.kcal, status: c.left > 0 ? 'booked' : 'wait', rank: c.left > 0 ? 0 : 1 + (c.booked % 3), paid: paid || 0, store: currentStore().short, createdAt: Date.now()
  };
  l.unshift(b);
  saveBookings(l);
  if (paid) addOrder({ id: 'C' + Date.now().toString().slice(-10), kind: 'class', title: '单次团课 · ' + c.name, price: paid, bid: b.bid, status: 'paid', createdAt: Date.now() });
  return b;
}
function updateBooking(bid, patch) {
  const l = getBookings();
  const b = l.find(x => x.bid === bid);
  if (b) Object.assign(b, patch);
  saveBookings(l);
  return b;
}
function findBooking(bid) { return getBookings().find(b => b.bid === bid) || null; }
// 取消：候补随时取消；已约的开课前 2 小时内取消记 1 次爽约；单次课费用原路退回
function cancelBooking(bid, now) {
  now = now || Date.now();
  const b = findBooking(bid);
  if (!b || (b.status !== 'booked' && b.status !== 'wait') || now >= startOf(b)) return null;
  const late = b.status === 'booked' && startOf(b) - now < FREE_CANCEL;
  if (b.paid) {
    const o = getOrders().find(x => x.bid === bid);
    if (o) updateOrder(o.id, { status: 'refund', refundAt: now });
  }
  return updateBooking(bid, { status: 'cancel', cancelAt: now, late });
}
function signState(b, now) {
  now = now || Date.now();
  return b.status === 'booked' && b.date === today() && now >= startOf(b) - SIGN_EARLY && now < endOf(b);
}
// 签到 = 上了这节课：记进训练记录并打卡
function signIn(bid) {
  const b = findBooking(bid);
  if (!b || !signState(b)) return null;
  updateBooking(bid, { status: 'done', signedAt: Date.now() });
  addWorkout({ date: b.date, time: b.time, name: b.name, type: '团课', min: b.dur, kcal: b.kcal });
  return b;
}
function lateFree(b, now) { return startOf(b) - (now || Date.now()) >= FREE_CANCEL; }
// 候补：开课前 2 小时排在第 1 位的自动转为预约（演示「有人退课」）；到下课还没排上的自动取消
function tick(now) {
  now = now || Date.now();
  const l = getBookings();
  let changed = false;
  l.forEach(b => {
    if (b.status === 'wait' && b.rank === 1 && now >= startOf(b) - FREE_CANCEL && now < startOf(b)) {
      Object.assign(b, { status: 'booked', promoted: true });
      changed = true;
    } else if ((b.status === 'booked' || b.status === 'wait') && now >= endOf(b)) {
      if (b.status === 'booked') b.status = 'miss';
      else Object.assign(b, { status: 'cancel', cancelAt: endOf(b), waitFail: true });
      changed = true;
    }
  });
  if (changed) saveBookings(l);
}
function upcoming() {
  return getBookings().filter(b => b.status === 'booked' || b.status === 'wait').sort((a, b) => startOf(a) - startOf(b));
}

// ───────── 训练计划 ─────────
function getPlan() {
  const v = wx.getStorageSync(K.plan);
  return v && v.id && data.findPlan(v.id) ? v : null;
}
function startPlan(id) { wx.setStorageSync(K.plan, { id, start: today(), done: [] }); }
function quitPlan() { wx.removeStorageSync(K.plan); }
// 计划进度：第几天、今天练什么、每一天的状态
function planView(now) {
  const p = getPlan();
  if (!p) return null;
  const plan = data.findPlan(p.id);
  const day = Math.min(plan.days, daysBetween(p.start, dateStr(now || Date.now())) + 1);
  const finished = daysBetween(p.start, today()) + 1 > plan.days;
  const list = [];
  for (let i = 1; i <= plan.days; i++) {
    const t = plan.week[(i - 1) % 7];
    const date = addDays(p.start, i - 1);
    const done = p.done.indexOf(date) >= 0;
    list.push({ i, date, name: t.rest ? '休息日' : t.name, rest: !!t.rest, min: t.min || 0, kcal: t.kcal || 0, done, state: t.rest ? 'rest' : (done ? 'done' : (i < day ? 'missed' : (i === day ? 'today' : 'todo'))) });
  }
  const trainDays = list.filter(x => !x.rest).length;
  return { plan, start: p.start, day, finished, today: list[day - 1], list, doneCount: p.done.length, trainDays, pct: Math.round(p.done.length / trainDays * 100) };
}
function completePlanDay() {
  const v = planView();
  if (!v || v.finished || v.today.rest || v.today.done) return null;
  const p = getPlan();
  p.done.push(today());
  wx.setStorageSync(K.plan, p);
  return addWorkout({ name: v.plan.name + ' · ' + v.today.name, type: '计划', min: v.today.min, kcal: v.today.kcal });
}

module.exports = {
  MIN, DAY, DAILY_LIMIT, pad, dateStr, hm, dateTime, today, addDays, parseDate, daysBetween,
  getProfile, saveProfile, storeIdx, setStore, currentStore,
  getCheckins, checkIn, streak, getWorkouts, addWorkout, removeWorkout, statsOf, weekStats,
  getBody, upsertBody, removeBody, latestBody, bmiOf,
  getCard, cardValid, cardLeftDays, getOrders, buyCard, canRefund, refundCard,
  getBookings, bookingOf, findBooking, startOf, endOf, strikes, checkBook, book, cancelBooking, signState, signIn, lateFree, tick, upcoming,
  getPlan, startPlan, quitPlan, planView, completePlanDay
};
