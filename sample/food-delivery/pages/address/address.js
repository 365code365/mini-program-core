const store = require('../../utils/store.js');
const host = require('../../utils/host.js');

Page({
  data: {
    select: false,
    pickedId: '',
    list: []
  },
  onLoad(q) {
    const select = q.select === '1';
    this.setData({ select });
    if (select) wx.setNavigationBarTitle({ title: '选择收货地址' });
  },
  onShow() {
    const cur = store.currentAddr();
    this.setData({
      pickedId: cur ? cur.id : '',
      list: store.getAddrs().map(a => Object.assign({}, a, { text: store.addrText(a), phoneMask: store.maskPhone(a.phone) }))
    });
  },
  // 选择模式：点哪个送哪个（首页定位、结算页共用）；管理模式：点了去编辑
  onPick(e) {
    const id = e.currentTarget.dataset.id;
    if (!this.data.select) {
      this.onEdit(e);
      return;
    }
    store.pickAddr(id);
    wx.navigateBack();
  },
  onEdit(e) {
    wx.navigateTo({ url: '/pages/address-edit/address-edit?id=' + e.currentTarget.dataset.id });
  },
  onAdd() {
    wx.navigateTo({ url: '/pages/address-edit/address-edit' });
  },
  onImport() {
    host.chooseAddress().then(r => {
      if (!r) return;
      const a = store.upsertAddr(Object.assign(r, { tag: '', gender: '', isDefault: false }));
      wx.showToast({ title: '已导入微信地址', icon: 'none' });
      if (this.data.select) {
        store.pickAddr(a.id);
        wx.navigateBack();
        return;
      }
      this.onShow();
    });
  }
});
