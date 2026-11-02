// 约球广场：组局的种子数据（日期按今天往后排）、报名 / 退出、发起
const D = require('./data.js');
const F = require('./fmt.js');
const store = require('./store.js');
const LEVELS = ['新手友好', '中级', '高手局'];
function day(n) { const d = new Date(); d.setHours(0, 0, 0, 0); d.setDate(d.getDate() + n); return F.dateKey(d); }
function seed() {
  return [
    { id: 't1', sport: 'badminton', venueId: 'v1', date: day(0), start: 19, end: 21, need: 6, fee: 30, level: 1, note: '双打轮换，自带球拍，羽毛球我出（威克多 金黄2号）', host: { name: '阿杰', avatar: '/images/avatar1.png' }, joined: ['阿杰', '小雨', '老周', 'Kevin'], createdAt: Date.now() - 7200000 },
    { id: 't2', sport: 'basketball', venueId: 'v2', date: day(1), start: 20, end: 22, need: 10, fee: 38, level: 1, note: '全场 5v5，打 4 节每节 12 分钟，穿深浅两色球衣各一件', host: { name: '大刘', avatar: '/images/avatar2.png' }, joined: ['大刘', '阿明', '浩子', '小宋', '东东', 'Jason', '老王'], createdAt: Date.now() - 18000000 },
    { id: 't3', sport: 'tennis', venueId: 'v3', date: day(2), start: 8, end: 10, need: 4, fee: 60, level: 0, note: '周末早场，NTRP 2.5 左右，以对拉为主，欢迎新手', host: { name: 'Lily', avatar: '/images/avatar3.png' }, joined: ['Lily', '晓峰'], createdAt: Date.now() - 36000000 },
    { id: 't4', sport: 'football', venueId: 'v4', date: day(3), start: 19, end: 21, need: 12, fee: 50, level: 2, note: '五人制轮换，节奏快，请穿碎钉鞋', host: { name: '球球', avatar: '/images/avatar4.png' }, joined: ['球球', '阿亮', '小黑', '老K', '阿强', '凯凯', '小飞', '大鹏', '东子', '阿伟', '小马', '石头'], createdAt: Date.now() - 50000000 },
    { id: 't5', sport: 'pickle', venueId: 'v6', date: day(4), start: 10, end: 12, need: 4, fee: 40, level: 0, note: '匹克球体验局，球拍场馆免费借，零基础教规则', host: { name: '小雨', avatar: '/images/avatar3.png' }, joined: ['小雨'], createdAt: Date.now() - 3600000 }
  ];
}
function all() {
  let t = store.teams();
  if (!t) { t = seed(); store.saveTeams(t); }
  return t;
}
function me() { return store.contact().name || '我'; }
function view(t) {
  const v = D.getVenue(t.venueId);
  const n = t.joined.length;
  const ended = new Date(F.fromKey(t.date).getTime() + t.end * 3600000).getTime() < Date.now();
  return Object.assign({}, t, {
    venueName: v.name, sportName: D.sportName(t.sport), dateText: F.dateText(t.date), timeText: F.hh(t.start) + '-' + F.hh(t.end),
    count: n, pct: Math.round(n * 100 / t.need), full: n >= t.need, inIt: t.joined.indexOf(me()) >= 0, ended,
    levelText: LEVELS[t.level], left: Math.max(0, t.need - n), shown: t.joined.slice(0, 5)
  });
}
function update(id, fn) {
  const list = all();
  const t = list.find((x) => x.id === id);
  if (!t) return null;
  fn(t);
  store.saveTeams(list);
  return t;
}
function join(id) { return update(id, (t) => { if (t.joined.indexOf(me()) < 0 && t.joined.length < t.need) t.joined.push(me()); }); }
function quit(id) { return update(id, (t) => { t.joined = t.joined.filter((x) => x !== me()); }); }
function remove(id) { store.saveTeams(all().filter((x) => x.id !== id)); }
function create(t) {
  const list = all();
  const name = me();
  const team = Object.assign({ id: 't' + Date.now(), host: { name, avatar: '/images/avatar1.png' }, joined: [name], mine: true, createdAt: Date.now() }, t);
  list.unshift(team);
  store.saveTeams(list);
  return team;
}
module.exports = { all, view, join, quit, remove, create, me, LEVELS, day };
