// 首页：navigationStyle custom 的沉浸头图。导航栏自己画，高度按胶囊（getMenuButtonBoundingClientRect）对齐；
// 往下滚时导航栏从透明渐变到米白，onPageScroll 每 60ms 最多算一次（节流），透明度量化到 0.05 才 setData
const { EXHIBITS, expos, NOTICE, ymd } = require('../../utils/data.js');
const U = require('../../utils/util.js');

const FEATURED = ['e1', 'e3', 'e5', 'e8'];

Page({
  data: {
    statusH: 20, navH: 64, menuTop: 24, menuH: 32, titleRight: 110,
    navAlpha: 0, navDark: false,
    state: {}, leftText: '', expos: [], cur: 0, treasures: [], notice: NOTICE, booked: 0,
    entries: [
      { k: 'book', t: '参观预约', s: '实名 · 分时段', c: '约' },
      { k: 'hall', t: '展厅导览', s: '五大常设展厅', c: '厅' },
      { k: 'cal', t: '展览日历', s: '特展与活动', c: '历' },
      { k: 'fav', t: '我的收藏', s: '收藏与足迹', c: '藏' }
    ]
  },
  onLoad() {
    const sys = wx.getSystemInfoSync();
    const mb = wx.getMenuButtonBoundingClientRect();
    const statusH = sys.statusBarHeight || 20;
    this.setData({
      statusH, menuTop: mb.top, menuH: mb.height,
      navH: mb.bottom + (mb.top - statusH), // 胶囊上下留白对称
      titleRight: sys.windowWidth - mb.left + 8
    });
    // 头图是深色天空：下拉露出的背景与状态栏文字跟着变深 / 变浅
    wx.setBackgroundColor({ backgroundColor: '#f6f2ec', backgroundColorTop: '#1b2a4a', backgroundColorBottom: '#f6f2ec' });
    wx.setBackgroundTextStyle({ textStyle: 'light' });
    this.onScrollThrottled = U.throttle(this.applyScroll, 60);
    this.setData({ treasures: FEATURED.map(id => U.exhibit(id)) });
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    const today = ymd(new Date());
    const list = expos().map(x => Object.assign(x, { status: x.start > today ? '即将开展' : '展出中' }));
    const left = U.isClosed(new Date()) ? 0 : U.slotsFor(today).reduce((a, s) => a + s.left, 0);
    const booked = U.load('bookings', []).filter(b => b.status === '待使用').length;
    const leftText = U.isClosed(new Date()) ? '可预约未来 7 天的场次' : '今日余票 ' + left + ' 张';
    this.setData({ state: U.openState(), leftText, expos: list, booked });
  },
  onPageScroll(e) { this.onScrollThrottled(e.scrollTop); },
  applyScroll(top) {
    const a = Math.round(Math.min(1, Math.max(0, (top - 40) / 160)) * 20) / 20;
    if (a === this.data.navAlpha) return;
    const dark = a > 0.6;
    if (dark !== this.data.navDark) wx.setBackgroundTextStyle({ textStyle: dark ? 'dark' : 'light' });
    this.setData({ navAlpha: a, navDark: dark });
  },
  onSwiper(e) { this.setData({ cur: e.detail.current }); },
  openExpo(e) { wx.navigateTo({ url: '/pages/expo/expo?id=' + e.currentTarget.dataset.id }); },
  openExhibit(e) { wx.navigateTo({ url: '/pages/exhibit/exhibit?id=' + e.currentTarget.dataset.id }); },
  onEntry(e) {
    const k = e.currentTarget.dataset.k;
    if (k === 'book') wx.navigateTo({ url: '/pages/booking/booking' });
    else if (k === 'hall') wx.switchTab({ url: '/pages/halls/halls' });
    else if (k === 'cal') wx.switchTab({ url: '/pages/calendar/calendar' });
    else wx.navigateTo({ url: '/pages/favorites/favorites' });
  },
  goBook() {
    if (!this.data.state.open && U.isClosed(new Date())) wx.showToast({ title: '今日闭馆，可预约明日起的场次', icon: 'none' });
    wx.navigateTo({ url: '/pages/booking/booking' });
  },
  goTickets() { wx.switchTab({ url: '/pages/mine/mine' }); },
  onShareAppMessage() { return { title: '云川博物院：看十大镇馆之宝', path: '/pages/index/index' }; }
});
