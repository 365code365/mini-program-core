// 预约档期：未来 7 天 × 每半小时一档。某档是否约满 = 发型师的固定排班（按 id+日期算出来，稳定可复现）+ 我自己已约的单
const store = require('./store.js');
const pad = store.pad;
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
const OPEN = 10 * 60, CLOSE = 21 * 60; // 最后一档 20:30 开始

function ymd(d) { return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`; }

function days(n) {
  const out = [];
  const base = new Date();
  for (let i = 0; i < (n || 7); i++) {
    const d = new Date(base.getFullYear(), base.getMonth(), base.getDate() + i);
    const label = i === 0 ? '今天' : i === 1 ? '明天' : WEEK[d.getDay()];
    out.push({ date: ymd(d), text: `${d.getMonth() + 1}月${d.getDate()}日 ${label}`, short: `${d.getMonth() + 1}/${d.getDate()}`, week: label });
  }
  return out;
}

function hash(s) {
  let h = 7;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 100003;
  return h;
}

// 某发型师某天的全部档期：{ time, state: 'free' | 'full' | 'past' | 'mine' }
function slots(stylistId, date, skipApptId) {
  const now = new Date();
  const today = ymd(now);
  const nowMin = now.getHours() * 60 + now.getMinutes();
  const mine = store.appts().filter(a => a.status === 'upcoming' && a.date === date && a.stylistId === stylistId && a.id !== skipApptId)
    .map(a => a.time);
  const out = [];
  for (let m = OPEN; m < CLOSE; m += 30) {
    const time = `${pad(Math.floor(m / 60))}:${pad(m % 60)}`;
    let state = 'free';
    if (date === today && m <= nowMin + 30) state = 'past';
    else if (mine.indexOf(time) >= 0) state = 'mine';
    else if (stylistId && hash(stylistId + date + time) % 10 < 3) state = 'full';
    out.push({ time, state, label: time + (state === 'full' ? '（约满）' : state === 'past' ? '（已过）' : state === 'mine' ? '（已约）' : '') });
  }
  return out;
}

function firstFree(list) {
  const i = list.findIndex(s => s.state === 'free');
  return i < 0 ? 0 : i;
}

module.exports = { days, slots, firstFree, ymd };
