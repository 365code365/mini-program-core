const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { filter: 'ing', list: [], cur: null, todayMins: 46, goal: 60, goalPct: 0, totalHours: 0, courseCount: 0, doneCount: 0 },
  onShow() { if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 }); this.refresh(); },
  refresh() {
    const all = store.myCourses();
    const cur = all.find(x => !x.finished) || null;
    if (cur) {
      const flat = data.flatLessons(data.courseOf(cur.id));
      const i = Math.max(0, flat.findIndex(l => l.id === cur.last));
      cur.lastTitle = flat[i].title;
      cur.nextNo = i + 1;
    }
    const totalMins = store.learnedMins();
    const todayMins = store.todayMins();
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
  // 继续学习：打开下一节没学完的课时
  study(e) {
    const id = String(e.currentTarget.dataset.id);
    wx.navigateTo({ url: '/pages/lesson/lesson?cid=' + id + '&lid=' + store.progress(id).last });
  },
  goDetail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  goExam() { wx.navigateTo({ url: '/pages/exam/exam' }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
