App({
  onLaunch() {
    // 首次启动给两位默认观演人，便于直接走通购票
    if (!wx.getStorageSync('ls_viewers')) {
      wx.setStorageSync('ls_viewers', [{ id: 'v1', name: '陈思远', type: '身份证', no: '310104199203154216', mask: '3101**********4216' },
        { id: 'v2', name: '林可', type: '身份证', no: '330106199508263022', mask: '3301**********3022' }]);
    }
  },
  globalData: { city: '上海' }
});
