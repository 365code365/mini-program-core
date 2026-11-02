// 定位与距离。wx.getLocation 在桌面引擎里立刻 fail，退回默认位置（并告诉页面是回退来的）。
const { DEFAULT_LOC } = require('./data.js');

const R = 6371008.8; // 地球平均半径（米）
function rad(d) { return (d * Math.PI) / 180; }

// haversine 球面距离，返回米
function distance(lat1, lng1, lat2, lng2) {
  const dLat = rad(lat2 - lat1), dLng = rad(lng2 - lng1);
  const a = Math.sin(dLat / 2) ** 2 + Math.cos(rad(lat1)) * Math.cos(rad(lat2)) * Math.sin(dLng / 2) ** 2;
  return 2 * R * Math.asin(Math.min(1, Math.sqrt(a)));
}

function fmtDistance(m) { return m < 1000 ? Math.round(m) + 'm' : (m / 1000).toFixed(1) + 'km'; }

function locate(cb) {
  wx.getLocation({
    type: 'gcj02',
    success: (r) => cb({ latitude: r.latitude, longitude: r.longitude, name: '当前位置', fallback: false }),
    fail: (err) => cb(Object.assign({}, DEFAULT_LOC, { fallback: true, reason: (err && err.errMsg) || 'getLocation:fail' }))
  });
}

module.exports = { distance, fmtDistance, locate };
