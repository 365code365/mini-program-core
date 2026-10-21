const catalog = require('../../utils/catalog.js');

// 列几个有代表性的：前面是已实现的，后面是故意保持 undefined 的（见引擎测试说明 G-10）
const PROBE = ['request', 'createSelectorQuery', 'createAnimation', 'previewImage', 'login',
  'connectSocket', 'createInnerAudioContext', 'getFileSystemManager'];

Page({
  data: {
    groups: catalog.pick(['api']),
    probes: []
  },
  onLoad() {
    this.setData({ probes: PROBE.map((name) => ({ name, ok: typeof wx[name] === 'function' })) });
  },
  go(e) {
    wx.navigateTo({ url: e.currentTarget.dataset.url });
  }
});
