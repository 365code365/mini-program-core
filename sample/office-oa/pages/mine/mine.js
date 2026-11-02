const S = require('../../utils/store.js');
const O = require('../../utils/org.js');
const ST = require('../../utils/stats.js');
const app = getApp();

Page({
  data: { me: O.me(), company: O.company, stat: null, counts: {}, dev: {}, win: {} },
  onLoad() {
    this.offResize = app.onResize(win => this.setData({ win }));
  },
  onUnload() { this.offResize && this.offResize(); ST.stop(); },
  onShow() {
    const list = S.approvals();
    const tb = this.getTabBar && this.getTabBar();
    if (tb) tb.setData({ selected: 3, badge: list.filter(a => a.needMe).length });
    const today = S.ymd(new Date());
    const sys = wx.getSystemInfoSync();
    const win = wx.getWindowInfo ? wx.getWindowInfo() : sys;
    this.setData({
      counts: {
        mine: list.filter(a => a.mine).length, ing: list.filter(a => a.mine && a.status === 'pending').length,
        book: S.bookings().filter(b => b.mine && b.date >= today).length
      },
      dev: { model: sys.model || sys.brand || '未知设备', system: sys.system || sys.platform, sdk: sys.SDKVersion || '—', pixel: sys.pixelRatio },
      win: { width: win.windowWidth, height: win.windowHeight }
    });
    ST.compute(today.slice(0, 7), S.attend(), today).then(r => this.setData({ stat: r.stats }));
  },
  goMine() { app.globalData.apprTab = 'mine'; wx.switchTab({ url: '/pages/approvals/approvals' }); },
  goAttend() { wx.navigateTo({ url: '/pages/attend/attend' }); },
  goRooms() { wx.navigateTo({ url: '/pages/rooms/rooms' }); },
  goCard() { wx.navigateTo({ url: '/pages/employee/employee?id=' + O.ME }); },
  clear() {
    wx.showModal({
      title: '清除本地数据', content: '审批、考勤、预订会恢复成初始演示数据。', confirmText: '清除', confirmColor: '#f53f3f',
      success: r => {
        if (!r.confirm) return;
        S.clearAll();
        wx.removeStorageSync('oa_modal_no_editable');
        this.onShow();
        wx.showToast({ title: '已清除', icon: 'success' });
      }
    });
  },
  logout() {
    wx.showModal({ title: '退出登录', content: '退出后需要重新使用企业账号登录。', success: r => { if (r.confirm) wx.showToast({ title: '演示环境不支持退出', icon: 'none' }); } });
  }
});
