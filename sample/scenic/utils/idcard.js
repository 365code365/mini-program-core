// 18 位居民身份证校验（GB 11643：出生日期 + ISO 7064 MOD 11-2 校验位），并按出生日期算周岁
const W = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
const C = '10X98765432';

function check(id) {
  id = String(id || '').trim().toUpperCase();
  if (!/^\d{17}[\dX]$/.test(id)) return { ok: false, msg: '请输入 18 位身份证号' };
  const y = +id.slice(6, 10), m = +id.slice(10, 12), d = +id.slice(12, 14);
  const dt = new Date(y, m - 1, d);
  if (y < 1900 || dt.getFullYear() !== y || dt.getMonth() !== m - 1 || dt.getDate() !== d || dt > new Date()) {
    return { ok: false, msg: '身份证号里的出生日期不对' };
  }
  let s = 0;
  for (let i = 0; i < 17; i++) s += (+id[i]) * W[i];
  if (C[s % 11] !== id[17]) return { ok: false, msg: '身份证号校验位不对，请核对' };
  const now = new Date();
  let age = now.getFullYear() - y;
  if (now.getMonth() + 1 < m || (now.getMonth() + 1 === m && now.getDate() < d)) age--;
  return { ok: true, id, age, male: (+id[16]) % 2 === 1 };
}
// 页面展示用：保留前 3 后 4 位
function mask(id) { return id ? id.slice(0, 3) + '***********' + id.slice(-4) : ''; }
// 生成一个校验位正确的号码（演示数据用）
function withCheck(p17) {
  let s = 0;
  for (let i = 0; i < 17; i++) s += (+p17[i]) * W[i];
  return p17 + C[s % 11];
}

module.exports = { check, mask, withCheck };
