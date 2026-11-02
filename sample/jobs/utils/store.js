// 本地状态：简历 / 投递 / 收藏 / 关注 / 会话与消息 / 面试邀约 / 求职意向 / 隐私设置 / 反馈，全部存在 storage 里
const data = require('./data.js');
const K = {
  resume: 'jobs_resume', fav: 'jobs_fav', applied: 'jobs_applied', follow: 'jobs_follow', intent: 'jobs_intent',
  sessions: 'jobs_sessions', msgs: 'jobs_msgs', interviews: 'jobs_interviews', settings: 'jobs_settings',
  feedback: 'jobs_feedback', seeded: 'jobs_seeded'
};
const MIN = 60000;
const DAY = 86400000;
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
// 投递状态：0 已投递 1 被查看 2 邀面试 3 不合适
const APP_STATUS = ['已投递', '被查看', '邀面试', '不合适'];
const IV_STATUS = { pending: '待确认', accepted: '已接受', declined: '已拒绝' };
function pad(n) {
  return n < 10 ? '0' + n : '' + n;
}
function hm(ts) {
  const d = new Date(ts);
  return pad(d.getHours()) + ':' + pad(d.getMinutes());
}
function startOfDay(ts) {
  const d = new Date(ts);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}
// 会话列表里的时间：今天 HH:MM / 昨天 / 一周内周几 / M/D
function shortTime(ts) {
  const today = startOfDay(Date.now());
  if (ts >= today) return hm(ts);
  if (ts >= today - DAY) return '昨天';
  if (ts >= today - 6 * DAY) return WEEK[new Date(ts).getDay()];
  const d = new Date(ts);
  return (d.getMonth() + 1) + '/' + d.getDate();
}
function dayText(ts) {
  const d = new Date(ts);
  return (d.getMonth() + 1) + '月' + d.getDate() + '日';
}
function fullTime(ts) {
  const d = new Date(ts);
  return dayText(ts) + ' ' + WEEK[d.getDay()] + ' ' + hm(ts);
}
// getStorageSync 缺省给 ''：数组 / 对象各自兜底，不用 `||`
function list(k) {
  const v = wx.getStorageSync(k);
  return Array.isArray(v) ? v : [];
}
function obj(k) {
  const v = wx.getStorageSync(k);
  return v && typeof v === 'object' && !Array.isArray(v) ? v : {};
}
// ───────── 首次打开的演示数据 ─────────
function ensureSeed() {
  if (wx.getStorageSync(K.seeded)) return;
  const now = Date.now();
  if (!wx.getStorageSync(K.resume)) {
    wx.setStorageSync(K.resume, {
      name: '林晓', gender: 1, age: '27', phone: '138****6621', city: '杭州',
      edu: 1, school: '浙江工业大学', major: '计算机科学与技术',
      expYears: 2, position: '前端开发工程师', salary: 3, open: true,
      intro: '4 年 Web 与小程序开发经验，主导过日活百万级电商小程序的性能优化，首屏耗时下降 38%。熟悉 React、TypeScript 与工程化建设。',
      works: [{ id: 1, company: '杭州某电商科技', title: '前端开发工程师', from: '2022', to: '至今' }]
    });
  }
  if (!list(K.applied).length) {
    wx.setStorageSync(K.applied, [
      { id: 'j3', ts: now - 10 * DAY, status: 2 },
      { id: 'j4', ts: now - 12 * DAY, status: 1 },
      { id: 'j5', ts: now - 16 * DAY, status: 3 }
    ]);
  }
  const sessions = [];
  const msgs = {};
  data.chats.forEach(c => {
    const end = now - c.ago * MIN;
    const n = c.history.length;
    msgs[c.id] = c.history.map((m, i) => ({
      id: i + 1, from: m.me ? 'me' : 'hr', text: m.text, invite: m.invite || '', ts: end - (n - 1 - i) * 3 * MIN
    }));
    sessions.push({ id: c.id, hr: c.hr, company: c.company, job: c.job, unread: c.unread, updatedAt: end });
  });
  wx.setStorageSync(K.sessions, sessions);
  wx.setStorageSync(K.msgs, msgs);
  // s3 里那条面试邀请：明天 14:00 视频面试
  const t = startOfDay(now) + DAY + 14 * 60 * MIN;
  wx.setStorageSync(K.interviews, [{
    id: 'iv1', job: 'j3', session: 's3', ts: t, mode: '视频面试', place: '腾讯会议 · 会议号 618 203 775', status: 'pending', createdAt: now - 25 * 60 * MIN
  }]);
  wx.setStorageSync(K.seeded, true);
}
// ───────── 简历 ─────────
function getResume() {
  ensureSeed();
  const r = obj(K.resume);
  return Object.assign({ intro: '', works: [], open: true }, r);
}
function saveResume(r) {
  wx.setStorageSync(K.resume, r);
}
// 投递前要有姓名与手机号
function resumeReady() {
  const r = getResume();
  return !!(String(r.name || '').trim() && String(r.phone || '').trim());
}
// ───────── 投递 ─────────
function getApps() {
  ensureSeed();
  return list(K.applied).map(a => {
    const j = data.getJob(a.id);
    return Object.assign({}, a, { title: j.title, salary: j.salary, co: j.co, statusText: APP_STATUS[a.status], time: a.ts ? dayText(a.ts) : (a.time || '') });
  });
}
function getApp(jobId) {
  return getApps().find(a => a.id === jobId) || null;
}
function saveApps(l) {
  wx.setStorageSync(K.applied, l.map(a => ({ id: a.id, ts: a.ts, status: a.status })));
}
// 已投过返回 null
function apply(jobId) {
  const l = getApps();
  if (l.some(a => a.id === jobId)) return null;
  const rec = { id: jobId, ts: Date.now(), status: 0 };
  l.unshift(rec);
  saveApps(l);
  return rec;
}
// 只能往前推（被查看 → 邀面试 → 不合适），不会倒退
function setAppStatus(jobId, status) {
  const l = getApps();
  let a = l.find(x => x.id === jobId);
  if (!a) {
    a = { id: jobId, ts: Date.now(), status };
    l.unshift(a);
  } else if (status > a.status) {
    a.status = status;
  }
  saveApps(l);
}
// HR 还没约面试之前可以撤回
function withdraw(jobId) {
  const l = getApps();
  const a = l.find(x => x.id === jobId);
  if (!a || a.status >= 2) return false;
  saveApps(l.filter(x => x.id !== jobId));
  return true;
}
// ───────── 收藏职位 / 关注公司 ─────────
function getFavs() {
  return list(K.fav);
}
function isFav(id) {
  return getFavs().indexOf(id) >= 0;
}
function toggleFav(id) {
  const l = getFavs();
  const on = l.indexOf(id) < 0;
  wx.setStorageSync(K.fav, on ? [id].concat(l) : l.filter(x => x !== id));
  return on;
}
function getFollows() {
  return list(K.follow);
}
function isFollow(id) {
  return getFollows().indexOf(id) >= 0;
}
function toggleFollow(id) {
  const l = getFollows();
  const on = l.indexOf(id) < 0;
  wx.setStorageSync(K.follow, on ? l.concat([id]) : l.filter(x => x !== id));
  return on;
}
// ───────── 会话 / 消息 ─────────
function rawSessions() {
  ensureSeed();
  return list(K.sessions);
}
function allMsgs() {
  return obj(K.msgs);
}
function getMsgs(sid) {
  return allMsgs()[sid] || [];
}
function previewOf(m) {
  if (!m) return '';
  return m.text;
}
function getSessions() {
  const ivs = list(K.interviews);
  const msgs = allMsgs();
  return rawSessions().slice().sort((a, b) => b.updatedAt - a.updatedAt).map(s => {
    const l = msgs[s.id] || [];
    const last = l[l.length - 1];
    return Object.assign({}, s, {
      hrInfo: data.hrs[s.hr], co: data.companies[s.company], jobTitle: data.getJob(s.job).title,
      last: previewOf(last), time: shortTime(s.updatedAt),
      hasInvite: ivs.some(v => v.session === s.id)
    });
  });
}
function getSession(sid) {
  return getSessions().find(s => s.id === sid) || null;
}
// 某个职位的会话；没有就新建（从职位详情点「聊一聊」）
function sessionForJob(jobId) {
  const l = rawSessions();
  let s = l.find(x => x.job === jobId);
  if (!s) {
    const j = data.getJob(jobId);
    s = { id: 'n' + jobId, hr: j.hr, company: j.company, job: jobId, unread: 0, updatedAt: Date.now() };
    l.push(s);
    wx.setStorageSync(K.sessions, l);
    const m = allMsgs();
    m[s.id] = [{ id: 1, from: 'sys', text: '你正在与' + j.co.name + '的' + j.hrInfo.name + '沟通「' + j.title + '」', ts: Date.now() }];
    wx.setStorageSync(K.msgs, m);
  }
  return s.id;
}
function addMsg(sid, msg) {
  const m = allMsgs();
  const l = m[sid] || [];
  const rec = Object.assign({ id: l.reduce((x, y) => Math.max(x, y.id), 0) + 1, ts: Date.now(), invite: '' }, msg);
  l.push(rec);
  m[sid] = l;
  wx.setStorageSync(K.msgs, m);
  const ss = rawSessions();
  const s = ss.find(x => x.id === sid);
  if (s) {
    s.updatedAt = rec.ts;
    wx.setStorageSync(K.sessions, ss);
  }
  return rec;
}
function markRead(ids) {
  const ss = rawSessions();
  ss.forEach(s => { if (ids.indexOf(s.id) >= 0) s.unread = 0; });
  wx.setStorageSync(K.sessions, ss);
}
function unreadTotal() {
  return rawSessions().reduce((n, s) => n + s.unread, 0);
}
// ───────── 面试邀约 ─────────
function getInterviews() {
  ensureSeed();
  return list(K.interviews).slice().sort((a, b) => a.ts - b.ts).map(v => {
    const j = data.getJob(v.job);
    return Object.assign({}, v, {
      title: j.title, salary: j.salary, co: j.co, hrInfo: j.hrInfo, when: fullTime(v.ts),
      statusText: IV_STATUS[v.status], past: v.ts < Date.now()
    });
  });
}
function getInterview(id) {
  return getInterviews().find(v => v.id === id) || null;
}
function saveInterview(id, patch) {
  const l = list(K.interviews);
  const v = l.find(x => x.id === id);
  if (v) Object.assign(v, patch);
  wx.setStorageSync(K.interviews, l);
  return v;
}
// HR 发来新的面试邀请：后天 10:30 到公司现场面试
function createInterview(jobId, sid) {
  const l = list(K.interviews);
  const j = data.getJob(jobId);
  const id = 'iv' + (l.length + 1) + '_' + jobId;
  const v = { id, job: jobId, session: sid, ts: startOfDay(Date.now()) + 2 * DAY + 630 * MIN, mode: '现场面试', place: j.co.addr, status: 'pending', createdAt: Date.now() };
  l.push(v);
  wx.setStorageSync(K.interviews, l);
  setAppStatus(jobId, 2);
  return v;
}
function acceptInterview(id) {
  const v = saveInterview(id, { status: 'accepted', decidedAt: Date.now() });
  if (v) {
    setAppStatus(v.job, 2);
    addMsg(v.session, { from: 'me', text: '[已接受面试] ' + fullTime(v.ts) + ' ' + v.mode });
  }
  return v;
}
function declineInterview(id, reason) {
  const v = saveInterview(id, { status: 'declined', reason, decidedAt: Date.now() });
  if (v) addMsg(v.session, { from: 'me', text: '[已拒绝面试] ' + reason });
  return v;
}
function setReminder(id) {
  return saveInterview(id, { remind: true });
}
function pendingInterviews() {
  return getInterviews().filter(v => v.status === 'pending' && !v.past).length;
}
// ───────── 求职意向 / 设置 / 反馈 ─────────
const JOB_STATUS = ['离职 · 随时到岗', '在职 · 月内到岗', '在职 · 考虑机会', '在职 · 暂不考虑'];
const JOB_TYPES = ['全职', '实习', '兼职'];
function getIntent() {
  const v = obj(K.intent);
  return {
    intent: v.intent || data.INTENTS[0], cityIdx: typeof v.cityIdx === 'number' ? v.cityIdx : 0,
    salary: typeof v.salary === 'number' ? v.salary : 0, status: typeof v.status === 'number' ? v.status : 2,
    type: typeof v.type === 'number' ? v.type : 0
  };
}
function saveIntent(patch) {
  wx.setStorageSync(K.intent, Object.assign(getIntent(), patch));
}
function getSettings() {
  const v = obj(K.settings);
  return { hidePhone: v.hidePhone === true, notify: v.notify !== false, blocked: Array.isArray(v.blocked) ? v.blocked : [] };
}
function saveSettings(patch) {
  wx.setStorageSync(K.settings, Object.assign(getSettings(), patch));
}
function getFeedback() {
  return list(K.feedback).map(f => Object.assign({}, f, { time: dayText(f.ts) + ' ' + hm(f.ts) }));
}
function addFeedback(f) {
  const l = list(K.feedback);
  l.unshift(Object.assign({ id: l.length + 1, ts: Date.now(), status: '处理中' }, f));
  wx.setStorageSync(K.feedback, l);
}
module.exports = {
  K, APP_STATUS, JOB_STATUS, JOB_TYPES, ensureSeed, shortTime, hm, dayText, fullTime,
  getResume, saveResume, resumeReady, getApps, getApp, apply, setAppStatus, withdraw,
  getFavs, isFav, toggleFav, getFollows, isFollow, toggleFollow,
  getSessions, getSession, getMsgs, sessionForJob, addMsg, markRead, unreadTotal,
  getInterviews, getInterview, createInterview, acceptInterview, declineInterview, setReminder, pendingInterviews,
  getIntent, saveIntent, getSettings, saveSettings, getFeedback, addFeedback
};
