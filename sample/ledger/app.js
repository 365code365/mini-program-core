const store = require('./utils/store.js')

App({
  onLaunch() {
    // 首次打开写入一批示例账单，之后一切增删改都落在 storage 里
    store.ensureSeed()
  },
  globalData: {}
})
