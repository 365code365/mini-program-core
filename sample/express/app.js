App({
  onLaunch() {
    // 选地址页回传用：寄件页 onShow 读这里
    this.globalData.pickedAddress = null
  },
  globalData: {
    pickedAddress: null
  }
})
