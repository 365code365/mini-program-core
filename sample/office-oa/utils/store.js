// 跨页数据：审批单、考勤记录、会议室预订，全部 storage。第一次打开时按「今天」生成一批像样的历史数据。
const O = require('./org.js');
const F = require('./forms.js');

const K_APPR = 'oa_approvals', K_ATT = 'oa_attend', K_BOOK = 'oa_bookings';

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function ymd(d) { return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`; }
function hm(d) { return `${pad(d.getHours())}:${pad(d.getMinutes())}`; }
function now() { const d = new Date(); return ymd(d) + ' ' + hm(d); }
function addDays(n) { const d = new Date(); d.setDate(d.getDate() + n); return ymd(d); }
function read(k) { const v = wx.getStorageSync(k); return v === '' ? null : v; }

// ---------- 审批 ----------
function node(role, who, status, time, comment) { return { role, who, whoName: O.emp(who).name, status, time: time || '', comment: comment || '' }; }
function seedApprovals() {
  const t1 = addDays(-1) + ' 10:12', t2 = addDays(-2) + ' 16:40', t3 = addDays(-6) + ' 09:05';
  return [
    { id: 'AP1001', type: 'leave', applicant: 'e028', values: { kind: '年假', start: addDays(3), startHalf: '上午', end: addDays(4), endHalf: '下午', days: '2', reason: '国庆调休后回老家处理房屋过户手续' },
      nodes: [node('发起申请', 'e028', 'done', t1), node('直属主管', 'e001', 'pending'), node('人力备案', 'e014', 'wait')], status: 'pending', createdAt: t1 },
    { id: 'AP1002', type: 'expense', applicant: 'e015', values: { kind: '市内交通', month: addDays(0).slice(0, 7), amount: '186.50', invoices: '6', reason: '本月 4 次外出到客户现场走查设计稿的打车费' },
      nodes: [node('发起申请', 'e015', 'done', t2), node('直属主管', 'e001', 'pending'), node('财务复核', 'e029', 'wait')], status: 'pending', createdAt: t2 },
    { id: 'AP1003', type: 'leave', applicant: 'e004', values: { kind: '调休', start: addDays(-3), startHalf: '下午', end: addDays(-3), endHalf: '下午', days: '0.5', reason: '上周六版本发布加班调休' },
      nodes: [node('发起申请', 'e004', 'done', t3), node('直属主管', 'e001', 'done', addDays(-6) + ' 09:30', '同意，注意交接'), node('人力备案', 'e014', 'done', addDays(-6) + ' 11:02')], status: 'approved', createdAt: t3 },
    { id: 'AP1004', type: 'expense', applicant: 'e001', values: { kind: '差旅费', month: addDays(-20).slice(0, 7), amount: '2368.00', invoices: '5', reason: '赴深圳参加客户需求评审，高铁往返 + 两晚住宿' },
      nodes: [node('发起申请', 'e001', 'done', t3), node('直属主管', 'e002', 'done', addDays(-5) + ' 14:20', '同意'), node('财务复核', 'e029', 'done', addDays(-4) + ' 10:08', '票据齐全，已安排打款')], status: 'approved', createdAt: t3 },
    { id: 'AP1005', type: 'leave', applicant: 'e001', values: { kind: '事假', start: addDays(6), startHalf: '上午', end: addDays(6), endHalf: '下午', days: '1', reason: '孩子幼儿园开放日，需家长全天陪同' },
      nodes: [node('发起申请', 'e001', 'done', t1), node('直属主管', 'e002', 'pending'), node('人力备案', 'e014', 'wait')], status: 'pending', createdAt: t1 }
  ];
}
function decorate(a) {
  const t = F.types.find(x => x.id === a.type);
  const who = O.emp(a.applicant);
  const cur = a.nodes.find(n => n.status === 'pending');
  return Object.assign({}, a, {
    typeName: t.name, color: t.color, ic: t.ic, applicantName: who.name, applicantIni: who.ini, applicantColor: who.color,
    deptName: who.deptName, summary: F.summary(a.type, a.values), current: cur ? cur.whoName : '',
    needMe: !!cur && cur.who === O.ME, mine: a.applicant === O.ME,
    handled: a.applicant !== O.ME && a.nodes.some(n => n.who === O.ME && (n.status === 'done' || n.status === 'rejected')),
    statusText: { pending: '审批中', approved: '已通过', rejected: '已驳回', withdrawn: '已撤回' }[a.status]
  });
}
function approvals() {
  let list = read(K_APPR);
  if (!list) { list = seedApprovals(); wx.setStorageSync(K_APPR, list); }
  return list.map(decorate);
}
function saveRaw(list) { wx.setStorageSync(K_APPR, list.map(a => ({ id: a.id, type: a.type, applicant: a.applicant, values: a.values, nodes: a.nodes, status: a.status, createdAt: a.createdAt }))); }
function getApproval(id) { return approvals().find(a => a.id === id); }
function addApproval(type, values) {
  const list = approvals();
  const flow = O.FLOW[type].slice();
  flow[0] = values.approver || flow[0];
  const role2 = { leave: '人力备案', expense: '财务复核', seal: '总经办审批' }[type];
  const rec = {
    id: 'AP' + Date.now().toString().slice(-6), type, applicant: O.ME, values, status: 'pending', createdAt: now(),
    nodes: [node('发起申请', O.ME, 'done', now()), node('审批人', flow[0], 'pending'), node(role2, flow[1], 'wait')]
  };
  list.unshift(rec);
  saveRaw(list);
  return rec.id;
}
// 当前节点处理：同意走到下一节点（没有下一节点就通过），驳回整单结束
function act(id, agree, comment, actor) {
  const list = approvals();
  const a = list.find(x => x.id === id);
  if (!a || a.status !== 'pending') return null;
  const i = a.nodes.findIndex(n => n.status === 'pending');
  const n = a.nodes[i];
  if (actor && n.who !== actor) return null;
  n.status = agree ? 'done' : 'rejected';
  n.time = now();
  n.comment = comment || (agree ? '同意' : '');
  if (!agree) a.status = 'rejected';
  else if (a.nodes[i + 1]) a.nodes[i + 1].status = 'pending';
  else a.status = 'approved';
  saveRaw(list);
  return decorate(a);
}
function withdraw(id) {
  const list = approvals();
  const a = list.find(x => x.id === id);
  if (!a || a.status !== 'pending') return;
  a.status = 'withdrawn';
  a.nodes.forEach(n => { if (n.status === 'pending' || n.status === 'wait') n.status = 'cancel'; });
  saveRaw(list);
}

// ---------- 考勤 ----------
function hash(s) { let h = 7; for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 100003; return h; }
function seedAttend() {
  const rec = {};
  const today = new Date();
  for (let k = 45; k >= 1; k--) {
    const d = new Date(today.getFullYear(), today.getMonth(), today.getDate() - k);
    const w = d.getDay();
    if (w === 0 || w === 6) continue;
    const key = ymd(d), h = hash(key);
    if (h % 23 === 0) continue; // 偶尔一天缺卡
    const inMin = 8 * 60 + 38 + (h % 37);            // 08:38 ~ 09:14
    const outMin = 17 * 60 + 52 + (h % 113);         // 17:52 ~ 19:44
    rec[key] = { in: pad(Math.floor(inMin / 60)) + ':' + pad(inMin % 60), out: h % 17 === 0 ? '' : pad(Math.floor(outMin / 60)) + ':' + pad(outMin % 60), place: '云协同大厦 12F' };
  }
  return rec;
}
function attend() {
  let r = read(K_ATT);
  if (!r) { r = seedAttend(); wx.setStorageSync(K_ATT, r); }
  return r;
}
function punch(kind, place) {
  const r = attend();
  const key = ymd(new Date());
  const day = r[key] || { in: '', out: '', place };
  day[kind] = hm(new Date());
  day.place = place;
  r[key] = day;
  wx.setStorageSync(K_ATT, r);
  return day;
}

// ---------- 会议室 ----------
const rooms = [
  { id: 'A101', name: '晨曦', floor: '12F', seats: 6, gear: '电视投屏' },
  { id: 'A203', name: '星河', floor: '12F', seats: 12, gear: '投影 · 白板' },
  { id: 'B305', name: '远山', floor: '13F', seats: 20, gear: '视频会议' },
  { id: 'B102', name: '小憩', floor: '13F', seats: 4, gear: '白板' }
];
// 09:00 ~ 18:00，每格半小时
const SLOTS = Array.from({ length: 18 }, (_, i) => pad(9 + Math.floor(i / 2)) + ':' + (i % 2 ? '30' : '00'));
function slotEnd(i) { return i >= 18 ? '18:00' : SLOTS[i]; }
function seedBookings() {
  const d0 = addDays(0), d1 = addDays(1);
  return [
    { id: 'BK1', room: 'B305', date: d0, start: 1, end: 3, subject: '研发周会', owner: 'e002', withMe: true },
    { id: 'BK2', room: 'A203', date: d0, start: 10, end: 12, subject: '客户需求评审（远程）', owner: 'e013', withMe: true },
    { id: 'BK3', room: 'A101', date: d0, start: 2, end: 4, subject: '新人面试', owner: 'e007' },
    { id: 'BK4', room: 'B102', date: d0, start: 6, end: 8, subject: '一对一沟通', owner: 'e014' },
    { id: 'BK5', room: 'A203', date: d1, start: 0, end: 2, subject: '市场季度复盘', owner: 'e016' },
    { id: 'BK6', room: 'B305', date: d1, start: 8, end: 12, subject: '全员月度会', owner: 'e010', withMe: true }
  ];
}
function bookings() {
  let b = read(K_BOOK);
  if (!b) { b = seedBookings(); wx.setStorageSync(K_BOOK, b); }
  return b.map(x => Object.assign({}, x, {
    ownerName: O.emp(x.owner).name, mine: x.owner === O.ME,
    roomName: (rooms.find(r => r.id === x.room) || {}).name, time: SLOTS[x.start] + '-' + slotEnd(x.end)
  }));
}
function addBooking(b) {
  const list = read(K_BOOK) || seedBookings();
  const clash = list.some(x => x.room === b.room && x.date === b.date && b.start < x.end && x.start < b.end);
  if (clash) return null;
  const rec = Object.assign({ id: 'BK' + Date.now().toString().slice(-6), owner: O.ME }, b);
  list.push(rec);
  wx.setStorageSync(K_BOOK, list);
  return rec;
}
function cancelBooking(id) { wx.setStorageSync(K_BOOK, (read(K_BOOK) || []).filter(x => x.id !== id)); }

function clearAll() { [K_APPR, K_ATT, K_BOOK].forEach(k => wx.removeStorageSync(k)); }

module.exports = {
  pad, ymd, hm, now, addDays, approvals, getApproval, addApproval, act, withdraw,
  attend, punch, rooms, SLOTS, slotEnd, bookings, addBooking, cancelBooking, clearAll
};
