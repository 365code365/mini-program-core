const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { t: null, list: [], followed: false },
  onLoad(q) {
    const t = data.teacherOf(q.id) || data.teachers[0];
    const list = data.courses.filter(c => c.teacher === t.id).map(c => ({ id: c.id, title: c.title, cover: c.cover, mark: c.mark, price: c.price, lessons: c.lessons, stu: data.fmtStudents(c.students), owned: store.owned(c.id) }));
    this.setData({ t: Object.assign({ sx: t.name.charAt(0) }, t), list });
    wx.setNavigationBarTitle({ title: t.name });
  },
  onShow() { if (this.data.t) this.setData({ followed: store.getFollow().indexOf(this.data.t.id) >= 0 }); },
  follow() {
    const id = this.data.t.id;
    let f = store.getFollow();
    if (f.indexOf(id) >= 0) f = f.filter(x => x !== id); else f.push(id);
    store.saveFollow(f);
    this.setData({ followed: f.indexOf(id) >= 0 });
    wx.showToast({ title: this.data.followed ? '已关注，开课会通知你' : '已取消关注', icon: 'none' });
  },
  goCourse(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); }
});
