App({
  onLaunch() {
    // 选地址页回传用：寄件页 onShow 读这里
    this.globalData.pickedAddress = null
  },
  // pickedAddress：选地址页回传；resend：「再寄一单」带给寄件页；filter：「我的 → 在途」带给首页的筛选
  globalData: {
    pickedAddress: null,
    resend: null,
    filter: ''
  }
})
