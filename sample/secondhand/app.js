const store = require('./utils/store.js');
App({
  onLaunch() {
    store.seed();
    // 全局打开分享菜单（转发给朋友 + 分享到朋友圈），详情页再按宝贝改标题
    wx.showShareMenu({ withShareTicket: true, menus: ['shareAppMessage', 'shareTimeline'] });
  },
  globalData: {}
});
