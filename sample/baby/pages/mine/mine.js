const store = require('../../utils/store.js');
const T = require('../../utils/time.js');

Page({
  data: {
    baby: {}, age: '', stats: [], storage: { used: 0, limit: 10240, keys: 0, percent: 0 },
    menu: [
      { key: 'profile', text: '宝宝资料', sub: '乳名、生日、出生信息' },
      { key: 'feed', text: '喂养记录', sub: '母乳计时、奶粉毫升' },
      { key: 'sleep', text: '睡眠记录', sub: '计时、补录、近 7 天' },
      { key: 'vaccine', text: '疫苗计划', sub: '国家免疫规划 + 自费疫苗' },
      { key: 'fav', text: '我的收藏', sub: '收藏的育儿文章' }
    ]
  },

  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 4 });
    this.load();
  },

  load() {
    const baby = store.profile();
    const done = store.get(store.K.vaccines, {});
    const stats = [
      { k: 'feed', n: store.list(store.K.feeds).length, t: '喂养' },
      { k: 'sleep', n: store.list(store.K.sleeps).length, t: '睡眠' },
      { k: 'photo', n: store.list(store.K.moments).length, t: '照片' },
      { k: 'vac', n: Object.keys(done).length, t: '已接种' }
    ];
    const info = wx.getStorageInfoSync();
    const used = Math.round(info.currentSize || 0), limit = info.limitSize || 10240;
    this.setData({
      baby, age: T.age(baby.birthday).label, stats,
      storage: { used, limit, keys: (info.keys || []).length, percent: Math.min(100, Math.max(1, Math.round(used / limit * 100))) }
    });
  },

  go(e) {
    const k = e.currentTarget.dataset.key;
    if (k === 'fav') {
      wx.setStorageSync('kn_open_fav', 1);
      return wx.switchTab({ url: '/pages/knowledge/knowledge' });
    }
    if (k === 'photo') return wx.switchTab({ url: '/pages/album/album' });
    if (k === 'vac') return wx.navigateTo({ url: '/pages/vaccine/vaccine' });
    wx.navigateTo({ url: '/pages/' + k + '/' + k });
  },

  clearRecords() {
    wx.showModal({
      title: '清空所有记录？',
      content: '喂养、睡眠、尿布、身高体重、疫苗与相册记录都会删除，宝宝资料保留。此操作不能撤销。',
      confirmText: '清空', confirmColor: '#f0506e',
      success: (r) => {
        if (!r.confirm) return;
        store.RECORD_KEYS.forEach((k) => wx.removeStorageSync(k));
        wx.vibrateShort({ type: 'heavy' });
        wx.showToast({ title: '已清空', icon: 'none' });
        this.load();
      }
    });
  },

  restore() {
    wx.showModal({
      title: '恢复示例数据？', content: '会用一份示例数据覆盖当前的资料与记录，方便体验各个功能。',
      success: (r) => {
        if (!r.confirm) return;
        store.seed(true);
        wx.showToast({ title: '已恢复', icon: 'success' });
        this.load();
      }
    });
  },

  about() {
    wx.showModal({
      title: '宝贝成长记 v1.2.0',
      content: '生长曲线采用 WHO 儿童生长标准（2006），疫苗程序依据国家免疫规划（2021 年版）。所有记录只保存在本机。',
      showCancel: false
    });
  }
});
