// 格式化（模板里不对字符串取下标 / 调方法，字段都在 JS 里算好）
function pad(n) { return n < 10 ? '0' + n : '' + n; }
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
function dateKey(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function fromKey(k) { const p = k.split('-'); return new Date(Number(p[0]), Number(p[1]) - 1, Number(p[2])); }
function dateText(k) { const d = fromKey(k); return (d.getMonth() + 1) + '月' + d.getDate() + '日 ' + WEEK[d.getDay()]; }
function dateTime(ts) { const d = new Date(ts); return dateKey(d) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes()); }
function hh(h) { return pad(h) + ':00'; }
function orderNo() {
  const d = new Date();
  return 'YC' + dateKey(d).replace(/-/g, '') + pad(d.getHours()) + pad(d.getMinutes()) + pad(d.getSeconds()) + Math.floor(100 + Math.random() * 900);
}
function km(d) { return d < 1 ? Math.round(d * 1000) + 'm' : d.toFixed(1) + 'km'; }
module.exports = { pad, WEEK, dateKey, fromKey, dateText, dateTime, hh, orderNo, km };
