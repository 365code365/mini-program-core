const store = require('../../utils/store.js');
const theme = require('../../utils/theme.js');

Page({
  data: {
    theme: 'light', mode: 'light', settings: {}, stats: {}, streak: {}, favN: 0, moodN: 0, doneChapters: 0, recent: [],
    themeOpts: [{ value: 'light', name: '浅色' }, { value: 'dark', name: '深色' }, { value: 'system', name: '跟随系统' }]
  },

  onShow() {
    this.applyTheme();
    const done = store.doneMap();
    const recent = store.sessions().slice(-5).reverse().map(s => Object.assign({}, s, {
      durText: s.sec >= 60 ? Math.round(s.sec / 60) + ' 分钟' : s.sec + ' 秒'
    }));
    const settings = store.settings();
    this.setData({
      settings, mode: settings.theme, stats: store.stats(), streak: store.streakInfo(), favN: store.favs().length,
      moodN: store.moods().length, doneChapters: Object.keys(done).reduce((a, k) => a + done[k].length, 0), recent
    });
  },

  applyTheme() { return theme.apply(this, 3); },

  onTheme(e) {
    store.saveSettings({ theme: e.detail.value });
    this.setData({ mode: e.detail.value });
    const t = this.applyTheme();
    wx.showToast({ title: e.detail.value === 'system' ? '已跟随系统' : (t === 'dark' ? '已切换深色模式' : '已切换浅色模式'), icon: 'none' });
  },

  onNoiseHide(e) {
    this.setData({ settings: store.saveSettings({ noiseOnHide: e.detail.value }) });
  },

  goFav() {
    getApp().globalData.catJump = 'fav';
    wx.switchTab({ url: '/pages/category/category' });
  },

  goMood() { wx.navigateTo({ url: '/pages/mood/mood' }); },
  goBreath() { wx.navigateTo({ url: '/pages/breath/breath' }); },

  clearData() {
    wx.showModal({
      title: '清除本地数据',
      content: '练习记录、打卡、收藏、睡眠与心情日记都会被删除，且无法恢复。',
      confirmText: '清除',
      confirmColor: '#e0604f',
      success: res => {
        if (!res.confirm) return;
        store.clearAll();
        wx.showToast({ title: '已清除', icon: 'success' });
        this.onShow();
      }
    });
  }
});
