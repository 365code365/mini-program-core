// 日期统一用本地时区的 'YYYY-MM-DD' 字符串（存储、比较、做 key 都方便）
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function fmt(d) { d = d || new Date(); return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function parse(s) { const p = s.split('-'); return new Date(+p[0], +p[1] - 1, +p[2]); }
function today() { return fmt(new Date()); }
function addDays(s, n) { const d = parse(s); d.setDate(d.getDate() + n); return fmt(d); }
// b - a 的天数
function diff(a, b) { return Math.round((parse(b) - parse(a)) / 86400000); }
const WEEK = ['日', '一', '二', '三', '四', '五', '六'];
function short(s) { const d = parse(s); return (d.getMonth() + 1) + '/' + d.getDate(); }
function weekday(s) { return '周' + WEEK[parse(s).getDay()]; }
function hm(d) { d = d || new Date(); return pad(d.getHours()) + ':' + pad(d.getMinutes()); }

module.exports = { fmt, parse, today, addDays, diff, short, weekday, hm, pad };
