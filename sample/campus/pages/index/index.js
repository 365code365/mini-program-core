const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
const kit = require('../../utils/kit.js');
const GRID = [
  { key: 'timetable', name: '课程表', ch: '课', bg: '#5b8def' }, { key: 'grades', name: '成绩', ch: '绩', bg: '#14937f' },
  { key: 'library', name: '图书馆', ch: '书', bg: '#f08a4b' }, { key: 'card', name: '校园卡', ch: '卡', bg: '#a46be0' },
  { key: 'rooms', name: '空教室', ch: '室', bg: '#3aa6c8' }, { key: 'notices', name: '通知', ch: '告', bg: '#e05a7a' },
  { key: 'calendar', name: '校历', ch: '历', bg: '#c49a1a' }, { key: 'phone', name: '常用电话', ch: '话', bg: '#6b8e23' }
];
Page({
  data: { grid: GRID, week: 1, today: '', todayCourses: [], notices: [], greeting: '', balance: '0.00' },
  onShow() {
    store.syncTab(this, 0);
    const now = new Date();
    const w = D.weekOf(now), day = now.getDay() || 7, h = now.getHours();
    const nowSec = h * 60 + now.getMinutes();
    const todayCourses = D.coursesOn(day, w).map(c => {
      const [sh, sm] = D.SECTIONS[c.start - 1].split(':').map(Number);
      const begin = sh * 60 + sm, end = begin + c.len * 50;
      return Object.assign({}, c, {
        timeText: '第' + c.start + '-' + (c.start + c.len - 1) + '节 ' + D.SECTIONS[c.start - 1],
        state: nowSec >= end ? '已结束' : nowSec >= begin ? '上课中' : ''
      });
    });
    const read = store.readSet();
    this.setData({
      week: w, today: kit.fmt(now, 'MM月DD日') + ' ' + kit.WEEK[now.getDay()], todayCourses,
      greeting: h < 11 ? '早上好' : h < 13 ? '中午好' : h < 18 ? '下午好' : '晚上好',
      notices: D.NOTICES.slice(0, 3).map(n => Object.assign({}, n, { unread: read.indexOf(n.id) < 0 })),
      balance: kit.money(store.card().balance)
    });
  },
  go(e) {
    const k = e.currentTarget.dataset.key;
    if (k === 'timetable' || k === 'notices') wx.switchTab({ url: '/pages/' + k + '/' + k });
    else if (k === 'calendar') wx.showModal({ title: '2026-2027 学年第一学期校历', content: '9 月 7 日开学上课\n10 月 1-7 日国庆放假\n第 9-10 周期中教学检查\n第 17-18 周期末考试\n2027 年 1 月 11 日寒假开始', showCancel: false });
    else if (k === 'phone') wx.showActionSheet({ itemList: ['教务处 0838-6612001', '保卫处 0838-6612110', '校医院 0838-6612120', '后勤报修 0838-6612233'], success: r => kit.call(['0838-6612001', '0838-6612110', '0838-6612120', '0838-6612233'][r.tapIndex]) });
    else wx.navigateTo({ url: '/pages/' + k + '/' + k });
  },
  openNotice(e) { wx.navigateTo({ url: '/pages/notice/notice?id=' + e.currentTarget.dataset.id }); },
  goTimetable() { wx.switchTab({ url: '/pages/timetable/timetable' }); }
});
