const store = require('../../utils/store.js');
Page({
  data: { cars: [] },
  onShow() { this.setData({ cars: store.getCars() }); },
  onDefault(e) {
    const id = String(e.currentTarget.dataset.id);
    store.setDefaultCar(id);
    this.onShow();
    wx.showToast({ title: '已设为默认车辆', icon: 'none' });
  },
  onEdit(e) { wx.navigateTo({ url: '/pages/vehicle-edit/vehicle-edit?id=' + e.currentTarget.dataset.id }); },
  onRemove(e) {
    const id = String(e.currentTarget.dataset.id);
    const s = store.getSession();
    if (s && s.carId === id) { wx.showToast({ title: '这辆车正在充电，结束后再删除', icon: 'none' }); return; }
    const c = store.findCar(id);
    wx.showModal({
      title: '删除车辆', content: '确定删除 ' + c.plate + ' 吗？', confirmText: '删除', confirmColor: '#F5455C',
      success: r => {
        if (!r.confirm) return;
        store.removeCar(id);
        this.onShow();
        wx.showToast({ title: '已删除', icon: 'none' });
      }
    });
  },
  onAdd() {
    if (this.data.cars.length >= 5) { wx.showToast({ title: '最多绑定 5 辆车', icon: 'none' }); return; }
    wx.navigateTo({ url: '/pages/vehicle-edit/vehicle-edit' });
  }
});
