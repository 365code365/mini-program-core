const S = require('../../utils/store.js');
const app = getApp();

Page({
  data: { stats: { consult: 0, active: 0, draft: 0, fav: 0 }, recent: null, statusText: '' },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 3 });
    const list = S.consults();
    this.setData({
      stats: { consult: list.length, active: list.filter(c => c.status === 'active').length, draft: S.drafts().length, fav: S.favs().length },
      recent: list[0] || null,
      statusText: list[0] ? S.STATUS[list[0].status] : ''
    });
  },
  goConsults(e) { wx.navigateTo({ url: '/pages/consults/consults?tab=' + (e.currentTarget.dataset.tab || 'all') }); },
  goRecent() { wx.navigateTo({ url: '/pages/chat/chat?id=' + this.data.recent.id }); },
  goDrafts() { wx.switchTab({ url: '/pages/contracts/contracts' }); },
  goFavs() {
    app.globalData.lawyerCat = '';
    app.globalData.lawyerFav = true;
    wx.switchTab({ url: '/pages/lawyers/lawyers' });
  },
  goCalc() { wx.navigateTo({ url: '/pages/calc/calc' }); },
  callService() {
    wx.makePhoneCall({
      phoneNumber: '4008206688',
      fail: () => wx.showModal({
        title: '客服热线 400-820-6688', content: '工作日 9:00-21:00。当前环境无法直接拨号，是否复制号码？', confirmText: '复制',
        success: r => { if (r.confirm) wx.setClipboardData({ data: '4008206688' }); }
      })
    });
  },
  about() {
    wx.showModal({ title: '免责声明', content: app.globalData.disclaimer + '平台律师均经执业证核验，咨询费用由平台代收，未回复全额退款。', showCancel: false, confirmText: '我知道了' });
  },
  clearData() {
    wx.showModal({
      title: '清除本地数据', content: '将删除咨询记录、合同草稿和收藏，且不可恢复。', confirmText: '清除', confirmColor: '#d64545',
      success: r => {
        if (!r.confirm) return;
        ['legal_consults', 'legal_drafts', 'legal_favs', 'legal_helpful'].forEach(k => wx.removeStorageSync(k));
        this.onShow();
        wx.showToast({ title: '已清除', icon: 'success' });
      }
    });
  }
});
