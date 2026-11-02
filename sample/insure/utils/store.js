// 本地数据：保单、理赔、推荐码，全部走 wx.setStorageSync，退出重进还在。
const util = require('./util.js');
const K_POLICY = 'ins_policies';
const K_CLAIM = 'ins_claims';
const K_PROFILE = 'ins_profile';

function read(k, def) { const v = wx.getStorageSync(k); return v === '' || v == null ? def : v; }

// 保障状态：待支付 / 待生效（次日零时生效）/ 保障中 / 已过期 / 已取消
function statusOf(p, now) {
  now = now || new Date();
  if (p.status === 'pending') return { key: 'pending', text: '待支付' };
  if (p.status === 'cancelled') return { key: 'cancelled', text: '已取消' };
  const today = util.ymd(now);
  if (p.endDate < today) return { key: 'expired', text: '已过期' };
  if (p.startDate > today) return { key: 'waiting', text: '待生效' };
  return { key: 'active', text: '保障中' };
}

function policies() { return read(K_POLICY, []); }
function savePolicies(list) { wx.setStorageSync(K_POLICY, list); }
function getPolicy(id) { return policies().find(p => p.id === id) || null; }
function addPolicy(p) { const list = policies(); list.unshift(p); savePolicies(list); return p; }
function updatePolicy(id, patch) {
  const list = policies();
  const i = list.findIndex(p => p.id === id);
  if (i < 0) return null;
  list[i] = Object.assign({}, list[i], patch);
  savePolicies(list);
  return list[i];
}

// 支付成功：出保单号，次日零时生效
function markPaid(id, method) {
  const now = new Date();
  const p = getPolicy(id);
  if (!p) return null;
  if (p.status === 'paid') return p;
  const start = util.addDays(now, 1);
  let end;
  const m = /^(\d+)天$/.exec(p.term);
  if (m) end = util.addDays(start, +m[1] - 1);
  else if (p.term === '1年') end = util.addDays(new Date(start.getFullYear() + 1, start.getMonth(), start.getDate()), -1);
  else {
    const to = p.term === '保终身' ? 105 : +(/\d+/.exec(p.term) || [60])[0];
    const b = new Date(p.holder.birthday.replace(/-/g, '/'));
    end = new Date(b.getFullYear() + to, b.getMonth(), b.getDate());
  }
  return updatePolicy(id, {
    status: 'paid', payMethod: method, paidAt: now.getTime(),
    policyNo: 'AX' + util.serial('').slice(2),
    startDate: util.ymd(start), endDate: util.ymd(end)
  });
}

// 理赔进度：提交后按时间推进（材料审核 1 分钟、理赔核定 3 分钟、赔款到账 5 分钟），撤销后停在当前步骤
const STEPS = [
  { key: 'submit', name: '提交申请', after: 0, note: '理赔申请已受理，案件号已生成' },
  { key: 'review', name: '材料审核', after: 60, note: '理赔专员已审核材料，材料齐全' },
  { key: 'assess', name: '理赔核定', after: 180, note: '已按保险责任核定赔付金额' },
  { key: 'pay', name: '赔款到账', after: 300, note: '赔款已转入您的收款账户，请注意查收' }
];
function claimView(c, now) {
  now = now || Date.now();
  const elapsed = ((c.cancelledAt || now) - c.createdAt) / 1000;
  let cur = 0;
  STEPS.forEach((s, i) => { if (elapsed >= s.after) cur = i; });
  const steps = STEPS.map((s, i) => ({
    name: s.name, note: i <= cur ? (s.key === 'assess' ? '核定赔付 ¥' + util.money(c.approved) + '，' + s.note.slice(2) : s.note) : '',
    time: i <= cur ? util.ymdhm(c.createdAt + s.after * 1000) : '',
    state: i <= cur ? 'done' : i === cur + 1 ? (c.cancelledAt ? 'stop' : 'doing') : 'todo'
  }));
  let statusText = c.cancelledAt ? '已撤销' : cur === STEPS.length - 1 ? '已赔付' : STEPS[cur + 1].name + '中';
  return Object.assign({}, c, { steps, cur, statusText, finished: cur === STEPS.length - 1 || !!c.cancelledAt, createdText: util.ymdhm(c.createdAt) });
}
function claims() { return read(K_CLAIM, []); }
function addClaim(c) { const list = claims(); list.unshift(c); wx.setStorageSync(K_CLAIM, list); return c; }
function cancelClaim(id) {
  const list = claims();
  const c = list.find(x => x.id === id);
  if (c && !c.cancelledAt) c.cancelledAt = Date.now();
  wx.setStorageSync(K_CLAIM, list);
}

function profile() { return read(K_PROFILE, { referral: '' }); }
function saveProfile(p) { wx.setStorageSync(K_PROFILE, Object.assign(profile(), p)); }

function clearAll() { [K_POLICY, K_CLAIM, K_PROFILE].forEach(k => wx.removeStorageSync(k)); }

module.exports = { statusOf, policies, getPolicy, addPolicy, updatePolicy, markPaid, claims, addClaim, cancelClaim, claimView, profile, saveProfile, clearAll };
