// 跨页数据统一走 storage：咨询订单（含会话消息）、合同草稿、收藏的律师。退出重进状态还在。
const K_CONSULT = 'legal_consults';
const K_DRAFT = 'legal_drafts';
const K_FAV = 'legal_favs';

function read(k, def) {
  const v = wx.getStorageSync(k);
  return v === '' || v === undefined || v === null ? def : v;
}
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function now() {
  const d = new Date();
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
function hm() { const d = new Date(); return `${pad(d.getHours())}:${pad(d.getMinutes())}`; }

// ---------- 咨询 ----------
function consults() { return read(K_CONSULT, []); }
function saveConsults(list) { wx.setStorageSync(K_CONSULT, list); }
function getConsult(id) { return consults().find(c => c.id === id); }
function updateConsult(id, patch) {
  const list = consults();
  const i = list.findIndex(c => c.id === id);
  if (i < 0) return null;
  list[i] = Object.assign({}, list[i], typeof patch === 'function' ? patch(list[i]) : patch);
  saveConsults(list);
  return list[i];
}
function addConsult(c) {
  const list = consults();
  const id = 'C' + Date.now().toString().slice(-8);
  const rec = Object.assign({ id, createdAt: now(), status: 'active', messages: [] }, c);
  list.unshift(rec);
  saveConsults(list);
  return rec;
}
function pushMessage(id, msg) {
  return updateConsult(id, c => ({ messages: c.messages.concat([Object.assign({ id: 'm' + Date.now() + Math.floor(Math.random() * 1000), time: hm() }, msg)]) }));
}

// ---------- 合同草稿 ----------
function drafts() { return read(K_DRAFT, []); }
function saveDraft(d) {
  const list = drafts();
  const i = list.findIndex(x => x.id === d.id);
  const rec = Object.assign({}, d, { updatedAt: now() });
  if (i >= 0) list.splice(i, 1);
  list.unshift(rec);
  wx.setStorageSync(K_DRAFT, list);
  return rec;
}
function getDraft(id) { return drafts().find(x => x.id === id); }
function removeDraft(id) { wx.setStorageSync(K_DRAFT, drafts().filter(x => x.id !== id)); }

// ---------- 收藏 ----------
function favs() { return read(K_FAV, []); }
function toggleFav(id) {
  const list = favs();
  const i = list.indexOf(id);
  if (i >= 0) list.splice(i, 1); else list.unshift(id);
  wx.setStorageSync(K_FAV, list);
  return i < 0;
}

const STATUS = { active: '咨询中', done: '待评价', rated: '已完成' };

module.exports = {
  now, hm, consults, getConsult, updateConsult, addConsult, pushMessage,
  drafts, saveDraft, getDraft, removeDraft, favs, toggleFav, STATUS
};
