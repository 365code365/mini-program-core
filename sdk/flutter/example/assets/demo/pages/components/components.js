const catalog = require('../../utils/catalog.js');

Page({
  data: {
    groups: catalog.pick(['comp']),
    icons: ['success', 'info', 'warn', 'waiting', 'search', 'download', 'star', 'heart']
  },
  go(e) {
    wx.navigateTo({ url: e.currentTarget.dataset.url });
  }
});
