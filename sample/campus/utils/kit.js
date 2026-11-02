// 共用小工具：存储（带前缀）、格式化、校验、以及「要宿主才能做」的能力的统一降级。
// 桌面引擎里 requestPayment / scanCode / getLocation / makePhoneCall / chooseImage 等没有宿主时会立刻走 fail，
// 这里把各自的合理降级收在一处：支付 → 模拟收银台页；扫码 → 返回 null 交给页面弹手动输入；定位 → 默认坐标。
const P = 'campus_';

function get(k, d) {
  const v = wx.getStorageSync(P + k);
  return v === '' || v === undefined || v === null ? d : v;
}
function set(k, v) { wx.setStorageSync(P + k, v); }
function del(k) { wx.removeStorageSync(P + k); }

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function money(n) { return (Math.round((Number(n) || 0) * 100) / 100).toFixed(2); }
function fmt(ts, f) {
  const d = ts instanceof Date ? ts : new Date(ts === undefined ? Date.now() : ts);
  return (f || 'YYYY-MM-DD HH:mm')
    .replace('YYYY', d.getFullYear()).replace('MM', pad(d.getMonth() + 1)).replace('DD', pad(d.getDate()))
    .replace('HH', pad(d.getHours())).replace('mm', pad(d.getMinutes())).replace('ss', pad(d.getSeconds()));
}
function dateStr(offsetDays) { return fmt(Date.now() + (offsetDays || 0) * 86400000, 'YYYY-MM-DD'); }
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
function weekday(dateString) { const p = dateString.split('-'); return WEEK[new Date(+p[0], +p[1] - 1, +p[2]).getDay()]; }
function uid(prefix) { return (prefix || '') + Date.now().toString(36).toUpperCase() + Math.floor(Math.random() * 900 + 100); }
function orderNo() { return fmt(Date.now(), 'YYYYMMDDHHmmss') + Math.floor(Math.random() * 9000 + 1000); }

function toast(title, icon) { wx.showToast({ title, icon: icon || 'none' }); }
function confirm(title, content, opt) {
  return new Promise(resolve => wx.showModal(Object.assign({ title, content, success: r => resolve(!!r.confirm), fail: () => resolve(false) }, opt || {})));
}
// 当前 tab 页高亮 + 角标（自定义 tabBar）
function tab(page, idx, extra) {
  const tb = typeof page.getTabBar === 'function' && page.getTabBar();
  if (tb) tb.setData(Object.assign({ selected: idx }, extra || {}));
}

// 支付：先走 wx.requestPayment，没有宿主（fail）时打开模拟收银台页，用户在那页确认后才算付了
function pay(opt) {
  return new Promise((resolve, reject) => {
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)), nonceStr: uid('N'), package: 'prepay_id=' + uid('wx'),
      signType: 'RSA', paySign: 'demo-sign',
      success: () => resolve({ channel: '微信支付' }),
      fail: () => wx.navigateTo({
        url: '/pages/cashier/cashier?amount=' + money(opt.amount) + '&title=' + encodeURIComponent(opt.title || '订单支付'),
        events: { paid: r => resolve(r), cancel: () => reject({ errMsg: 'requestPayment:fail cancel' }) },
        fail: () => reject({ errMsg: 'requestPayment:fail' })
      })
    });
  });
}

// 扫码：成功给结果；没有宿主时给 null，页面自己弹手动输入
function scan() {
  return new Promise(resolve => wx.scanCode({ success: r => resolve(r.result), fail: () => resolve(null) }));
}

function call(phone) {
  wx.makePhoneCall({
    phoneNumber: phone,
    fail() {
      wx.showModal({ title: '拨打电话', content: phone, confirmText: '复制号码', success(r) { if (r.confirm) wx.setClipboardData({ data: phone }); } });
    }
  });
}

// 定位：失败时用默认坐标（fallback: true），页面据此提示「定位失败，已使用默认位置」
function locate(def) {
  return new Promise(resolve => wx.getLocation({
    type: 'gcj02',
    success: r => resolve({ latitude: r.latitude, longitude: r.longitude, fallback: false }),
    fail: () => resolve(Object.assign({ fallback: true }, def || { latitude: 30.6586, longitude: 104.0648 }))
  }));
}
function distance(a, b) {
  const R = 6371, rad = x => x * Math.PI / 180;
  const dLat = rad(b.latitude - a.latitude), dLng = rad(b.longitude - a.longitude);
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(rad(a.latitude)) * Math.cos(rad(b.latitude)) * Math.sin(dLng / 2) ** 2;
  return 2 * R * Math.asin(Math.sqrt(h));
}
function km(d) { return d < 1 ? Math.round(d * 1000) + 'm' : d.toFixed(1) + 'km'; }

// 选图：chooseMedia → chooseImage → 示例图（samples），总能拿到图
function pickImages(count, samples) {
  return new Promise(resolve => {
    const fallback = () => resolve((samples || []).slice(0, count));
    wx.chooseMedia({
      count, mediaType: ['image'],
      success: r => resolve(r.tempFiles.map(f => f.tempFilePath)),
      fail: () => wx.chooseImage({ count, success: r => resolve(r.tempFilePaths), fail: fallback })
    });
  });
}

// 订阅消息：用户拒绝或没有宿主时返回 false，页面改记本地提醒
function subscribe(tmplId) {
  return new Promise(resolve => wx.requestSubscribeMessage({
    tmplIds: [tmplId], success: r => resolve(r[tmplId] === 'accept'), fail: () => resolve(false)
  }));
}
function addCalendar(title, startTime, desc) {
  return new Promise(resolve => wx.addPhoneCalendar({
    title, startTime: Math.floor(startTime / 1000), description: desc || '', alarm: true, alarmOffset: 3600,
    success: () => resolve(true), fail: () => resolve(false)
  }));
}

const ID_W = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
const ID_C = '10X98765432';
function idValid(s) {
  s = String(s || '').toUpperCase();
  if (!/^\d{17}[\dX]$/.test(s)) return false;
  let sum = 0;
  for (let i = 0; i < 17; i++) sum += Number(s[i]) * ID_W[i];
  return ID_C[sum % 11] === s[17];
}
function phoneValid(s) { return /^1[3-9]\d{9}$/.test(String(s || '')); }
function maskPhone(s) { return String(s).replace(/^(\d{3})\d{4}(\d{4})$/, '$1****$2'); }
function maskId(s) { return String(s).replace(/^(.{4}).*(.{4})$/, '$1**********$2'); }

module.exports = {
  get, set, del, pad, money, fmt, dateStr, weekday, WEEK, uid, orderNo, toast, confirm, tab,
  pay, scan, call, locate, distance, km, pickImages, subscribe, addCalendar, idValid, phoneValid, maskPhone, maskId
};
