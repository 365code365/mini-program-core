const store = require('./utils/store.js');

App({
  onLaunch() {
    // 首次启动写入示例数据（批量写 storage），之后都读本地
    store.seed(false);
  },
  globalData: {}
});
