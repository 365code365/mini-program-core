// 进行中的那一单：（预约）→ 呼叫 → 应答 → 司机赶来 → 司机到达 → 行程中 → 到达（生成待支付行程）。
// 进度按「拍」走：页面每秒 tick 一次，拍数随订单存进 storage，不拿 Date.now() 相减 ——
// 离开页面再回来接着走；固定时钟（MINI_FAKE_NOW）的无头快照里也照样推进。
// 演示把时间压缩了：1 拍 ≈ 行程里 20 秒（SPEED）。只有预约单「到点出发」看墙上时钟
const data = require('./data.js');
const store = require('./store.js');
const KEY = 'ride_order';
const SPEED = 20;
const ACCEPT_AT = 4;      // 呼叫 4 拍后应答
const APPROACH = 9;       // 司机赶来 9 拍（≈ 3 分钟）
const FREE_CANCEL = 6;    // 接单 6 拍（≈ 2 分钟）内取消免费
const BOARD_AT = 8;       // 司机到达后 8 拍接到乘客（乘客也可以点「我已上车」）
const FREE_WAIT = 15;     // 免费等待 15 拍（≈ 5 分钟）
const RIDE = 12;          // 行程 12 拍
const BOOK_LEAD = 15 * 60000; // 预约单：出发前 15 分钟司机出发

function get() {
  const o = wx.getStorageSync(KEY);
  return o && o.id ? o : null;
}
function save(o) {
  wx.setStorageSync(KEY, o);
}
function clear() {
  wx.removeStorageSync(KEY);
}
function mmss(sec) {
  const s = Math.max(0, Math.round(sec));
  return store.pad(Math.floor(s / 60)) + ':' + store.pad(s % 60);
}
function whenText(ts) {
  const d = new Date(ts);
  const today = new Date();
  const tomorrow = new Date(Date.now() + 86400000);
  const day = d.toDateString() === today.toDateString() ? '今天' : d.toDateString() === tomorrow.toDateString() ? '明天' : (d.getMonth() + 1) + '月' + d.getDate() + '日';
  return day + ' ' + store.hm(ts);
}
// 预约时段：今天从半小时后起每半小时一档（最多 8 档），明天 07:00 起 8 档
function slots() {
  const H = 30 * 60000;
  const now = Date.now();
  const end = new Date(now);
  end.setHours(23, 30, 0, 0);
  const today = [];
  for (let t = Math.ceil((now + H) / H) * H; t <= end.getTime() && today.length < 8; t += H) today.push(t);
  const tm = new Date(now + 86400000);
  tm.setHours(7, 0, 0, 0);
  const tomorrow = [];
  for (let i = 0; i < 8; i++) tomorrow.push(tm.getTime() + i * H);
  return [{ name: '今天', list: today }, { name: '明天', list: tomorrow }];
}

function pickDriver(typeKey) {
  return Object.assign({}, data.DRIVERS[typeKey === 'premier' ? 1 : typeKey === 'pool' ? 2 : 0]);
}
function create(opts) {
  const k = data.km(opts.from, opts.to);
  const min = data.minutes(k);
  const t = data.carType(opts.typeKey);
  const now = Date.now();
  const o = {
    id: 'T' + now, typeKey: t.key, type: t.name, from: opts.from, to: opts.to, km: k, min,
    est: data.fare(t.key, k, min).total,
    passenger: opts.passenger || null, when: opts.when || 0,
    stage: opts.when ? 'booked' : 'waiting', t: 0, createdAt: now, msgs: [], shared: false, driver: null
  };
  // 预约单下单即派司机
  if (o.when) o.driver = pickDriver(o.typeKey);
  save(o);
  return o;
}
// 上车：夜间（22:00-05:00）且开了「自动分享」、有紧急联系人，就自动把行程发给他们（真 App 由服务端发短信）
function board(o) {
  o.stage = 'ongoing';
  o.startT = o.t;
  const contacts = store.getContacts();
  if (!o.shared && store.autoShare() && store.isNight(Date.now()) && contacts.length) {
    o.shared = true;
    o.autoShared = contacts.map(c => c.name).join('、');
  }
}
// 乘客点「我已上车」
function boardNow() {
  const o = get();
  if (!o || o.stage !== 'arrived') return null;
  board(o);
  save(o);
  return o;
}
// 走一拍，返回这一拍发生的事：accepted / depart / arrived / started / finished / reply / ''
function tick(o) {
  o.t += 1;
  let ev = '';
  if (o.stage === 'booked') {
    if (Date.now() >= o.when - BOOK_LEAD) {
      o.stage = 'accepted';
      o.acceptT = o.t;
      ev = 'depart';
    }
  } else if (o.stage === 'waiting') {
    if (o.t >= ACCEPT_AT) {
      o.driver = pickDriver(o.typeKey);
      o.stage = 'accepted';
      o.acceptT = o.t;
      ev = 'accepted';
    }
  } else if (o.stage === 'accepted') {
    if (o.t - o.acceptT >= APPROACH) {
      o.stage = 'arrived';
      o.arriveT = o.t;
      ev = 'arrived';
    }
  } else if (o.stage === 'arrived') {
    if (o.t - o.arriveT >= BOARD_AT) {
      board(o);
      ev = 'started';
    }
  } else if (o.stage === 'ongoing' && o.t - o.startT >= RIDE) {
    ev = 'finished';
  }
  if (o.replyAt && o.t >= o.replyAt) {
    o.seq = (o.seq || 0) + 1;
    o.msgs.push({ k: o.seq, me: false, text: o.reply });
    o.msgs = o.msgs.slice(-4);
    o.replyAt = 0;
    ev = ev || 'reply';
  }
  save(o);
  return ev;
}
// 页面要显示的随时间变的那部分
function view(o) {
  const v = { stage: o.stage };
  if (o.stage === 'waiting') v.waited = mmss(o.t);
  if (o.stage === 'booked') v.whenText = whenText(o.when);
  if (o.stage === 'accepted') {
    const p = Math.min(1, (o.t - o.acceptT) / APPROACH);
    const d = o.driver.dist * (1 - p);
    v.etaMin = Math.max(1, Math.ceil(o.driver.eta * (1 - p)));
    v.distText = d >= 1 ? store.round1(d) + '公里' : Math.max(50, Math.round(d * 100) * 10) + '米';
    const free = FREE_CANCEL - (o.t - o.acceptT);
    v.freeLeft = free > 0 ? mmss(free * SPEED) : '';
  }
  if (o.stage === 'arrived') v.waitLeft = mmss((FREE_WAIT - (o.t - o.arriveT)) * SPEED);
  if (o.stage === 'ongoing') {
    const q = Math.min(1, (o.t - o.startT) / RIDE);
    v.progress = Math.round(q * 100);
    v.traveled = store.round1(o.km * q);
    v.leftMin = Math.max(1, Math.ceil(o.min * (1 - q)));
    v.arriveText = store.hm(Date.now() + v.leftMin * 60000);
  }
  return v;
}
// 取消规则：应答前、预约单出发前、接单 2 分钟内免费；之后（含司机已到达）收 5 元；司机原因不收；行程中不能取消（-1）
function cancelFee(o, reason) {
  if (o.stage === 'ongoing') return -1;
  if (reason !== undefined && data.DRIVER_FAULT.indexOf(reason) >= 0) return 0;
  if (o.stage === 'waiting' || o.stage === 'booked') return 0;
  if (o.stage === 'accepted' && o.t - o.acceptT < FREE_CANCEL) return 0;
  return data.CANCEL_FEE;
}
function baseTrip(o) {
  const d = o.driver;
  return {
    id: o.id, typeKey: o.typeKey, type: o.type, from: o.from, to: o.to, km: o.km, min: o.min,
    date: store.dayText(o.when || o.createdAt), createdAt: o.createdAt, when: o.when,
    plate: d ? d.plate : '-', driver: d ? d.name : '-', car: d ? d.model + ' · ' + d.color : '-', driverPhone: d ? d.phone : '',
    passenger: o.passenger, shared: o.shared
  };
}
// 取消：有取消费的生成待支付单（去支付页付），没有的直接记成已取消
function cancel(o, reason) {
  const fee = cancelFee(o, reason);
  if (fee < 0) return null;
  const trip = Object.assign(baseTrip(o), {
    cancelled: true, cancelStage: o.stage, cancelFee: fee, pay: 0, status: fee ? 'unpaid' : 'cancel',
    cancelReason: reason === undefined ? (o.stage === 'booked' ? '取消预约' : '应答前取消呼叫') : data.CANCEL_REASONS[reason]
  });
  store.addTrip(trip);
  clear();
  return trip;
}
// 这一单用哪张券：券页「去使用」点过的那张能用就用它，否则挑最省的。
// 按结束时的终点重新挑（行程中改了目的地，满减门槛可能变了），不沿用下单时那张
function couponOf(o, total) {
  return store.pickFor({ typeKey: o.typeKey, from: o.from, to: o.to, amount: total, at: o.when || o.createdAt });
}
// 呼叫 / 行程中页面上的「预估 ¥X」：券后价（改了目的地跟着变）
function estimate(o) {
  const total = data.fare(o.typeKey, o.km, o.min).total;
  const c = couponOf(o, total);
  return { total, cut: c.cut, pay: store.round1(total - c.cut), couponName: c.coupon ? c.coupon.name : '' };
}
// 到达终点：按实际里程计费，券按 couponOf 选（支付页还能换）
function finish(o) {
  const trip = baseTrip(o);
  trip.fare = data.fare(o.typeKey, o.km, o.min);
  const best = couponOf(o, trip.fare.total);
  Object.assign(trip, {
    status: 'unpaid', couponId: best.coupon ? best.coupon.id : '', couponTitle: best.coupon ? best.coupon.name : '',
    couponCut: best.cut, finishedAt: Date.now()
  });
  store.addTrip(trip);
  // 这一程走完了：首页回到「你要去哪儿」
  store.setTo(null);
  clear();
  return trip;
}
// 页面每秒调一次：走一拍；到终点就生成待支付行程。返回 { o, ev, trip }（每次都从 storage 现读，别的页面改过的也算数）
function step() {
  const o = get();
  if (!o) return { o: null, ev: '' };
  const ev = tick(o);
  if (ev === 'finished') return { o: null, ev, trip: finish(o) };
  return { o, ev };
}
// 改目的地：按新终点重新估里程与车费
function changeDest(p) {
  const o = get();
  if (!o) return null;
  o.to = store.place(p);
  o.km = data.km(o.from, o.to);
  o.min = data.minutes(o.km);
  o.est = data.fare(o.typeKey, o.km, o.min).total;
  save(o);
  return o;
}
function sendMsg(o, i) {
  o.seq = (o.seq || 0) + 1;
  o.msgs.push({ k: o.seq, me: true, text: data.QUICK_MSGS[i] });
  o.msgs = o.msgs.slice(-4);
  o.reply = data.DRIVER_REPLY[i];
  o.replyAt = o.t + 2;
  save(o);
}
function shareText(o) {
  const d = o.driver;
  const car = d ? d.color + d.model + '（' + d.plate + '）' : o.type;
  return '【滴达出行】我正在乘坐' + car + '，从' + o.from.name + '前往' + o.to.name + '。实时行程：https://dida.example.com/s/' + o.id;
}
// 首页横条上的一句话
function banner(o) {
  const v = view(o);
  const map = {
    booked: '预约 ' + (v.whenText || '') + ' · ' + (o.driver ? o.driver.name : o.type),
    waiting: '正在为你呼叫' + o.type + '，已等待 ' + (v.waited || ''),
    accepted: '司机正在赶来 · 约 ' + v.etaMin + ' 分钟',
    arrived: '司机已到达上车点，请尽快上车',
    ongoing: '行程中 · 预计 ' + v.arriveText + ' 到达'
  };
  return { text: map[o.stage] || '', stage: o.stage };
}

module.exports = {
  SPEED, FREE_CANCEL, APPROACH, RIDE, get, save, clear, mmss, whenText, slots, create, board, boardNow, tick, step, view,
  cancelFee, cancel, estimate, finish, changeDest, sendMsg, shareText, banner
};
