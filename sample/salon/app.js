// 型格美业：门店信息、项目与发型师、在线预约（选项目 → 发型师 → 时段 → 确认）、会员储值
App({
  globalData: {
    // 切 tab 时带的筛选条件（switchTab 不能带 query）
    stylistFilter: '',
    serviceCat: ''
  },
  onLaunch() {
    // 过了时间还没核销的预约，不自动改状态：门店以前台核销为准，这里只做提示
    const list = wx.getStorageSync('salon_appts') || [];
    this.globalData.upcoming = list.filter(a => a.status === 'upcoming').length;
  }
});
