const S = require('./utils/store.js');
App({
  onLaunch() {
    // 首次打开放演示数据（简历、历史投递、会话、面试邀请），见 utils/store.js
    S.ensureSeed();
  },
  globalData: {}
});
