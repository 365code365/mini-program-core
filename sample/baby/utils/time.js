// 日期 / 时长的格式化与月龄计算（全部按本地时间）
const DAY = 86400000;

function pad(n) { return n < 10 ? '0' + n : '' + n; }

function ymd(d) {
  d = d instanceof Date ? d : new Date(d);
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
}

function hm(d) {
  d = d instanceof Date ? d : new Date(d);
  return pad(d.getHours()) + ':' + pad(d.getMinutes());
}

// 'YYYY-MM-DD' + 'HH:mm' → 时间戳
function parse(dateStr, timeStr) {
  const [y, m, d] = String(dateStr).split('-').map(Number);
  const [hh, mm] = String(timeStr || '00:00').split(':').map(Number);
  return new Date(y, m - 1, d, hh || 0, mm || 0, 0).getTime();
}

function startOfDay(ts) {
  const d = new Date(ts);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

function isToday(ts) { return startOfDay(ts) === startOfDay(Date.now()); }

// 秒 → mm:ss（超过一小时 h:mm:ss）
function clock(sec) {
  sec = Math.max(0, Math.floor(sec));
  const h = Math.floor(sec / 3600), m = Math.floor(sec % 3600 / 60), s = sec % 60;
  return (h ? h + ':' + pad(m) : pad(m)) + ':' + pad(s);
}

// 分钟 → 「1小时20分」
function duration(min) {
  min = Math.round(min);
  if (min < 60) return min + '分钟';
  return Math.floor(min / 60) + '小时' + (min % 60 ? (min % 60) + '分' : '');
}

// 出生到某天：{ days, months(整月), rest(余天), monthFloat, label }
function age(birthday, at) {
  const b = new Date(parse(birthday));
  const t = new Date(startOfDay(at || Date.now()));
  const days = Math.max(0, Math.round((t - b) / DAY));
  let months = (t.getFullYear() - b.getFullYear()) * 12 + t.getMonth() - b.getMonth();
  if (t.getDate() < b.getDate()) months -= 1;
  months = Math.max(0, months);
  const anchor = new Date(b.getFullYear(), b.getMonth() + months, b.getDate());
  const rest = Math.max(0, Math.round((t - anchor) / DAY));
  let label;
  if (months === 0) label = days + '天';
  else if (months < 24) label = months + '个月' + (rest ? rest + '天' : '');
  else label = Math.floor(months / 12) + '岁' + (months % 12 ? (months % 12) + '个月' : '');
  return { days, months, rest, monthFloat: days / 30.4375, label };
}

// 生日加 n 个月
function addMonths(birthday, n) {
  const b = new Date(parse(birthday));
  return ymd(new Date(b.getFullYear(), b.getMonth() + n, b.getDate()));
}

function daysBetween(a, b) { return Math.round((startOfDay(parse(b)) - startOfDay(parse(a))) / DAY); }

function cnDate(str) {
  const [y, m, d] = String(str).split('-').map(Number);
  return y + '年' + m + '月' + d + '日';
}

module.exports = { DAY, pad, ymd, hm, parse, startOfDay, isToday, clock, duration, age, addMonths, daysBetween, cnDate };
