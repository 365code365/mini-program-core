// 就诊人、挂号单、缴费、检查报告、在线问诊、住院：全部存在本地 storage，重启后仍在。
// 状态随时间推进（签到 → 候诊 → 就诊 → 完成后开处方；问诊 1 分钟后医生回复）：页面 onShow 先 tick() 再读
const data = require('./data.js');
const clinic = require('./clinic.js');
const K = { patients: 'hs_patients', orders: 'hs_orders', follow: 'hs_follow', bills: 'hs_bills', reports: 'hs_reports', consults: 'hs_consults', inpatient: 'hs_inpatient' };
const MIN = 60000;
const DAY = 24 * 3600 * 1000;
const AHEAD = 3; // 签到时前面排着的人数，每分钟叫一位
const VISIT = 3; // 叫到号后看诊约 3 分钟
const MAX_PATIENTS = 5;

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function round2(n) { return Math.round(n * 100) / 100; }
function dateStr(ts) { const d = new Date(ts); return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function hm(ts) { const d = new Date(ts); return pad(d.getHours()) + ':' + pad(d.getMinutes()); }
function dateTime(ts) { return dateStr(ts) + ' ' + hm(ts); }
function dayStart(n) { const d = new Date(); return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n).getTime(); }
function dayAfter(n) { return dateStr(dayStart(n)); }
function today() { return dayAfter(0); }
function parseDate(s) { const p = s.split('-').map(Number); return new Date(p[0], p[1] - 1, p[2]).getTime(); }
// storage 里没有这一项时 getStorageSync 给 ''：列表一律判 Array.isArray，不用 `|| 默认值`
function list(key, def) { const v = wx.getStorageSync(key); return Array.isArray(v) ? v : def(); }
let seq = 0;
function uid(prefix) { seq += 1; return prefix + Date.now().toString(36).toUpperCase() + seq; }

// ───────── 就诊人 ─────────
const DEFAULT_PATIENTS = [
  { id: 'p1', name: '张明远', relation: '本人', gender: '男', birth: '1986-03-12', idcard: '310104********2817', phone: '138****6621', card: '0012 8837 6612', isDefault: true },
  { id: 'p2', name: '张小雨', relation: '子女', gender: '女', birth: '2016-09-04', idcard: '310104********0426', phone: '138****6621', card: '0012 9901 2205', isDefault: false },
  { id: 'p3', name: '李秀兰', relation: '父母', gender: '女', birth: '1957-11-20', idcard: '310104********1625', phone: '139****0937', card: '0011 2046 7318', isDefault: false }
];
function getPatients() {
  const v = wx.getStorageSync(K.patients);
  return Array.isArray(v) && v.length ? v : DEFAULT_PATIENTS.map(p => Object.assign({}, p));
}
function savePatients(l) { wx.setStorageSync(K.patients, l); }
function findPatient(id) { return getPatients().find(p => p.id === id) || null; }
function defaultPatient() { const l = getPatients(); return l.find(p => p.isDefault) || l[0]; }
// 新增 / 编辑：返回错误文案（'' 表示成功）
function upsertPatient(p) {
  let l = getPatients();
  const i = l.findIndex(x => x.id === p.id);
  if (i < 0 && l.length >= MAX_PATIENTS) return '最多添加 ' + MAX_PATIENTS + ' 位就诊人';
  if (p.relation === '本人' && l.some(x => x.relation === '本人' && x.id !== p.id)) return '已有关系为「本人」的就诊人';
  if (i < 0 && l.some(x => x.idcard === p.idcard)) return '该身份证号已添加';
  if (p.isDefault) l = l.map(x => Object.assign({}, x, { isDefault: false }));
  if (i >= 0) l[i] = Object.assign({}, l[i], p);
  else l.push(p);
  if (!l.some(x => x.isDefault)) l[0].isDefault = true;
  savePatients(l);
  return '';
}
function setDefaultPatient(id) { savePatients(getPatients().map(p => Object.assign({}, p, { isDefault: p.id === id }))); }
function removePatient(id) {
  const l = getPatients();
  if (l.length <= 1) return '至少保留一位就诊人';
  const p = l.find(x => x.id === id);
  if (getOrders().some(o => o.patientId === id && (o.status === 'wait' || o.status === 'checked'))) return p.name + ' 还有待就诊的挂号，取消后再删除';
  const ip = getInpatient();
  if (ip && ip.status === 'in' && ip.patientId === id) return p.name + ' 正在住院，出院结算后再删除';
  const rest = l.filter(x => x.id !== id);
  if (!rest.some(x => x.isDefault)) rest[0].isDefault = true;
  savePatients(rest);
  return '';
}

// ───────── 挂号单 ─────────
// status：wait 待就诊 / checked 已签到候诊 / done 已完成 / cancel 已取消 / miss 过期未就诊
function defaultOrders() {
  return [
    { id: 'R' + dayAfter(-1).replace(/-/g, '') + '105', doctorId: 'd9', doctor: '林佳', title: '副主任医师', deptId: 'yk', dept: '眼科', date: dayAfter(2), time: '14:30-15:30',
      patientId: 'p2', patient: '张小雨', fee: 30, status: 'wait', no: 17, room: '门诊 2 楼 A 区 205', payType: '上海医保', createdAt: dayStart(-1) + 20 * 3600 * 1000 },
    { id: 'R' + dayAfter(-7).replace(/-/g, '') + '083', doctorId: 'd5', doctor: '孙建国', title: '主任医师', deptId: 'hx', dept: '呼吸内科', date: dayAfter(-5), time: '09:00-10:00',
      patientId: 'p1', patient: '张明远', fee: 50, status: 'done', no: 12, room: '门诊 3 楼 B 区 307', payType: '上海医保', billed: true,
      createdAt: dayStart(-7) + 9 * 3600 * 1000, checkedAt: dayStart(-5) + 9 * 3600 * 1000, doneAt: dayStart(-5) + 9.6 * 3600 * 1000 }
  ];
}
function getOrders() { return list(K.orders, defaultOrders); }
function saveOrders(l) { wx.setStorageSync(K.orders, l); }
function findOrder(id) { return getOrders().find(o => o.id === id) || null; }
function updateOrder(id, patch) {
  const l = getOrders();
  const o = l.find(x => x.id === id);
  if (!o) return null;
  Object.assign(o, patch);
  saveOrders(l);
  return o;
}
function isActive(o) { return o.status === 'wait' || o.status === 'checked'; }
// 排班减去自己已挂的号；今天已经结束的时段标成 -2（已过）。orders 可由调用方先读好（列表页一次算几十位医生）
function slotsFor(doc, dayIdx, orders) {
  const date = dayAfter(dayIdx);
  const mine = (orders || getOrders()).filter(o => o.doctorId === doc.id && o.date === date && isActive(o));
  const t = new Date();
  const nowMin = t.getHours() * 60 + t.getMinutes();
  return data.slotsFor(doc, dayIdx).map(s => {
    let left = s.left > 0 ? Math.max(0, s.left - mine.filter(o => o.time === s.time).length) : s.left;
    const end = s.time.split('-')[1].split(':').map(Number);
    if (dayIdx === 0 && left >= 0 && end[0] * 60 + end[1] <= nowMin) left = -2;
    return { time: s.time, half: s.half, left };
  });
}
function leftOf(doc, days, orders) {
  const l = orders || getOrders();
  let n = 0;
  for (let i = 0; i < (days || 7); i++) slotsFor(doc, i, l).forEach(s => { if (s.left > 0) n += s.left; });
  return n;
}
// 挂号前的校验：返回错误文案
function canBook(q) {
  const day = Math.round((parseDate(q.date) - dayStart(0)) / DAY);
  const slot = slotsFor(data.findDoctor(q.doctorId), day).find(s => s.time === q.time);
  if (!slot || slot.left <= 0) return slot && slot.left === -2 ? '该时段已结束，请重新选择' : '该时段已约满，请重新选择';
  const dup = getOrders().find(o => isActive(o) && o.patientId === q.patientId && o.deptId === q.deptId && o.date === q.date);
  if (dup) return q.patientName + ' 当天已挂' + dup.dept + '的号，同一科室同一天限挂 1 个';
  return '';
}
function createOrder(q) {
  const l = getOrders();
  const dept = data.findDept(q.doc.dept);
  const o = {
    id: 'R' + q.date.replace(/-/g, '') + (100 + l.length) + Date.now().toString().slice(-3),
    doctorId: q.doc.id, doctor: q.doc.name, title: q.doc.title, deptId: q.doc.dept, dept: dept.name,
    date: q.date, time: q.time, patientId: q.patient.id, patient: q.patient.name, fee: q.doc.fee,
    status: 'wait', no: 10 + (l.length * 7) % 30, room: dept.room, payType: q.payType, desc: q.desc || '',
    createdAt: Date.now()
  };
  l.unshift(o);
  saveOrders(l);
  addBill({ kind: 'reg', title: '挂号费 · ' + o.dept + ' ' + o.doctor, patientId: o.patientId, patient: o.patient, orderId: o.id,
    items: [{ name: o.title + '挂号费', cat: '挂号', price: o.fee, qty: 1 }], total: o.fee, insured: 0, self: o.fee, status: 'paid', paidAt: Date.now() });
  return o;
}
// 免费退号截止：就诊前一天 17:00
function cancelDeadline(o) { return parseDate(o.date) - 7 * 3600 * 1000; }
function cancelOrder(id) {
  const o = findOrder(id);
  if (!o || o.status !== 'wait' || Date.now() >= cancelDeadline(o)) return null;
  const bill = getBills().find(b => b.orderId === id && b.kind === 'reg' && b.status === 'paid');
  if (bill) updateBill(bill.id, { status: 'refund', refundAt: Date.now() });
  return updateOrder(id, { status: 'cancel', cancelledAt: Date.now(), refund: o.fee });
}
function checkin(id) { return updateOrder(id, { status: 'checked', checkedAt: Date.now() }); }
function queueOf(o, now) {
  const passed = Math.floor(((now || Date.now()) - o.checkedAt) / MIN);
  return { ahead: Math.max(0, AHEAD - passed) };
}
function stageOf(o, now) {
  now = now || Date.now();
  const names = ['预约成功', '到院签到', '候诊叫号', '就诊完成'];
  const base = { names, canCancel: false, canCheckin: false, ahead: 0 };
  if (o.status === 'cancel') return Object.assign(base, { stage: -1, status: '已取消', tip: o.refund ? '已取消，挂号费 ¥' + o.refund + ' 已原路退回' : '预约已取消' });
  if (o.status === 'miss') return Object.assign(base, { stage: -1, status: '已过期', tip: '就诊日未签到，号源已作废' });
  if (o.status === 'done') {
    const unpaid = getBills().some(b => b.orderId === o.id && b.kind === 'clinic' && b.status === 'unpaid');
    return Object.assign(base, { stage: 3, status: '已完成', unpaid, tip: unpaid ? '医生已开具检查与药品，请到「门诊缴费」支付' : '就诊已完成，祝您早日康复' });
  }
  if (o.status === 'checked') {
    const q = queueOf(o, now);
    return q.ahead > 0
      ? Object.assign(base, { stage: 1, status: '候诊中', ahead: q.ahead, tip: '前面还有 ' + q.ahead + ' 人，请在候诊区留意叫号' })
      : Object.assign(base, { stage: 2, status: '就诊中', tip: '已叫到您的号，请到 ' + o.room + ' 诊室就诊' });
  }
  const isToday = o.date === today();
  return Object.assign(base, {
    stage: 0, status: '待就诊', canCancel: now < cancelDeadline(o), canCheckin: isToday,
    tip: isToday ? '请到院后签到，签到后按号序候诊' : '请于 ' + o.date.slice(5) + ' ' + o.time.slice(0, 5) + ' 前到 ' + o.room + ' 候诊区签到'
  });
}

// ───────── 缴费（挂号费 / 门诊处方 / 问诊费 / 住院预交金 / 出院结算） ─────────
// status：unpaid 待缴 / paid 已缴 / refund 已退
function defaultBills() {
  const done = defaultOrders()[1];
  const items = clinic.billItems('hx', 'nk');
  const total = round2(items.reduce((n, x) => n + x.price * x.qty, 0));
  const ip = defaultInpatient();
  return [
    { id: 'BD1', kind: 'reg', title: '挂号费 · 眼科 林佳', patientId: 'p2', patient: '张小雨', orderId: defaultOrders()[0].id, items: [{ name: '副主任医师挂号费', cat: '挂号', price: 30, qty: 1 }], total: 30, insured: 0, self: 30, status: 'paid', paidAt: dayStart(-1) + 20 * 3600 * 1000 },
    { id: 'BD2', kind: 'clinic', title: '门诊处方 · 呼吸内科 孙建国', patientId: 'p1', patient: '张明远', orderId: done.id, items, total, insured: round2(total * 0.7), self: round2(total - round2(total * 0.7)), status: 'paid', paidAt: done.doneAt + 10 * MIN },
    { id: 'BD3', kind: 'reg', title: '挂号费 · 呼吸内科 孙建国', patientId: 'p1', patient: '张明远', orderId: done.id, items: [{ name: '主任医师挂号费', cat: '挂号', price: 50, qty: 1 }], total: 50, insured: 0, self: 50, status: 'paid', paidAt: done.createdAt }
  ].concat(ip.deposits.map((d, i) => ({ id: 'BDP' + i, kind: 'deposit', title: '住院预交金 · ' + ip.patient, patientId: ip.patientId, patient: ip.patient, total: d.amount, insured: 0, self: d.amount, status: 'paid', paidAt: d.at })));
}
function getBills() { return list(K.bills, defaultBills).slice().sort((a, b) => (b.paidAt || b.createdAt || 0) - (a.paidAt || a.createdAt || 0)); }
function saveBills(l) { wx.setStorageSync(K.bills, l); }
function findBill(id) { return getBills().find(b => b.id === id) || null; }
function addBill(b) {
  const l = getBills();
  const bill = Object.assign({ id: uid('B'), createdAt: Date.now(), insured: 0 }, b);
  l.unshift(bill);
  saveBills(l);
  return bill;
}
function updateBill(id, patch) {
  const l = getBills();
  const b = l.find(x => x.id === id);
  if (!b) return null;
  Object.assign(b, patch);
  saveBills(l);
  return b;
}
// 医保（上海 / 异地）门诊统筹报销 70%，自费与商业保险全额自付
function insuredPart(total, payType) { return payType === '上海医保' || payType === '异地医保' ? round2(total * 0.7) : 0; }
function addClinicBill(o) {
  const items = clinic.billItems(o.deptId, data.groupOfDept(o.deptId).id);
  const total = round2(items.reduce((n, x) => n + x.price * x.qty, 0));
  const insured = insuredPart(total, o.payType);
  return addBill({ kind: 'clinic', title: '门诊处方 · ' + o.dept + ' ' + o.doctor, patientId: o.patientId, patient: o.patient, orderId: o.id, items, total, insured, self: round2(total - insured), status: 'unpaid' });
}
// 缴费成功：检验 / 检查项目出报告
function payBill(id) {
  const b = updateBill(id, { status: 'paid', paidAt: Date.now() });
  if (!b) return null;
  const o = b.orderId ? findOrder(b.orderId) : null;
  (b.items || []).filter(x => x.report).forEach(x => addReport({ key: x.report, patientId: b.patientId, patient: b.patient, dept: o ? o.dept : '', doctor: o ? o.doctor : '', billId: b.id }));
  return b;
}
function unpaidBills() { return getBills().filter(b => b.status === 'unpaid'); }

// ───────── 检查报告 ─────────
function defaultReports() {
  const done = defaultOrders()[1];
  return ['cbc', 'ct'].map((key, i) => ({ id: 'RP' + (i + 1), key, patientId: 'p1', patient: '张明远', dept: done.dept, doctor: done.doctor, date: done.date, read: false }));
}
function getReports() { return list(K.reports, defaultReports); }
function addReport(r) {
  const l = getReports();
  l.unshift(Object.assign({ id: uid('RP'), date: today(), read: false }, r));
  wx.setStorageSync(K.reports, l);
}
function markReportRead(id) { wx.setStorageSync(K.reports, getReports().map(r => (r.id === id ? Object.assign({}, r, { read: true }) : r))); }
function unreadReports() { return getReports().filter(r => !r.read).length; }

// ───────── 在线问诊 ─────────
// status：wait 待接诊 / reply 医生已回复 / done 已结束 / cancel 已取消（问诊费原路退回）
function getConsults() { return list(K.consults, () => []); }
function saveConsults(l) { wx.setStorageSync(K.consults, l); }
function addConsult(c) {
  const l = getConsults();
  const one = Object.assign({ id: uid('C'), createdAt: Date.now(), status: 'wait' }, c);
  l.unshift(one);
  saveConsults(l);
  addBill({ kind: 'consult', title: '在线问诊 · ' + one.typeName, patientId: one.patientId, patient: one.patient, consultId: one.id,
    items: [{ name: one.typeName + ' · ' + one.dept, cat: '问诊', price: one.price, qty: 1 }], total: one.price, self: one.price, status: 'paid', paidAt: Date.now() });
  return one;
}
function updateConsult(id, patch) {
  const l = getConsults();
  const c = l.find(x => x.id === id);
  if (c) Object.assign(c, patch);
  saveConsults(l);
  return c;
}
function cancelConsult(id) {
  const c = getConsults().find(x => x.id === id);
  if (!c || c.status !== 'wait') return null;
  const b = getBills().find(x => x.consultId === id && x.status === 'paid');
  if (b) updateBill(b.id, { status: 'refund', refundAt: Date.now() });
  return updateConsult(id, { status: 'cancel', cancelledAt: Date.now() });
}

// ───────── 住院 ─────────
function defaultInpatient() {
  const inAt = dayStart(-4);
  return {
    id: 'ZY' + dayAfter(-4).replace(/-/g, '') + '0617', patientId: 'p3', patient: '李秀兰', dept: '心血管内科', doctor: '陈立华',
    ward: '住院部 12 楼东区', bed: '12-05 床', inDate: dayAfter(-4), status: 'in', outDate: '',
    deposits: [{ amount: 3000, at: inAt + 10.2 * 3600 * 1000 }, { amount: 500, at: inAt + 2 * DAY + 9.5 * 3600 * 1000 }],
    settle: null
  };
}
function getInpatient() {
  const v = wx.getStorageSync(K.inpatient);
  return v && v.id ? v : defaultInpatient();
}
function saveInpatient(v) { wx.setStorageSync(K.inpatient, v); }
// 每日费用清单：入院当天到今天（已出院到出院日）
function inpatientDays(ip) {
  const end = ip.status === 'out' ? parseDate(ip.outDate) : dayStart(0);
  const n = Math.max(1, Math.round((end - parseDate(ip.inDate)) / DAY) + 1);
  const out = [];
  for (let k = 0; k < n; k++) {
    const items = clinic.dailyItems(k);
    out.push({ k, date: dateStr(parseDate(ip.inDate) + k * DAY), items, total: round2(items.reduce((s, x) => s + x.amount, 0)) });
  }
  return out.reverse();
}
function inpatientSummary(ip) {
  const days = inpatientDays(ip);
  const total = round2(days.reduce((s, d) => s + d.total, 0));
  const deposit = round2(ip.deposits.reduce((s, d) => s + d.amount, 0));
  return { days: days.length, total, deposit, balance: round2(deposit - total) };
}
function addDeposit(amount) {
  const ip = getInpatient();
  ip.deposits.push({ amount, at: Date.now() });
  saveInpatient(ip);
  addBill({ kind: 'deposit', title: '住院预交金 · ' + ip.patient, patientId: ip.patientId, patient: ip.patient, total: amount, self: amount, status: 'paid', paidAt: Date.now() });
  return ip;
}
// 出院结算：预交金多退少补（补缴的部分由页面先付款）
function settleInpatient(extraPaid) {
  const ip = getInpatient();
  const s = inpatientSummary(ip);
  const refund = s.balance > 0 ? s.balance : 0;
  ip.status = 'out';
  ip.outDate = today();
  ip.settle = { total: s.total, deposit: s.deposit, refund, extra: extraPaid || 0, at: Date.now() };
  saveInpatient(ip);
  if (extraPaid) addBill({ kind: 'settle', title: '出院结算补缴 · ' + ip.patient, patientId: ip.patientId, patient: ip.patient, total: extraPaid, self: extraPaid, status: 'paid', paidAt: Date.now() });
  if (refund) addBill({ kind: 'settle', title: '出院结算退预交金 · ' + ip.patient, patientId: ip.patientId, patient: ip.patient, total: refund, self: refund, status: 'refund', paidAt: Date.now(), refundAt: Date.now() });
  return ip;
}

// ───────── 时间推进 ─────────
function tick(now) {
  now = now || Date.now();
  const orders = getOrders();
  let changed = false;
  const born = [];
  orders.forEach(o => {
    if (o.status === 'checked' && now >= o.checkedAt + (AHEAD + VISIT) * MIN) {
      o.status = 'done';
      o.doneAt = o.checkedAt + (AHEAD + VISIT) * MIN;
      changed = true;
      if (!o.billed) { o.billed = true; born.push(o); }
    } else if (o.status === 'wait' && parseDate(o.date) < dayStart(0)) {
      o.status = 'miss';
      changed = true;
    }
  });
  if (changed) saveOrders(orders);
  born.forEach(addClinicBill);
  const cs = getConsults();
  let cc = false;
  cs.forEach(c => {
    if (c.status === 'wait' && now >= c.createdAt + MIN) {
      c.status = 'reply';
      c.replyAt = c.createdAt + MIN;
      c.reply = clinic.consultReply(c.type, c.groupId);
      cc = true;
      if (c.type === 'renew') {
        const items = clinic.RENEW_ITEMS.map(x => Object.assign({}, x));
        const total = round2(items.reduce((n, x) => n + x.price * x.qty, 0));
        addBill({ kind: 'clinic', title: '复诊续方 · ' + c.dept, patientId: c.patientId, patient: c.patient, consultId: c.id, items, total, insured: 0, self: total, status: 'unpaid' });
      }
    }
  });
  if (cc) saveConsults(cs);
}

function getFollows() { return list(K.follow, () => []); }

module.exports = {
  MIN, DAY, MAX_PATIENTS, pad, round2, dateStr, hm, dateTime, dayAfter, today, parseDate,
  getPatients, savePatients, findPatient, defaultPatient, upsertPatient, setDefaultPatient, removePatient,
  getOrders, saveOrders, findOrder, updateOrder, isActive, slotsFor, leftOf, canBook, createOrder, cancelDeadline, cancelOrder, checkin, stageOf,
  getBills, findBill, addBill, updateBill, payBill, unpaidBills, insuredPart,
  getReports, markReportRead, unreadReports,
  getConsults, addConsult, updateConsult, cancelConsult,
  getInpatient, saveInpatient, inpatientDays, inpatientSummary, addDeposit, settleInpatient,
  tick, getFollows
};
