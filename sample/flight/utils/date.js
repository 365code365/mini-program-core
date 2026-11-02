// 日期工具：一律用本地时区的 'YYYY-MM-DD' 字符串在页面间传递
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function fmt(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function parse(s) { const p = String(s).split('-').map(Number); return new Date(p[0], p[1] - 1, p[2]); }
function today() { const d = new Date(); return fmt(new Date(d.getFullYear(), d.getMonth(), d.getDate())); }
function addDays(s, n) { const d = parse(s); d.setDate(d.getDate() + n); return fmt(d); }
function diffDays(a, b) { return Math.round((parse(b) - parse(a)) / 86400000); }

// 「今天 / 明天 / 后天 / 周X」
function weekLabel(s) {
  const n = diffDays(today(), s);
  if (n === 0) return '今天';
  if (n === 1) return '明天';
  if (n === 2) return '后天';
  return WEEK[parse(s).getDay()];
}
function md(s) { const d = parse(s); return (d.getMonth() + 1) + '月' + d.getDate() + '日'; }
function mdShort(s) { const d = parse(s); return pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function hm(min) { const m = ((min % 1440) + 1440) % 1440; return pad(Math.floor(m / 60)) + ':' + pad(m % 60); }
function dur(min) { const h = Math.floor(min / 60), m = min % 60; return (h ? h + '小时' : '') + (m ? m + '分' : ''); }
function stamp(ts) { const d = new Date(ts); return fmt(d) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes()); }

module.exports = { WEEK, pad, fmt, parse, today, addDays, diffDays, weekLabel, md, mdShort, hm, dur, stamp };
