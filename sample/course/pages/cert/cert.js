const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
// 证书：学完一门课自动发放（编号按课程 + 学号生成，可复现）
function certNo(cid) { let h = 7; ('ZX20240318' + cid).split('').forEach(ch => { h = (h * 31 + ch.charCodeAt(0)) % 1000000; }); return 'ZXC-' + cid.toUpperCase() + '-' + ('00000' + h).slice(-6); }
Page({
  data: { certs: [], ing: [], focus: '' },
  onLoad(q) { this.setData({ focus: q.id || '' }); },
  onShow() {
    const mine = store.myCourses();
    const learn = store.getLearn();
    this.setData({
      certs: mine.filter(x => x.finished).map(x => ({ id: x.id, title: x.title, teacher: x.teacher, no: certNo(x.id), date: learn[x.id].at, lessons: x.lessons })),
      ing: mine.filter(x => !x.finished).map(x => ({ id: x.id, title: x.title, left: x.lessons - x.done, pct: x.pct }))
    });
  },
  copy(e) { host.copy(String(e.currentTarget.dataset.no), '证书编号已复制，可在官网验证'); },
  study(e) { wx.navigateTo({ url: '/pages/lesson/lesson?cid=' + e.currentTarget.dataset.id }); }
});
