App({
  globalData: { brand: '田园直供' },
  onLaunch() {
    // 首次启动给一条示范地址，确认订单页能直接下单；用户可在地址管理里改删
    if (!wx.getStorageSync('farm_addr_inited')) {
      wx.setStorageSync('farm_addr', [{ id: 'a1', name: '林晓', phone: '13867425190', region: ['浙江省', '杭州市', '西湖区'], city: '杭州市', detail: '文三路 478 号华星时代广场 A 座 1203', tag: '公司', isDefault: true }]);
      wx.setStorageSync('farm_addr_inited', 1);
    }
  }
});
