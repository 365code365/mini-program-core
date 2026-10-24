App({
  // 记一次「最近使用」，首页按时间倒序展示
  touchTool(id) {
    let recent = wx.getStorageSync('tb_recent') || []
    recent = [id].concat(recent.filter(x => x !== id)).slice(0, 4)
    wx.setStorageSync('tb_recent', recent)
  }
})
