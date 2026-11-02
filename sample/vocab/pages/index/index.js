const store = require('../../utils/store.js');
const books = require('../../utils/books.js');
const D = require('../../utils/date.js');
const tab = require('../../utils/tab.js');

Page({
  data: {
    book: null, learned: 0, total: 0, bookPct: 0,
    todayNew: 0, plan: 8, due: 0, stars: 0, streak: 0, checked: false, canCheck: false,
    remindTip: '', greeting: '', dateText: ''
  },
  onShow() {
    tab.sync(this, 0);
    this.refresh();
  },
  refresh() {
    const s = store.settings();
    const b = books.book(s.book);
    const all = books.words(b.id).length;
    const learned = store.learnedOf(b.id).length;
    const tl = store.todayLog();
    const left = store.newWords(b.id, 1).length;
    const checked = store.checkedToday();
    // 今天新词学满计划（或这本书已经学完）且没有到期复习，才能打卡
    const due = store.dueList().length;
    const canCheck = !checked && (tl.learn >= s.plan || left === 0) && due === 0;
    const h = new Date().getHours();
    let remindTip = '';
    if (s.remindOn && !checked && D.hm() >= s.remind) remindTip = '已过 ' + s.remind + ' 提醒时间，今天还没打卡哦';
    this.setData({
      book: b, learned, total: all, bookPct: Math.round(learned * 100 / all),
      todayNew: Math.min(tl.learn, s.plan), plan: s.plan, due, stars: store.stars().length,
      streak: store.streak(), checked, canCheck, remindTip,
      greeting: h < 11 ? '早上好' : h < 14 ? '中午好' : h < 18 ? '下午好' : '晚上好',
      dateText: D.today().slice(5).replace('-', '月') + '日 ' + D.weekday(D.today())
    });
  },
  goLearn() { wx.navigateTo({ url: '/pages/learn/learn' }); },
  goReview() { wx.switchTab({ url: '/pages/review/review' }); },
  goQuiz() { wx.navigateTo({ url: '/pages/quiz/quiz?src=book' }); },
  goBooks() { wx.navigateTo({ url: '/pages/books/books' }); },
  goCalendar() { wx.navigateTo({ url: '/pages/calendar/calendar' }); },
  goStats() { wx.navigateTo({ url: '/pages/stats/stats' }); },
  goWrong() { wx.navigateTo({ url: '/pages/wrong/wrong' }); },
  onCheck() {
    if (this.data.checked) { this.goCalendar(); return; }
    if (!this.data.canCheck) {
      const msg = this.data.due > 0 ? '还有 ' + this.data.due + ' 个词待复习，复习完再来打卡' : '今天的新词还差 ' + (this.data.plan - this.data.todayNew) + ' 个';
      wx.showModal({ title: '还不能打卡', content: msg, confirmText: this.data.due > 0 ? '去复习' : '去学习', success: (r) => {
        if (r.confirm) { if (this.data.due > 0) this.goReview(); else this.goLearn(); }
      } });
      return;
    }
    const n = store.checkin();
    wx.vibrateShort({ type: 'medium' });
    wx.showToast({ title: '打卡成功，连续 ' + n + ' 天', icon: 'success' });
    this.refresh();
  }
});
