// 就诊人与挂号单：存在本地 storage，重启后仍在
const DEFAULT_PATIENTS = [
  { id: 'p1', name: '张明远', relation: '本人', gender: '男', idcard: '310104********2817', phone: '138****6621', card: '0012 8837 6612', isDefault: true },
  { id: 'p2', name: '张小雨', relation: '子女', gender: '女', idcard: '310104********0426', phone: '138****6621', card: '0012 9901 2205', isDefault: false }
];

function dayAfter(n) {
  const d = new Date();
  const x = new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
  const p = v => (v < 10 ? '0' + v : '' + v);
  return x.getFullYear() + '-' + p(x.getMonth() + 1) + '-' + p(x.getDate());
}

const DEFAULT_ORDERS = [
  { id: 'R20241026105', doctor: '林佳', title: '副主任医师', dept: '眼科', date: dayAfter(2), time: '14:30-15:30', patient: '张小雨', fee: 30, status: 'wait', no: 17, room: '门诊 2 楼 A 区 205' },
  { id: 'R20241015083', doctor: '孙建国', title: '主任医师', dept: '呼吸内科', date: '2024-10-15', time: '09:00-10:00', patient: '张明远', fee: 50, status: 'done', no: 12, room: '门诊 3 楼 B 区 307' }
];

function getPatients() {
  const v = wx.getStorageSync('hs_patients');
  return Array.isArray(v) && v.length ? v : DEFAULT_PATIENTS.slice();
}
function savePatients(list) { wx.setStorageSync('hs_patients', list); }

function getOrders() {
  const v = wx.getStorageSync('hs_orders');
  return Array.isArray(v) ? v : DEFAULT_ORDERS.slice();
}
function saveOrders(list) { wx.setStorageSync('hs_orders', list); }

module.exports = { getPatients, savePatients, getOrders, saveOrders };
