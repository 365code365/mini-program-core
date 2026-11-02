const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
function text(a) {
  const r = a.region || [];
  return (r[0] === r[1] ? r[0] + (r[2] || '') : r.join('')) + a.detail;
}
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
      list: store.getAddrs().map(a => Object.assign({}, a, { text: text(a), phoneMask: store.maskPhone(a.phone) }))
    });
  },
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
      const a = store.upsertAddr(Object.assign(r, { tag: '', isDefault: false }));
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
