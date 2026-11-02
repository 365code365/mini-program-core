const theme = require('./utils/theme.js');

App({
  globalData: { sysTheme: 'light', hiddenAt: 0, catJump: '' },

  onLaunch() {
    try {
      const info = wx.getSystemInfoSync();
      this.globalData.sysTheme = info.theme === 'dark' ? 'dark' : 'light';
    } catch (e) {}
    // 系统切深色 / 浅色：「跟随系统」时当前页立刻换主题
    wx.onThemeChange(res => {
      this.globalData.sysTheme = res.theme === 'dark' ? 'dark' : 'light';
      const pages = getCurrentPages();
      const p = pages[pages.length - 1];
      if (p && typeof p.applyTheme === 'function') p.applyTheme();
    });
    // 切到后台记下时间，回来超过 30 分钟提示一句（睡前听着睡着了的常见场景）
    wx.onAppHide(() => { this.globalData.hiddenAt = Date.now(); });
    wx.onAppShow(() => {
      const t = this.globalData.hiddenAt;
      if (t && Date.now() - t > 30 * 60 * 1000) {
        wx.showToast({ title: '欢迎回来，记得记录昨晚的睡眠', icon: 'none' });
      }
      this.globalData.hiddenAt = 0;
    });
  },

  theme
});
