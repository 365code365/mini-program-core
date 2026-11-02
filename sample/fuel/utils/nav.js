// 导航到油站：真机 wx.openLocation 拉起地图 App；桌面直接 fail，退成「复制地址 / 拨打电话」。
function openStation(st) {
  wx.openLocation({
    latitude: st.lat, longitude: st.lng, name: st.name, address: st.address, scale: 16,
    fail: () => {
      wx.showActionSheet({
        alertText: `无法打开地图：${st.address}`,
        itemList: ['复制地址', '拨打油站电话 ' + st.phone],
        success: (r) => {
          if (r.tapIndex === 0) wx.setClipboardData({ data: st.address });
          else wx.makePhoneCall({ phoneNumber: st.phone, fail: () => wx.showModal({ title: st.name, content: '油站电话：' + st.phone, showCancel: false }) });
        }
      });
    }
  });
}

module.exports = { openStation };
