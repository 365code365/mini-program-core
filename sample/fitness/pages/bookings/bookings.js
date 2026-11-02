const store = require('../../utils/store.js');
const flows = require('../../utils/flows.js');
const TABS = [{ key: 'todo', name: '待上课' }, { key: 'done', name: '已上课' }, { key: 'off', name: '已取消 / 爽约' }];
const IN_TAB = { todo: ['booked', 'wait'], done: ['done'], off: ['cancel', 'miss'] };
const STATUS = { booked: '待上课', wait: '候补中', done: '已上课', miss: '爽约', cancel: '已取消' };
// 我的团课预约：签到、取消、候补
Page({
  data: { tabs: TABS, tab: 'todo', list: [], strikes: 0 },
  onLoad(q) { if (IN_TAB[q.tab]) this.setData({ tab: q.tab }); },
  onShow() { this.refresh(); },
  refresh() {
    store.tick();
    const keys = IN_TAB[this.data.tab];
    const l = store.getBookings().filter(b => keys.indexOf(b.status) >= 0)
      .sort((a, b) => (this.data.tab === 'todo' ? store.startOf(a) - store.startOf(b) : store.startOf(b) - store.startOf(a)));
    this.setData({
      strikes: store.strikes().length,
      list: l.map(b => Object.assign({}, b, {
        statusText: b.status === 'wait' ? '候补第 ' + b.rank + ' 位' : (b.status === 'cancel' && b.late ? '已取消 · 记爽约' : (b.waitFail ? '候补未成功' : STATUS[b.status])),
        when: (b.date === store.today() ? '今天' : b.date.slice(5)) + ' ' + b.time + ' · ' + b.dur + ' 分钟',
        canSign: store.signState(b),
        canCancel: (b.status === 'booked' || b.status === 'wait') && Date.now() < store.startOf(b)
      }))
    });
  },
  setTab(e) {
    this.setData({ tab: e.currentTarget.dataset.k });
    this.refresh();
  },
  find(e) { return store.findBooking(e.currentTarget.dataset.id); },
  onCancel(e) { flows.cancel(this.find(e)).then(() => this.refresh()); },
  onSign(e) { flows.sign(this.find(e)).then(() => this.refresh()); },
  goCourse(e) {
    const b = this.find(e);
    const day = store.daysBetween(store.today(), b.date);
    if (day >= 0 && day < 7) wx.navigateTo({ url: '/pages/course/course?id=' + b.courseId + '&day=' + day });
    else wx.showToast({ title: '这节课已经不在近 7 天的课表里了', icon: 'none' });
  },
  goClasses() { wx.switchTab({ url: '/pages/classes/classes' }); }
});
