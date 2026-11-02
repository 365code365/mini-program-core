// 本地数据：练习记录、打卡、已学章节、收藏、睡眠、心情。全部走 storage，退出重进还在
const K = {
  sessions: 'md_sessions', // [{ts, date, sec, type: 'course'|'breath', title}]
  done: 'md_done',         // {courseId: [已完成章节下标]}
  favs: 'md_favs',         // [courseId]
  sleep: 'md_sleep',       // [{date, bed, wake, quality}]
  moods: 'md_moods',       // [{id, ts, date, time, mood, tags, text}]
  last: 'md_last',         // {courseId, idx}
  settings: 'md_settings'  // {theme: light|dark|system, remind, remindTime, noiseOnHide}
};

function get(key, def) {
  const v = wx.getStorageSync(key);
  return v === '' || v === undefined || v === null ? def : v;
}

function pad(n) { return n < 10 ? '0' + n : '' + n; }

function dateStr(d) {
  d = d || new Date();
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
}

function addDays(d, n) {
  const x = new Date(d.getTime());
  x.setDate(x.getDate() + n);
  return x;
}

function sessions() { return get(K.sessions, []); }

function addSession(sec, type, title) {
  const list = sessions();
  list.push({ ts: Date.now(), date: dateStr(), sec: Math.max(1, Math.round(sec)), type, title });
  wx.setStorageSync(K.sessions, list);
  return list;
}

// 连续打卡：从今天（今天还没练就从昨天）往前数有练习的天数
function streakInfo() {
  const days = {};
  sessions().forEach(s => { days[s.date] = true; });
  const today = new Date();
  let cur = days[dateStr(today)] ? today : addDays(today, -1);
  let streak = 0;
  while (days[dateStr(cur)]) { streak++; cur = addDays(cur, -1); }
  // 最长连续
  const sorted = Object.keys(days).sort();
  let best = 0, run = 0, prev = null;
  sorted.forEach(ds => {
    const d = new Date(ds.replace(/-/g, '/'));
    run = prev && dateStr(addDays(prev, 1)) === ds ? run + 1 : 1;
    best = Math.max(best, run);
    prev = d;
  });
  const week = [];
  const names = ['日', '一', '二', '三', '四', '五', '六'];
  for (let i = 6; i >= 0; i--) {
    const d = addDays(today, -i);
    week.push({ label: i === 0 ? '今' : names[d.getDay()], on: !!days[dateStr(d)] });
  }
  return { streak, best, todayDone: !!days[dateStr(today)], week, totalDays: sorted.length };
}

function stats() {
  const list = sessions();
  const sec = list.reduce((a, s) => a + s.sec, 0);
  return {
    count: list.length,
    minutes: Math.round(sec / 6) / 10, // 保留 1 位小数（内置示范音频较短）
    breath: list.filter(s => s.type === 'breath').length
  };
}

function doneMap() { return get(K.done, {}); }

function markDone(courseId, idx) {
  const m = doneMap();
  const arr = m[courseId] || [];
  if (arr.indexOf(idx) < 0) arr.push(idx);
  m[courseId] = arr;
  wx.setStorageSync(K.done, m);
}

// 第 1 节免费；之后每节要完成上一节才解锁
function unlocked(courseId, idx) {
  if (idx === 0) return true;
  return (doneMap()[courseId] || []).indexOf(idx - 1) >= 0;
}

function favs() { return get(K.favs, []); }

function toggleFav(id) {
  const f = favs();
  const i = f.indexOf(id);
  if (i >= 0) f.splice(i, 1); else f.unshift(id);
  wx.setStorageSync(K.favs, f);
  return i < 0;
}

function settings() {
  return Object.assign({ theme: 'light', remind: false, remindTime: '22:30', noiseOnHide: true }, get(K.settings, {}));
}

function saveSettings(patch) {
  const s = Object.assign(settings(), patch);
  wx.setStorageSync(K.settings, s);
  return s;
}

// 首次打开：导入近 6 天的手环睡眠数据（演示用），后面用户自己记
function sleepList() {
  let list = wx.getStorageSync(K.sleep);
  if (list === '' || list === undefined) {
    const seed = [['23:40', '07:05', 3], ['00:20', '07:30', 2], ['23:10', '06:50', 4], ['23:55', '07:15', 3], ['01:05', '07:40', 2], ['22:50', '06:45', 5]];
    const today = new Date();
    list = seed.map((s, i) => ({ date: dateStr(addDays(today, -(6 - i))), bed: s[0], wake: s[1], quality: s[2], seed: true }));
    wx.setStorageSync(K.sleep, list);
  }
  return list;
}

function saveSleep(list) { wx.setStorageSync(K.sleep, list); }

// 入睡 → 起床的时长（小时，跨午夜）
function sleepHours(bed, wake) {
  const p = s => { const a = s.split(':'); return +a[0] * 60 + +a[1]; };
  let m = p(wake) - p(bed);
  if (m <= 0) m += 24 * 60;
  return Math.round(m / 6) / 10;
}

function moods() { return get(K.moods, []); }

function saveMoods(list) { wx.setStorageSync(K.moods, list); }

function last() { return get(K.last, null); }

function setLast(courseId, idx) { wx.setStorageSync(K.last, { courseId, idx }); }

function clearAll() {
  Object.keys(K).forEach(k => wx.removeStorageSync(K[k]));
}

module.exports = {
  dateStr, addDays, pad, sessions, addSession, streakInfo, stats, doneMap, markDone, unlocked,
  favs, toggleFav, settings, saveSettings, sleepList, saveSleep, sleepHours, moods, saveMoods, last, setLast, clearAll
};
