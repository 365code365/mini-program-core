// 格式化小工具（模板里不能对字符串取下标 / 调方法，字段一律在 JS 里算好）
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function dateTime(ts) {
  const d = new Date(ts);
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes());
}
function monthDay(ts) { const d = new Date(ts); return pad(d.getMonth() + 1) + '月' + pad(d.getDate()) + '日 ' + pad(d.getHours()) + ':' + pad(d.getMinutes()); }
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
// 剩余毫秒 → { d, h, m, s, text }
function countdown(ms) {
  if (ms < 0) ms = 0;
  const s = Math.floor(ms / 1000);
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60), sec = s % 60;
  return { d, h: pad(h), m: pad(m), s: pad(sec), text: (d > 0 ? d + '天' : '') + pad(h) + ':' + pad(m) + ':' + pad(sec) };
}
function wantText(n) { return n >= 10000 ? (n / 10000).toFixed(1) + '万' : '' + n; }
const STATUS = { unpaid: '待支付', paid: '待观演', refunding: '退票审核中', refunded: '已退票', cancelled: '已取消' };
function orderNo() {
  const d = new Date();
  return 'XC' + d.getFullYear() + pad(d.getMonth() + 1) + pad(d.getDate()) + pad(d.getHours()) + pad(d.getMinutes()) + Math.floor(1000 + Math.random() * 9000);
}
module.exports = { pad, dateTime, monthDay, WEEK, countdown, wantText, STATUS, orderNo };
