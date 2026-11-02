// 通用：金额 / 日期格式化、身份证校验（GB 11643-1999 校验位）、编号生成。
function pad(n) { return n < 10 ? '0' + n : '' + n; }

function money(n) {
  const v = Math.round(Number(n || 0) * 100) / 100;
  const parts = v.toFixed(2).split('.');
  parts[0] = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return parts.join('.');
}

function amountText(n) {
  n = Number(n);
  if (n >= 10000 && n % 10000 === 0) return n / 10000 + '万';
  return n + '元';
}

function ymd(d) {
  d = d instanceof Date ? d : new Date(d);
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
}

function ymdhm(d) {
  d = d instanceof Date ? d : new Date(d);
  return ymd(d) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes());
}

function addDays(d, n) { const x = new Date(d); x.setDate(x.getDate() + n); return x; }

// 周岁：过了今年生日才加一岁
function ageOf(birthday, now) {
  const b = new Date(birthday.replace(/-/g, '/'));
  now = now || new Date();
  let a = now.getFullYear() - b.getFullYear();
  if (now.getMonth() < b.getMonth() || (now.getMonth() === b.getMonth() && now.getDate() < b.getDate())) a--;
  return a;
}

const W = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
const CODES = '10X98765432';
// 返回 { ok, msg, birthday, gender }
function checkIdcard(raw) {
  const id = String(raw || '').trim().toUpperCase();
  if (!/^\d{17}[\dX]$/.test(id)) return { ok: false, msg: '身份证号应为 18 位，最后一位可为 X' };
  const y = +id.substr(6, 4), m = +id.substr(10, 2), d = +id.substr(12, 2);
  const dt = new Date(y, m - 1, d);
  if (y < 1900 || dt.getFullYear() !== y || dt.getMonth() !== m - 1 || dt.getDate() !== d || dt > new Date()) {
    return { ok: false, msg: '身份证号中的出生日期不正确' };
  }
  let sum = 0;
  for (let i = 0; i < 17; i++) sum += (+id[i]) * W[i];
  if (CODES[sum % 11] !== id[17]) return { ok: false, msg: '身份证号校验位不正确，请核对最后一位' };
  return { ok: true, msg: '', birthday: y + '-' + pad(m) + '-' + pad(d), gender: (+id[16]) % 2 ? 'M' : 'F' };
}

function maskId(id) { return id ? id.substr(0, 4) + '**********' + id.substr(14) : ''; }
function maskPhone(p) { return p ? p.substr(0, 3) + '****' + p.substr(7) : ''; }

function serial(prefix) {
  const d = new Date();
  return prefix + d.getFullYear() + pad(d.getMonth() + 1) + pad(d.getDate()) + String(Date.now()).slice(-6) + Math.floor(Math.random() * 90 + 10);
}

module.exports = { pad, money, amountText, ymd, ymdhm, addDays, ageOf, checkIdcard, maskId, maskPhone, serial };
