const store = require('../../utils/store.js');

Page({
  data: { list: [], select: false },
  onLoad(q) {
    this.setData({ select: q.select === '1' });
    if (q.select === '1') wx.setNavigationBarTitle({ title: '选择收货地址' });
  },
  onShow() { this.setData({ list: store.addresses() }); },
  onPick(e) {
    if (!this.data.select) { this.edit(e); return; }
    const a = this.data.list.find((x) => x.id === e.currentTarget.dataset.id);
    this.getOpenerEventChannel().emit('picked', a);
    wx.navigateBack();
  },
  setDefault(e) {
    const a = this.data.list.find((x) => x.id === e.currentTarget.dataset.id);
    if (a.isDefault) return;
    this.setData({ list: store.saveAddress(Object.assign({}, a, { isDefault: true })) });
    wx.showToast({ title: '已设为默认', icon: 'success' });
  },
  edit(e) { wx.navigateTo({ url: '/pages/address-edit/address-edit?id=' + e.currentTarget.dataset.id }); },
  add() { wx.navigateTo({ url: '/pages/address-edit/address-edit' }); },
  remove(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '删除地址', content: '删除后不可恢复，确定删除吗？', confirmColor: '#e4572e',
      success: (r) => { if (r.confirm) { this.setData({ list: store.removeAddress(id) }); wx.showToast({ title: '已删除', icon: 'none' }); } }
    });
  },
  // 微信地址簿：真机弹授权页；桌面 fail 后转到手动新增
  importWx() {
    wx.chooseAddress({
      success: (r) => {
        this.setData({ list: store.saveAddress({ name: r.userName, phone: r.telNumber, region: [r.provinceName, r.cityName, r.countyName], city: r.cityName, detail: r.detailInfo, tag: '', isDefault: false }) });
        wx.showToast({ title: '已导入', icon: 'success' });
      },
      fail: () => {
        wx.showModal({
          title: '无法读取微信地址', content: '当前环境不支持获取微信收货地址，请手动填写', confirmText: '去填写', showCancel: true,
          success: (r) => { if (r.confirm) this.add(); }
        });
      }
    });
  }
});
