// 通用工具：本地存储、身份证校验、节流、开放状态、余票
const { HALLS, EXHIBITS, SLOTS, ymd, addDays, pad } = require('./data.js');

const KEYS = { fav: 'mu_fav', history: 'mu_history', bookings: 'mu_bookings', visitors: 'mu_visitors', events: 'mu_events' };

function load(key, def) {
  const v = wx.getStorageSync(KEYS[key] || key);
  return v === '' || v === undefined || v === null ? def : v;
}
function save(key, v) { wx.setStorageSync(KEYS[key] || key, v); }

const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];

// 18 位身份证：前 17 位加权求和 mod 11 → 校验位（GB 11643-1999）
function checkIdCard(id) {
  id = String(id || '').toUpperCase();
  if (!/^\d{17}[\dX]$/.test(id)) return '身份证号应为 18 位';
  const y = +id.slice(6, 10), m = +id.slice(10, 12), d = +id.slice(12, 14);
  const dt = new Date(y, m - 1, d);
  if (y < 1900 || dt.getMonth() !== m - 1 || dt.getDate() !== d) return '身份证号中的出生日期不正确';
  const w = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
  let s = 0;
  for (let i = 0; i < 17; i++) s += +id[i] * w[i];
  if ('10X98765432'[s % 11] !== id[17]) return '身份证号校验位不正确，请核对';
  return '';
}
function maskId(id) { return id.slice(0, 4) + '**********' + id.slice(14); }
function maskName(n) { return n.length <= 1 ? n : n[0] + '*'.repeat(n.length - 1); }

function throttle(fn, ms) {
  let last = 0, timer = null;
  return function (...args) {
    const now = Date.now(), ctx = this;
    clearTimeout(timer);
    if (now - last >= ms) { last = now; fn.apply(ctx, args); }
    else timer = setTimeout(() => { last = Date.now(); fn.apply(ctx, args); }, ms - (now - last));
  };
}

function isClosed(d) { return d.getDay() === 1; }

// 今日开放状态：按当前时刻给「开放中 / 未开馆 / 已停止入馆 / 今日闭馆」
function openState(now) {
  now = now || new Date();
  if (isClosed(now)) return { text: '今日闭馆', tip: '每周一闭馆，明日 09:00 开放', open: false };
  const m = now.getHours() * 60 + now.getMinutes();
  if (m < 540) return { text: '尚未开馆', tip: '今日 09:00 开馆', open: false };
  if (m >= 960) return { text: '停止入馆', tip: '16:00 停止入馆，17:00 闭馆', open: false };
  return { text: '开放中', tip: '09:00-17:00，16:00 停止入馆', open: true };
}

// 余票：按日期与时段算一个稳定的已约人数，再扣掉本机已约的人数
function slotsFor(date) {
  const mine = load('bookings', []).filter(b => b.date === date && b.status !== '已取消');
  let seed = 0;
  for (const c of date) seed = (seed * 31 + c.charCodeAt(0)) % 9973;
  const wk = new Date(date.replace(/-/g, '/')).getDay();
  return SLOTS.map((s, i) => {
    const busy = wk === 0 || wk === 6 ? 0.9 : 0.55;
    let used = Math.floor(s.cap * busy + ((seed * (i + 3)) % 97) * 3);
    if (wk === 6 && i === 1) used = s.cap; // 周六上午场常年约满
    used += mine.filter(b => b.slotId === s.id).reduce((a, b) => a + b.visitors.length, 0);
    const left = Math.max(0, s.cap - used);
    return Object.assign({}, s, { left, full: left === 0, tight: left > 0 && left < 100 });
  });
}

// 未来 7 天可约日期（跳过周一）
function bookDays() {
  const out = [], t = new Date();
  for (let i = 0; out.length < 7 && i < 14; i++) {
    const d = addDays(t, i);
    if (isClosed(d)) continue;
    out.push({ date: ymd(d), md: (d.getMonth() + 1) + '/' + d.getDate(), wk: i === 0 ? '今天' : i === 1 ? '明天' : WEEK[d.getDay()] });
  }
  return out;
}

function hallOf(id) { return HALLS.find(h => h.id === id); }
function exhibit(id) { const e = EXHIBITS[id]; return e ? Object.assign({ id }, e, { hallName: hallOf(e.hall).name }) : null; }

function toggleFav(id) {
  const fav = load('fav', []);
  const i = fav.indexOf(id);
  if (i >= 0) fav.splice(i, 1); else fav.unshift(id);
  save('fav', fav);
  return i < 0;
}
function pushHistory(id) {
  const h = load('history', []).filter(x => x.id !== id);
  h.unshift({ id, at: Date.now() });
  save('history', h.slice(0, 30));
}

module.exports = { pad2: pad, load, save, WEEK, checkIdCard, maskId, maskName, throttle, openState, isClosed, slotsFor, bookDays,
  hallOf, exhibit, toggleFav, pushHistory };
