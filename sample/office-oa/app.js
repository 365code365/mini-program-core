// 云协同：企业 OA。窗口尺寸统一在这里读一次，变化时通知各页（工作台按宽度排快捷入口列数）。
App({
  globalData: { win: null, sys: null, resizeCbs: [] },
  onLaunch() {
    const sys = wx.getSystemInfoSync();
    const win = wx.getWindowInfo ? wx.getWindowInfo() : sys;
    this.globalData.sys = sys;
    this.globalData.win = { width: win.windowWidth, height: win.windowHeight };
    wx.onWindowResize && wx.onWindowResize(res => {
      const s = res.size || {};
      this.globalData.win = { width: s.windowWidth, height: s.windowHeight };
      this.globalData.resizeCbs.forEach(cb => { try { cb(this.globalData.win); } catch (e) { console.error(e); } });
    });
  },
  onResize(cb) {
    this.globalData.resizeCbs.push(cb);
    return () => { this.globalData.resizeCbs = this.globalData.resizeCbs.filter(f => f !== cb); };
  }
});
