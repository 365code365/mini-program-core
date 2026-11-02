// 跨页数据：办件、预约走 wx.setStorageSync（退出重进还在）；申报草稿走文件系统（draft.json），见 draft.js。
const { getItem } = require('./items');

const DAY = 86400000;
const STATUS = {
  submitted: { text: '待受理', color: '#1d64d8' },
  reviewing: { text: '审核中', color: '#ea580c' },
  supplement: { text: '待补正', color: '#dc2626' },
  done: { text: '已办结', color: '#16a34a' },
  withdrawn: { text: '已撤回', color: '#8a94a6' }
};

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function fmtTime(ts) {
  const d = new Date(ts);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
function fmtDate(ts) {
  const d = new Date(ts);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
function maskId(id) { return id ? id.slice(0, 4) + '**********' + id.slice(-4) : ''; }
function maskName(n) { return n ? n[0] + '*'.repeat(Math.max(1, n.length - 1)) : ''; }

// 18 位公民身份号码校验（GB 11643：前 17 位加权求和 mod 11 对应校验码）
function checkIdcard(v) {
  const s = String(v || '').toUpperCase();
  if (!/^\d{17}[\dX]$/.test(s)) return '身份证号应为 18 位，末位可以是 X';
  const y = +s.slice(6, 10), m = +s.slice(10, 12), d = +s.slice(12, 14);
  const dt = new Date(y, m - 1, d);
  if (y < 1900 || dt.getMonth() !== m - 1 || dt.getDate() !== d || dt.getTime() > Date.now()) return '身份证号中的出生日期不正确';
  const W = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
  let sum = 0;
  for (let i = 0; i < 17; i++) sum += +s[i] * W[i];
  if ('10X98765432'[sum % 11] !== s[17]) return '身份证号校验位不正确，请核对';
  return '';
}

const USER = { name: '张明远', idcard: '530102199208163216', phone: '13987651234', level: '实名认证（L3 级）' };

function seedOrders() {
  const now = Date.now();
  const t1 = now - 12 * DAY, t2 = now - 3 * DAY;
  return [
    {
      id: 'YC' + fmtDate(t2).replace(/-/g, '') + '00418', itemId: 'G0102', name: '居住证办理', status: 'supplement', createdAt: t2,
      applicant: { name: USER.name, idcard: USER.idcard, phone: USER.phone }, way: '窗口领取',
      note: '房屋租赁合同缺少出租方签字页，请补充上传完整扫描件。',
      timeline: [
        { t: t2, title: '网上申请', desc: '申请已提交，材料 2 份' },
        { t: t2 + 2 * 3600000, title: '窗口受理', desc: '滨江区江湾派出所 已受理' },
        { t: t2 + DAY, title: '退回补正', desc: '房屋租赁合同缺少出租方签字页，请补充上传完整扫描件。' }
      ]
    },
    {
      id: 'YC' + fmtDate(t1).replace(/-/g, '') + '00137', itemId: 'G0301', name: '住房公积金提取（租房）', status: 'done', createdAt: t1,
      applicant: { name: USER.name, idcard: USER.idcard, phone: USER.phone }, way: '线上办结',
      timeline: [
        { t: t1, title: '网上申请', desc: '申请提取 15000.00 元' },
        { t: t1 + 600000, title: '系统核验', desc: '缴存状态正常，租赁备案核验通过' },
        { t: t1 + DAY, title: '资金到账', desc: '15000.00 元已转入尾号 6621 的银行卡' }
      ]
    }
  ];
}

function getOrders() {
  let list = wx.getStorageSync('gov_orders');
  if (!Array.isArray(list)) { list = seedOrders(); wx.setStorageSync('gov_orders', list); }
  return list;
}
function saveOrders(list) { wx.setStorageSync('gov_orders', list); }
function getOrder(id) { return getOrders().find(o => o.id === id); }
function updateOrder(id, fn) {
  const list = getOrders();
  const o = list.find(x => x.id === id);
  if (o) { fn(o); saveOrders(list); }
  return o;
}

function addOrder(form) {
  const item = getItem(form.itemId);
  const now = Date.now();
  const list = getOrders();
  const seq = pad(list.length + 1) + String(now % 1000).padStart(3, '0');
  const o = {
    id: 'YC' + fmtDate(now).replace(/-/g, '') + seq, itemId: item.id, name: item.name, status: 'submitted', createdAt: now,
    applicant: { name: form.name, idcard: form.idcard, phone: form.phone, region: form.region, address: form.address },
    reason: form.reason, way: form.way, remark: form.remark, files: form.files,
    timeline: [{ t: now, title: '网上申请', desc: `申请已提交，材料 ${Object.keys(form.files || {}).length} 份，等待${item.dept}受理` }]
  };
  list.unshift(o);
  saveOrders(list);
  return o;
}

function pendingCount() { return getOrders().filter(o => o.status === 'supplement').length; }

function getAppoints() { const l = wx.getStorageSync('gov_appoints'); return Array.isArray(l) ? l : []; }
function saveAppoints(l) { wx.setStorageSync('gov_appoints', l); }

// 每个 tab 页 onShow：选中态 + 「我的办件」待补正角标
function syncTab(page, n) {
  const tb = page.getTabBar && page.getTabBar();
  if (tb) tb.setData({ selected: n, badge: pendingCount() });
}

module.exports = {
  STATUS, USER, DAY, fmtTime, fmtDate, maskId, maskName, checkIdcard,
  getOrders, getOrder, updateOrder, addOrder, pendingCount, getAppoints, saveAppoints, syncTab
};
