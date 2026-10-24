const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
function lessonTitle(c, lid) {
  for (const ch of c.chapters) for (const l of ch.lessons) if (l.id === lid) return l.title;
  return c.chapters[0].lessons[0].title;
}
Page({
  data: { filter: 'ing', list: [], cur: null, todayMins: 46, goal: 60, goalPct: 0, totalHours: 0, courseCount: 0, doneCount: 0 },
  onShow() { this.refresh(); },
  refresh() {
    const all = store.myCourses();
    const cur = all.find(x => !x.finished) || null;
    if (cur) cur.lastTitle = lessonTitle(data.courseOf(cur.id), cur.last);
    const totalMins = all.reduce((s, x) => s + x.done * 21, 0);
    const todayMins = wx.getStorageSync('zx_today') || 46;
    this.setData({
      cur,
      list: all.filter(x => (this.data.filter === 'ing' ? !x.finished : x.finished)),
      courseCount: all.length,
      doneCount: all.filter(x => x.finished).length,
      totalHours: (totalMins / 60).toFixed(1),
      todayMins,
      goalPct: Math.min(100, Math.round(todayMins / this.data.goal * 100))
    });
  },
  onFilter(e) { this.setData({ filter: e.currentTarget.dataset.f }); this.refresh(); },
  // 继续学习：学完一节，进度 +1
  study(e) {
    const id = e.currentTarget.dataset.id;
    const c = data.courseOf(id);
    const learn = store.getLearn();
    const l = learn[id];
    if (l.done >= c.lessons) { wx.showToast({ title: '这门课已经学完啦', icon: 'none' }); return; }
    l.done += 1;
    l.at = store.daysAgo(0);
    const flat = [];
    c.chapters.forEach(ch => ch.lessons.forEach(x => flat.push(x.id)));
    l.last = flat[Math.min(flat.length - 1, flat.indexOf(l.last) + 1)] || l.last;
    store.saveLearn(learn);
    wx.setStorageSync('zx_today', (wx.getStorageSync('zx_today') || 46) + 21);
    wx.showToast({ title: '已学完 1 节，继续加油', icon: 'none' });
    this.refresh();
  },
  goDetail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  goExam() { wx.navigateTo({ url: '/pages/exam/exam' }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
