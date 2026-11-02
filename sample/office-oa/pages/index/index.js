const S = require('../../utils/store.js');
const O = require('../../utils/org.js');
const app = getApp();

const ENTRIES = [
  { id: 'leave', name: '请假', ic: '假', color: '#1677ff', url: '/pages/apply/apply?type=leave' },
  { id: 'expense', name: '报销', ic: '报', color: '#fa8c16', url: '/pages/apply/apply?type=expense' },
  { id: 'seal', name: '用印', ic: '印', color: '#722ed1', url: '/pages/apply/apply?type=seal' },
  { id: 'attend', name: '考勤打卡', ic: '卡', color: '#00b42a', url: '/pages/attend/attend' },
  { id: 'rooms', name: '会议室', ic: '会', color: '#13a8a8', url: '/pages/rooms/rooms' },
  { id: 'approvals', name: '审批中心', ic: '批', color: '#f53f3f', tab: '/pages/approvals/approvals' },
  { id: 'contacts', name: '通讯录', ic: '人', color: '#2f54eb', tab: '/pages/contacts/contacts' },
  { id: 'mine', name: '我的', ic: '我', color: '#86909c', tab: '/pages/mine/mine' }
];
const WEEK = ['日', '一', '二', '三', '四', '五', '六'];

Page({
  data: { me: O.me(), company: O.company, greet: '', dateText: '', todo: {}, today: null, meetings: [], entries: ENTRIES, cols: 4, entryW: '25%' },
  onLoad() {
    // 快捷入口按窗口宽度排列数：窄屏 4 列，平板 / 横屏 / 桌面宽窗 6 列；窗口变了重排
    this.applyWidth(app.globalData.win || wx.getWindowInfo());
    this.offResize = app.onResize(win => this.applyWidth(win));
  },
  onUnload() { this.offResize && this.offResize(); },
  applyWidth(win) {
    const cols = win.width >= 600 || win.windowWidth >= 600 ? 6 : 4;
    this.setData({ cols, entryW: (100 / cols).toFixed(4) + '%' });
  },
  onShow() {
    const list = S.approvals();
    const needMe = list.filter(a => a.needMe).length;
    const tb = this.getTabBar && this.getTabBar();
    if (tb) tb.setData({ selected: 0, badge: needMe });
    const d = new Date();
    const h = d.getHours();
    const greet = h < 6 ? '夜深了' : h < 11 ? '早上好' : h < 13 ? '中午好' : h < 18 ? '下午好' : '晚上好';
    const key = S.ymd(d);
    const rec = S.attend()[key] || null;
    const meetings = S.bookings().filter(b => b.date === key && (b.mine || b.withMe)).sort((a, b) => a.start - b.start);
    this.setData({
      greet, dateText: `${d.getMonth() + 1} 月 ${d.getDate()} 日 星期${WEEK[d.getDay()]}`,
      todo: { needMe, mineIng: list.filter(a => a.mine && a.status === 'pending').length, meetings: meetings.length },
      today: rec, meetings
    });
  },
  goEntry(e) {
    const it = ENTRIES.find(x => x.id === e.currentTarget.dataset.id);
    if (it.tab) wx.switchTab({ url: it.tab }); else wx.navigateTo({ url: it.url });
  },
  goTodo(e) {
    getApp().globalData.apprTab = e.currentTarget.dataset.tab;
    wx.switchTab({ url: '/pages/approvals/approvals' });
  },
  goAttend() { wx.navigateTo({ url: '/pages/attend/attend' }); },
  goRooms() { wx.navigateTo({ url: '/pages/rooms/rooms' }); }
});
