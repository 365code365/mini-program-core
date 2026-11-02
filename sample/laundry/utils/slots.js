// 取件时段：今天 / 明天 / 后天 × 6 个两小时时段。
// 今天只留「开始时间距现在 ≥ 1 小时」的时段（取件员要路上时间）；今天一个都不剩就从明天开始排
const data = require('./data.js');

const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
const LEAD_MIN = 60;

function pad(n) { return n < 10 ? '0' + n : '' + n; }

function slotStart(s) {
  const p = s.split('-')[0].split(':');
  return +p[0] * 60 + +p[1];
}

function todaySlots(now) {
  const m = now.getHours() * 60 + now.getMinutes();
  return data.SLOTS.filter(s => slotStart(s) - m >= LEAD_MIN);
}

function days(now) {
  now = now || new Date();
  const out = [];
  const startToday = todaySlots(now).length > 0;
  for (let i = startToday ? 0 : 1; out.length < 3; i++) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + i);
    const md = pad(d.getMonth() + 1) + '-' + pad(d.getDate());
    const name = i === 0 ? '今天' : i === 1 ? '明天' : i === 2 ? '后天' : WEEK[d.getDay()];
    out.push({ offset: i, date: d.getFullYear() + '-' + md, label: name + ' ' + md + ' ' + WEEK[d.getDay()] });
  }
  return out;
}

function slotsFor(day, now) {
  return day.offset === 0 ? todaySlots(now || new Date()) : data.SLOTS.slice();
}

module.exports = { days, slotsFor, todaySlots };
