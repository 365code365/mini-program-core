// 已购课程、学习进度、收藏、打卡记录：存在本地 storage
const data = require('./data.js');

function pad(v) { return v < 10 ? '0' + v : '' + v; }
function dayKey(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function daysAgo(n) { const t = new Date(); return dayKey(new Date(t.getFullYear(), t.getMonth(), t.getDate() - n)); }

// 首次打开：送两门已购课程，带一点进度，学习中心不是空的
const DEFAULT_LEARN = {
  k1: { done: 23, last: 'l3-2', at: daysAgo(0) },
  k3: { done: 9, last: 'l2-1', at: daysAgo(2) },
  k7: { done: 16, last: 'l2-1', at: daysAgo(6) }
};

function getLearn() {
  const v = wx.getStorageSync('zx_learn');
  return v && typeof v === 'object' && Object.keys(v).length ? v : JSON.parse(JSON.stringify(DEFAULT_LEARN));
}
function saveLearn(v) { wx.setStorageSync('zx_learn', v); }

function myCourses() {
  const learn = getLearn();
  return Object.keys(learn).map(id => {
    const c = data.courseOf(id);
    const l = learn[id];
    const pct = Math.min(100, Math.round(l.done / c.lessons * 100));
    return { id, title: c.title, cover: c.cover, mark: c.mark, teacher: data.teacherOf(c.teacher).name, lessons: c.lessons,
      done: l.done, pct, at: l.at, last: l.last, finished: pct >= 100 };
  }).sort((a, b) => (a.at < b.at ? 1 : -1));
}

function getFav() { return wx.getStorageSync('zx_fav') || []; }
function saveFav(v) { wx.setStorageSync('zx_fav', v); }

// 打卡：默认连续打了 6 天（不含今天），外加月初零星几天
function getCheckins() {
  const v = wx.getStorageSync('zx_checkin');
  if (Array.isArray(v)) return v;
  const list = [];
  for (let i = 1; i <= 6; i++) list.push(daysAgo(i));
  [9, 10, 12, 15, 16, 17].forEach(n => list.push(daysAgo(n)));
  return list;
}
function saveCheckins(v) { wx.setStorageSync('zx_checkin', v); }
function streak(list) {
  const set = {};
  list.forEach(k => { set[k] = true; });
  let n = 0;
  let i = set[daysAgo(0)] ? 0 : 1;
  while (set[daysAgo(i)]) { n++; i++; }
  return n;
}

module.exports = { getLearn, saveLearn, myCourses, getFav, saveFav, getCheckins, saveCheckins, streak, dayKey, daysAgo };
