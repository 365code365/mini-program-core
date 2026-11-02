// 通用工具：日期、距离、以及需要宿主的能力（定位 / 电话 / 导航）的降级封装。
const { DEFAULT_LOC } = require('./data.js');

const pad = (n) => (n < 10 ? '0' + n : '' + n);
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];

function ymd(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function hm(d) { return pad(d.getHours()) + ':' + pad(d.getMinutes()); }
function stamp(d) { d = d || new Date(); return ymd(d) + ' ' + hm(d); }
function parseYmd(s) { const p = String(s).split('-'); return new Date(+p[0], +p[1] - 1, +p[2]); }
function daysBetween(a, b) { return Math.round((parseYmd(b) - parseYmd(a)) / 86400000); }
function addDays(s, n) { const d = parseYmd(s); d.setDate(d.getDate() + n); return ymd(d); }

// 未来 n 天的日期条：今天 / 明天 / 周X
function nextDays(n) {
  const out = [], now = new Date();
  for (let i = 0; i < n; i++) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + i);
    out.push({ date: ymd(d), week: i === 0 ? '今天' : i === 1 ? '明天' : WEEK[d.getDay()], md: (d.getMonth() + 1) + '月' + d.getDate() + '日' });
  }
  return out;
}

// 两点球面距离（km，保留 1 位）
function distanceKm(a, b) {
  const R = 6371, rad = Math.PI / 180;
  const dLat = (b.latitude - a.latitude) * rad, dLng = (b.longitude - a.longitude) * rad;
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(a.latitude * rad) * Math.cos(b.latitude * rad) * Math.sin(dLng / 2) ** 2;
  return Math.round(2 * R * Math.asin(Math.sqrt(h)) * 10) / 10;
}

// 定位：成功用真实坐标，失败（桌面端 / 用户拒绝）回退武林广场并提示一次
// 几个页面同时要定位时只发一次请求、只提示一次
let cachedLoc = null, waiting = null;
function getLocation(cb) {
  if (cachedLoc) { cb(cachedLoc); return; }
  if (waiting) { waiting.push(cb); return; }
  waiting = [cb];
  const done = (loc) => { cachedLoc = loc; const list = waiting; waiting = null; list.forEach((f) => f(loc)); };
  wx.getLocation({
    type: 'gcj02',
    success: (r) => done({ latitude: r.latitude, longitude: r.longitude, name: '当前位置', ok: true }),
    fail: () => {
      wx.showToast({ title: '定位失败，按杭州市中心计算距离', icon: 'none' });
      done(Object.assign({ ok: false }, DEFAULT_LOC));
    }
  });
}

// 打电话：没有通话能力时弹出号码并可复制
function callPhone(phone, title) {
  wx.makePhoneCall({
    phoneNumber: phone.replace(/-/g, ''),
    fail: () => {
      wx.showModal({
        title: title || '联系电话', content: phone, confirmText: '复制号码', cancelText: '关闭',
        success: (r) => { if (r.confirm) wx.setClipboardData({ data: phone }); }
      });
    }
  });
}

// 导航到门店：打不开地图时给出地址并可复制
function openStoreLocation(store) {
  wx.openLocation({
    latitude: store.latitude, longitude: store.longitude, name: store.name, address: store.address, scale: 16,
    fail: () => {
      wx.showModal({
        title: '无法打开地图', content: store.address, confirmText: '复制地址', cancelText: '关闭',
        success: (r) => { if (r.confirm) wx.setClipboardData({ data: store.address }); }
      });
    }
  });
}

function money(n) { return (Math.round(n * 100) / 100).toFixed(0); }

// 车牌：省份简称 + 字母 + 5 位（燃油）或 6 位（新能源，D/F 开头或结尾）
function checkPlate(p) { return /^[\u4e00-\u9fa5][A-Z][A-HJ-NP-Z0-9]{5,6}$/.test(p); }
// VIN：17 位，不含 I / O / Q
function checkVin(v) { return /^[A-HJ-NPR-Z0-9]{17}$/.test(v); }
function fmtPlate(p) { return p ? p.slice(0, 2) + '·' + p.slice(2) : ''; }

module.exports = {
  pad, ymd, hm, stamp, parseYmd, daysBetween, addDays, nextDays, distanceKm,
  getLocation, callPhone, openStoreLocation, money, checkPlate, checkVin, fmtPlate
};
