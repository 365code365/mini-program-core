const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { c: null, t: null, chapters: [], reviews: [], owned: false, fav: false, stu: '', tab: 'intro', lessonTotal: 0, minsTotal: 0 },
  onLoad(q) {
    const id = q.id || 'k1';
    const c = data.courseOf(id) || data.courses[0];
    const t = data.teacherOf(c.teacher);
    let minsTotal = 0, lessonTotal = 0;
    const chapters = c.chapters.map((ch, i) => {
      ch.lessons.forEach(l => { minsTotal += l.mins; lessonTotal++; });
      return Object.assign({}, ch, { open: i === 0, count: ch.lessons.length });
    });
    const reviews = data.reviews.map(r => Object.assign({ sx: r.name.charAt(0), stars: [1, 2, 3, 4, 5].map(n => n <= r.score) }, r));
    this.setData({ c, t: Object.assign({ sx: t.name.charAt(0) }, t), chapters, reviews, stu: data.fmtStudents(c.students), lessonTotal, minsTotal });
    wx.setNavigationBarTitle({ title: c.title.length > 12 ? c.title.slice(0, 12) + '…' : c.title });
    this.syncState();
  },
  syncState() {
    const id = this.data.c.id;
    this.setData({ owned: !!store.getLearn()[id], fav: store.getFav().indexOf(id) >= 0 });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); },
  toggle(e) {
    const i = e.currentTarget.dataset.i;
    this.setData({ ['chapters[' + i + '].open']: !this.data.chapters[i].open });
  },
  onLesson(e) {
    const free = e.currentTarget.dataset.free;
    if (this.data.owned || free) wx.showToast({ title: '开始播放：' + e.currentTarget.dataset.title, icon: 'none' });
    else wx.showToast({ title: '购买后即可学习全部课时', icon: 'none' });
  },
  toggleFav() {
    const id = this.data.c.id;
    let fav = store.getFav();
    if (fav.indexOf(id) >= 0) fav = fav.filter(x => x !== id); else fav.unshift(id);
    store.saveFav(fav);
    this.setData({ fav: fav.indexOf(id) >= 0 });
    wx.showToast({ title: this.data.fav ? '已收藏' : '已取消收藏', icon: 'none' });
  },
  consult() {
    wx.showActionSheet({ itemList: ['在线咨询课程顾问', '拨打 400-820-6688', '加入课程交流群'], success: r => {
      wx.showToast({ title: ['顾问稍后联系您', '正在呼叫…', '已复制群号 682 113 905'][r.tapIndex], icon: 'none' });
    } });
  },
  buy() {
    const c = this.data.c;
    if (this.data.owned) { wx.switchTab({ url: '/pages/learn/learn' }); return; }
    const content = c.price === 0 ? '确认免费领取「' + c.title + '」？' : '「' + c.title + '」\n实付 ¥' + c.price + '（已优惠 ¥' + (c.origin - c.price) + '）';
    wx.showModal({ title: c.price === 0 ? '免费领取' : '确认购买', content, confirmText: c.price === 0 ? '领取' : '支付', success: r => {
      if (!r.confirm) return;
      const learn = store.getLearn();
      learn[c.id] = { done: 0, last: c.chapters[0].lessons[0].id, at: store.daysAgo(0) };
      store.saveLearn(learn);
      this.setData({ owned: true });
      wx.showToast({ title: c.price === 0 ? '领取成功' : '购买成功', icon: 'success' });
    } });
  }
});
