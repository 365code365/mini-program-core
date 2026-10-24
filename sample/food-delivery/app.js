App({
  globalData: {
    city: '北京',
    location: '望京SOHO T3',
    addressId: 'a1'
  },
  onLaunch() {
    const addr = wx.getStorageSync('fd_address');
    if (addr) this.globalData.addressId = addr;
  }
});
