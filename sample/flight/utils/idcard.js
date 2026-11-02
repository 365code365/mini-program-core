// 证件校验：二代身份证（18 位，GB 11643 校验位）、护照、港澳 / 台湾通行证、外国人永久居留证
const TYPES = ['身份证', '护照', '港澳居民来往内地通行证', '台湾居民来往大陆通行证', '外国人永久居留身份证'];
const W = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
const M = '10X98765432';

function checkId(no) {
  if (!/^\d{17}[\dX]$/.test(no)) return '身份证号应为 18 位，最后一位可以是 X';
  const y = +no.slice(6, 10), m = +no.slice(10, 12), d = +no.slice(12, 14);
  const dt = new Date(y, m - 1, d);
  if (y < 1900 || dt.getFullYear() !== y || dt.getMonth() !== m - 1 || dt.getDate() !== d || dt > new Date()) return '身份证号里的出生日期不对';
  let s = 0;
  for (let i = 0; i < 17; i++) s += +no[i] * W[i];
  if (M[s % 11] !== no[17]) return '身份证号校验位不对，请核对';
  return '';
}

function check(type, raw) {
  const no = String(raw || '').trim().toUpperCase();
  if (!no) return '请填写证件号码';
  if (type === 0) return checkId(no);
  if (type === 1) return /^[EGDSP]\d{8}$|^E[A-HJ-NP-Z]\d{7}$/.test(no) ? '' : '护照号格式不对（如 E12345678）';
  if (type === 2) return /^[CHM]\d{8}$/.test(no) ? '' : '港澳通行证号为 C/H/M 加 8 位数字';
  if (type === 3) return /^\d{8}$/.test(no) ? '' : '台湾居民来往大陆通行证号为 8 位数字';
  return /^[A-Z]{3}\d{12}$|^9\d{17}$/.test(no) ? '' : '永久居留身份证号格式不对';
}

function checkName(name) {
  const n = String(name || '').trim();
  if (!n) return '请填写乘机人姓名';
  if (/^[\u4e00-\u9fa5·]{2,15}$/.test(n)) return '';
  if (/^[A-Za-z]+\/[A-Za-z ]+$/.test(n)) return '';
  return '姓名需与证件一致：中文姓名，或英文「姓/名」如 ZHANG/SAN';
}

// 身份证号推出生日期与年龄段（成人 ≥12 岁、儿童 2~12 岁、婴儿 <2 岁）
function ageType(type, no) {
  if (type !== 0 || !no || no.length !== 18) return '成人';
  const b = new Date(+no.slice(6, 10), +no.slice(10, 12) - 1, +no.slice(12, 14));
  const age = (Date.now() - b.getTime()) / (365.25 * 86400000);
  return age >= 12 ? '成人' : age >= 2 ? '儿童' : '婴儿';
}

function mask(no) {
  const s = String(no || '');
  return s.length <= 6 ? s : s.slice(0, 3) + '*'.repeat(s.length - 7) + s.slice(-4);
}

function checkPhone(p) { return /^1[3-9]\d{9}$/.test(String(p || '')) ? '' : '请填写 11 位手机号'; }

module.exports = { TYPES, check, checkName, ageType, mask, checkPhone };
