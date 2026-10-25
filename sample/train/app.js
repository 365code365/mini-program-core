App({
  globalData: {
    // 城市选择页回传：{ field: 'from' | 'to', name }，首页 onShow 取走
    pickedCity: null,
    // 车次列表点进选座页时带过去的整趟车（字段多，不走 url 参数）
    train: null
  }
})
