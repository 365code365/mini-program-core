const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { c: null, t: null, chapters: [], reviews: [], owned: false, fav: false, stu: '', tab: 'intro', lessonTotal: 0, minsTotal: 0,
    pg: null, mine: null, panel: false, score: 5, stars5: [1, 2, 3, 4, 5], rvText: '', formSeq: 0 },
  onLoad(q) {
    const id = q.id || 'k1';
    const c = data.courseOf(id) || data.courses[0];
    const t = data.teacherOf(c.teacher);
    this.setData({ c, t: Object.assign({ sx: t.name.charAt(0) }, t), stu: data.fmtStudents(c.students), lessonTotal: c.lessons, minsTotal: c.mins, tab: q.tab || 'intro' });
    wx.setNavigationBarTitle({ title: c.title.length > 12 ? c.title.slice(0, 12) + '…' : c.title });
  },
  // 从订单 / 课时页回来：已购、进度、收藏、我的评价都要重读
  onShow() { this.syncState(); },
  syncState() {
    const c = this.data.c;
    const owned = store.owned(c.id);
    const p = store.progress(c.id);
    const opened = this.data.chapters.map(ch => ch.open);
    const chapters = c.chapters.map((ch, i) => Object.assign({}, ch, {
      open: opened.length ? opened[i] : i === 0, count: ch.lessons.length,
      lessons: ch.lessons.map(l => Object.assign({}, l, { done: p.doneIds.indexOf(l.id) >= 0 }))
    }));
    const mr = store.getMyReviews()[c.id];
    const mine = mr ? { name: '我', sx: '我', color: '#3d6cf6', time: mr.time, text: mr.text, score: mr.score, stars: [1, 2, 3, 4, 5].map(n => n <= mr.score) } : null;
    const reviews = data.reviews.map(r => Object.assign({ sx: r.name.charAt(0), stars: [1, 2, 3, 4, 5].map(n => n <= r.score) }, r));
    this.setData({ owned, fav: store.getFav().indexOf(c.id) >= 0, chapters, pg: owned ? p : null, mine, reviews });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); },
  toggle(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ ['chapters[' + i + '].open']: !this.data.chapters[i].open });
  },
  onLesson(e) {
    const ds = e.currentTarget.dataset;
    const lid = String(ds.id);
    if (this.data.owned || ds.free) { wx.navigateTo({ url: '/pages/lesson/lesson?cid=' + this.data.c.id + '&lid=' + lid }); return; }
    wx.showModal({ title: '购买后解锁', content: '「' + ds.title + '」需要购买课程后学习，第一章前两节可以免费试看', confirmText: '去购买', success: r => {
      if (r.confirm) this.buy();
    } });
  },
  goTeacher() { wx.navigateTo({ url: '/pages/teacher/teacher?id=' + this.data.t.id }); },
  toggleFav() {
    const id = this.data.c.id;
    let fav = store.getFav();
    if (fav.indexOf(id) >= 0) fav = fav.filter(x => x !== id); else fav.unshift(id);
    store.saveFav(fav);
    this.setData({ fav: fav.indexOf(id) >= 0 });
    wx.showToast({ title: this.data.fav ? '已收藏' : '已取消收藏', icon: 'none' });
  },
  consult() {
    wx.showActionSheet({ itemList: ['拨打课程顾问 400-820-6688', '复制顾问微信号', '复制课程交流群号'], success: r => {
      if (r.tapIndex === 0) host.call('400-820-6688');
      else if (r.tapIndex === 1) host.copy('zhixue_kf01', '微信号已复制，去微信添加好友');
      else host.copy('682113905', '群号已复制');
    } });
  },
  buy() {
    const c = this.data.c;
    if (this.data.owned) { wx.navigateTo({ url: '/pages/lesson/lesson?cid=' + c.id }); return; }
    wx.navigateTo({ url: '/pages/order/order?id=' + c.id });
  },
  // ---------- 写评价 ----------
  openReview() {
    if (!this.data.owned) { wx.showToast({ title: '购买后才能评价', icon: 'none' }); return; }
    const m = this.data.mine;
    this.setData({ panel: true, score: m ? m.score : 5, rvText: m ? m.text : '', formSeq: this.data.formSeq + 1 });
  },
  closeReview() { this.setData({ panel: false }); },
  noop() {},
  onStar(e) { this.setData({ score: Number(e.currentTarget.dataset.n) }); },
  onRv(e) { this.setData({ rvText: e.detail.value }); },
  submitReview() {
    const text = this.data.rvText.trim();
    if (text.length < 5) { wx.showToast({ title: '至少写 5 个字', icon: 'none' }); return; }
    store.saveMyReview(this.data.c.id, { score: this.data.score, text, time: store.today() });
    this.setData({ panel: false });
    this.syncState();
    wx.showToast({ title: '评价已发布', icon: 'success' });
  }
});
