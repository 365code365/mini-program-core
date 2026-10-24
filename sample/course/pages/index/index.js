const data = require('../../utils/data.js');
Page({
  data: {
    keyword: '',
    cats: data.categories,
    cat: '全部',
    banners: [
      { title: '秋季开学季 · 全场 5 折起', sub: '精选 300+ 门好课，限时 10 天', btn: '立即抢课', bg: 'linear-gradient(120deg, #5b8cff, #3d4ff0)', id: 'k1' },
      { title: '公考冲刺营火热报名', sub: '王立诚老师直播带练 · 21 天上岸计划', btn: '了解详情', bg: 'linear-gradient(120deg, #a28bff, #6b4ff0)', id: 'k6' },
      { title: '新人专享 0 元学', sub: '手机摄影课限时免费领取', btn: '免费领取', bg: 'linear-gradient(120deg, #ffa98a, #f0607a)', id: 'k7' }
    ],
    teachers: [],
    list: [],
    total: 0
  },
  onLoad() {
    this.setData({ teachers: data.teachers.map(t => Object.assign({ sx: t.name.charAt(0) }, t)) });
    this.refresh();
  },
  refresh() {
    const kw = this.data.keyword.trim().toLowerCase();
    const cat = this.data.cat;
    const list = data.courses.filter(c => (cat === '全部' || c.cat === cat) &&
      (!kw || c.title.toLowerCase().indexOf(kw) >= 0 || data.teacherOf(c.teacher).name.indexOf(kw) >= 0))
      .map(c => Object.assign({}, c, { tname: data.teacherOf(c.teacher).name, stu: data.fmtStudents(c.students), free: c.price === 0 }));
    this.setData({ list, total: list.length });
  },
  onInput(e) { this.setData({ keyword: e.detail.value }); this.refresh(); },
  clearKw() { this.setData({ keyword: '' }); this.refresh(); },
  onCat(e) { this.setData({ cat: e.currentTarget.dataset.c }); this.refresh(); },
  goCourse(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onTeacher(e) {
    const t = data.teachers.find(x => x.id === e.currentTarget.dataset.id);
    wx.showModal({ title: t.name + ' · ' + t.title, content: t.intro, confirmText: '看课程', success: r => {
      if (r.confirm) { this.setData({ cat: t.field, keyword: '' }); this.refresh(); }
    } });
  },
  goExam() { wx.navigateTo({ url: '/pages/exam/exam' }); },
  goCheckin() { wx.switchTab({ url: '/pages/checkin/checkin' }); },
  goLearn() { wx.switchTab({ url: '/pages/learn/learn' }); }
});
