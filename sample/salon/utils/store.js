// 跨页数据：预约、会员卡、收藏、我的评价，统一走 storage（退出重进还在）
const data = require('./data.js');

const K_APPTS = 'salon_appts';
const K_MEMBER = 'salon_member';
const K_FAVS = 'salon_favs';
const K_PROFILE = 'salon_profile';

function read(key, dflt) {
  try {
    const v = wx.getStorageSync(key);
    return v === '' || v === undefined || v === null ? dflt : v;
  } catch (e) { return dflt; }
}
function write(key, v) { wx.setStorageSync(key, v); }

// ---------- 预约 ----------
function appts() { return read(K_APPTS, []); }
function saveAppts(list) { write(K_APPTS, list); }
function appt(id) { return appts().find(a => a.id === id) || null; }
function addAppt(a) { const l = appts(); l.unshift(a); saveAppts(l); return a; }
function updateAppt(id, patch) {
  const l = appts();
  const i = l.findIndex(a => a.id === id);
  if (i < 0) return null;
  l[i] = Object.assign({}, l[i], patch);
  saveAppts(l);
  return l[i];
}

// ---------- 会员卡 ----------
function member() {
  return read(K_MEMBER, { cardNo: '6688 2026 1102 0731', balance: 0, points: 320, growth: 1260, records: [
    { id: 'm0', type: 'points', title: '新会员注册礼', amount: 200, time: '2026-09-18 14:20' },
    { id: 'm1', type: 'spend', title: '设计师洗剪吹 · Lily 林', amount: -88, time: '2026-09-18 15:32' }
  ] });
}
function saveMember(m) { write(K_MEMBER, m); }
function levelOf(growth) {
  let cur = data.levels[0], next = null;
  for (let i = 0; i < data.levels.length; i++) {
    if (growth >= data.levels[i].min) { cur = data.levels[i]; next = data.levels[i + 1] || null; }
  }
  const percent = next ? Math.round((growth - cur.min) * 100 / (next.min - cur.min)) : 100;
  return { cur, next, percent, gap: next ? next.min - growth : 0 };
}
function addRecord(m, type, title, amount) {
  m.records.unshift({ id: 'm' + Date.now(), type, title, amount, time: nowText() });
  if (m.records.length > 30) m.records.length = 30;
}

// ---------- 收藏 / 资料 ----------
function favs() { return read(K_FAVS, []); }
function toggleFav(id) {
  const l = favs();
  const i = l.indexOf(id);
  if (i >= 0) l.splice(i, 1); else l.push(id);
  write(K_FAVS, l);
  return i < 0;
}
function profile() { return read(K_PROFILE, { name: '', phone: '' }); }
function saveProfile(p) { write(K_PROFILE, p); }

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function nowText() {
  const d = new Date();
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

module.exports = { appts, appt, addAppt, updateAppt, saveAppts, member, saveMember, levelOf, addRecord,
  favs, toggleFav, profile, saveProfile, pad, nowText };
