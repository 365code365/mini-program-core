// 学习数据都在本地 storage：设置、每个词的复习阶段、每日记录、打卡、生词本、错题本
const D = require('./date.js');
const books = require('./books.js');

const K = {
  set: 'vocab_settings', prog: 'vocab_progress', log: 'vocab_log',
  check: 'vocab_checkin', star: 'vocab_star', wrong: 'vocab_wrong'
};
// 艾宾浩斯式间隔：第 n 次复习通过后，隔 INTERVALS[n] 天再复习；过完最后一档算「已掌握」
const INTERVALS = [1, 2, 4, 7, 15];
const DEFAULT_SET = { book: 'cet4', plan: 8, remind: '21:00', remindOn: false, autoPlay: true, nick: '词力学员' };

function get(k, def) {
  const v = wx.getStorageSync(k);
  return v === '' || v === undefined || v === null ? def : v;
}
function put(k, v) { wx.setStorageSync(k, v); }

function settings() { return Object.assign({}, DEFAULT_SET, get(K.set, {})); }
function saveSettings(patch) { const s = Object.assign(settings(), patch); put(K.set, s); return s; }
function progress() { return get(K.prog, {}); }
function logs() { return get(K.log, {}); }
function bump(field, n) {
  const l = logs(), t = D.today();
  l[t] = Object.assign({ learn: 0, review: 0, quiz: 0 }, l[t]);
  l[t][field] += n || 1;
  put(K.log, l);
}
function todayLog() { return Object.assign({ learn: 0, review: 0, quiz: 0 }, logs()[D.today()]); }

// 新学一个词：认识直接进第 2 档（2 天后复习），模糊 / 不认识从第 1 档（明天）开始
function markLearn(item, res) {
  const p = progress(), t = D.today();
  const stage = res === 'known' ? 1 : 0;
  p[item.w] = { book: item.book, stage, due: D.addDays(t, INTERVALS[stage]), first: t, last: t, res };
  put(K.prog, p);
  bump('learn');
}
// 复习：记得升一档，忘了退回第 1 档；升过最后一档 = 已掌握，不再安排复习
function markReview(w, ok) {
  const p = progress(), t = D.today(), r = p[w];
  if (!r) return null;
  if (ok) {
    r.stage += 1;
    r.due = r.stage >= INTERVALS.length ? '' : D.addDays(t, INTERVALS[r.stage]);
  } else {
    r.stage = 0;
    r.due = D.addDays(t, INTERVALS[0]);
  }
  r.last = t;
  put(K.prog, p);
  bump('review');
  return r;
}
function dueList() {
  const p = progress(), t = D.today();
  return Object.keys(p).filter((w) => p[w].due && p[w].due <= t)
    .sort((a, b) => (p[a].due < p[b].due ? -1 : 1))
    .map((w) => Object.assign({}, books.find(w), { stage: p[w].stage, due: p[w].due }));
}
// 未来 n 天每天要复习几个
function upcoming(n) {
  const p = progress(), t = D.today(), out = [];
  for (let i = 1; i <= n; i++) {
    const day = D.addDays(t, i);
    out.push({ day, label: i === 1 ? '明天' : i === 2 ? '后天' : D.short(day), count: Object.keys(p).filter((w) => p[w].due === day).length });
  }
  return out;
}
function newWords(bookId, n) {
  const p = progress();
  return books.words(bookId).filter((x) => !p[x.w]).slice(0, Math.max(0, n));
}
function learnedOf(bookId) {
  const p = progress();
  return books.words(bookId).filter((x) => p[x.w]);
}

function checkins() { return get(K.check, []); }
function checkedToday() { return checkins().indexOf(D.today()) >= 0; }
function checkin() {
  const c = checkins(), t = D.today();
  if (c.indexOf(t) < 0) { c.push(t); c.sort(); put(K.check, c); }
  return streak();
}
// 连续打卡天数：今天还没打就从昨天往回数
function streak() {
  const c = checkins();
  let d = D.today(), n = 0;
  if (c.indexOf(d) < 0) d = D.addDays(d, -1);
  while (c.indexOf(d) >= 0) { n++; d = D.addDays(d, -1); }
  return n;
}

function stars() { return get(K.star, []); }
function isStar(w) { return stars().indexOf(w) >= 0; }
function toggleStar(w) {
  const s = stars(), i = s.indexOf(w);
  if (i >= 0) s.splice(i, 1); else s.unshift(w);
  put(K.star, s);
  return i < 0;
}
function addStar(w) { if (!isStar(w)) toggleStar(w); }

function wrongs() { return get(K.wrong, []); }
function addWrong(w) {
  const l = wrongs(), r = l.find((x) => x.w === w);
  if (r) { r.times += 1; r.last = D.today(); } else l.unshift({ w, times: 1, last: D.today() });
  put(K.wrong, l);
}
function removeWrong(w) { put(K.wrong, wrongs().filter((x) => x.w !== w)); }
function clearWrong() { put(K.wrong, []); }

function resetAll() { Object.keys(K).forEach((k) => wx.removeStorageSync(K[k])); }

// 首次打开的演示数据：近 10 天陆续学了四级的 16 个词，按间隔规则推演复习，到今天有几个刚好到期
function ensureSeed() {
  if (wx.getStorageSync(K.set)) return;
  const t = D.today();
  const plan = [[-10, 3], [-9, 2], [-8, 2], [-7, 2], [-5, 3], [-4, 2], [-2, 2]];
  const list = books.words('cet4');
  const p = {}, l = {}, days = {};
  const add = (day, f) => { l[day] = Object.assign({ learn: 0, review: 0, quiz: 0 }, l[day]); l[day][f] += 1; days[day] = 1; };
  let k = 0;
  plan.forEach(([off, n]) => {
    for (let j = 0; j < n; j++, k++) {
      const it = list[k], first = D.addDays(t, off);
      const r = { book: 'cet4', stage: k % 3 === 0 ? 1 : 0, first, last: first, res: k % 3 === 0 ? 'known' : 'fuzzy' };
      r.due = D.addDays(first, INTERVALS[r.stage]);
      add(first, 'learn');
      // 推演复习：每次都按时复习，k % 5 === 2 的词第一次复习忘了
      let fails = k % 5 === 2 ? 1 : 0;
      while (r.due && r.due < t) {
        const day = r.due;
        add(day, 'review');
        if (fails > 0) { fails--; r.stage = 0; } else r.stage += 1;
        r.due = r.stage >= INTERVALS.length ? '' : D.addDays(day, INTERVALS[r.stage]);
        r.last = day;
      }
      p[it.w] = r;
    }
  });
  l[D.addDays(t, -6)] = { learn: 0, review: 0, quiz: 10 };
  days[D.addDays(t, -6)] = 1;
  put(K.prog, p);
  put(K.log, l);
  put(K.check, Object.keys(days).sort());
  put(K.star, ['adequate', 'acquire', 'accompany']);
  put(K.wrong, [{ w: 'accompany', times: 2, last: D.addDays(t, -6) }, { w: 'adequate', times: 1, last: D.addDays(t, -6) }]);
  put(K.set, Object.assign({}, DEFAULT_SET));
}

module.exports = {
  INTERVALS, settings, saveSettings, progress, logs, todayLog, bump, markLearn, markReview, dueList, upcoming,
  newWords, learnedOf, checkins, checkedToday, checkin, streak, stars, isStar, toggleStar, addStar,
  wrongs, addWrong, removeWrong, clearWrong, resetAll, ensureSeed
};
