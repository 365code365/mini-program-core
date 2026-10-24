// 金额工具：千分位、大写金额、输入清洗、卡号分组
const DIG = '零壹贰叁肆伍陆柒捌玖';
const UNIT = ['', '拾', '佰', '仟'];
const BIG = ['', '万', '亿'];

function fmt(n) {
  const v = Number(n) || 0;
  const neg = v < 0;
  const s = Math.abs(v).toFixed(2);
  const parts = s.split('.');
  const int = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return (neg ? '-' : '') + int + '.' + parts[1];
}

// 输入框里的千分位：保留用户正在输的小数位（"12345." 显示成 "12,345."）
function fmtTyping(str) {
  if (!str) return '';
  const parts = str.split('.');
  const int = (parts[0] || '0').replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return parts.length > 1 ? int + '.' + parts[1] : int;
}

function upper(n) {
  const fen = Math.round((Number(n) || 0) * 100);
  if (fen <= 0) return '';
  const int = Math.floor(fen / 100);
  const jiao = Math.floor(fen / 10) % 10;
  const f = fen % 10;
  let out = '';
  if (int > 0) {
    const s = String(int);
    let zero = false;
    for (let i = 0; i < s.length; i++) {
      const d = +s[i];
      const pos = s.length - 1 - i;
      const u = pos % 4;
      const b = Math.floor(pos / 4);
      if (d === 0) {
        zero = true;
      } else {
        if (zero) out += '零';
        zero = false;
        out += DIG[d] + UNIT[u];
      }
      if (u === 0 && b > 0 && +s.slice(Math.max(0, i - 3), i + 1) > 0) {
        out += BIG[b];
        zero = false;
      }
    }
    out += '元';
  }
  if (jiao === 0 && f === 0) return out + '整';
  if (jiao > 0) out += DIG[jiao] + '角';
  else if (int > 0) out += '零';
  if (f > 0) out += DIG[f] + '分';
  return out;
}

// 金额输入：只留数字和一个小数点，两位小数，整数部分不超过 9 位，去掉多余前导 0
function cleanAmount(str) {
  let s = String(str || '').replace(/[^\d.]/g, '');
  const dot = s.indexOf('.');
  if (dot >= 0) s = s.slice(0, dot + 1) + s.slice(dot + 1).replace(/\./g, '');
  if (s[0] === '.') s = '0' + s;
  let [i, d] = s.split('.');
  i = i.replace(/^0+(?=\d)/, '').slice(0, 9);
  if (d !== undefined) return i + '.' + d.slice(0, 2);
  return i;
}

// 卡号：只留数字，最多 19 位，每 4 位一个空格
function formatCard(str) {
  const d = String(str || '').replace(/\D/g, '').slice(0, 19);
  return d.replace(/(\d{4})(?=\d)/g, '$1 ');
}

function maskCard(no) {
  const d = String(no).replace(/\D/g, '');
  return d.slice(0, 4) + ' **** **** ' + d.slice(-4);
}

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function dateStr(t) {
  const d = new Date(t);
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
}
function timeStr(t) {
  const d = new Date(t);
  return dateStr(t) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes());
}

module.exports = { fmt, fmtTyping, upper, cleanAmount, formatCard, maskCard, dateStr, timeStr, pad };
