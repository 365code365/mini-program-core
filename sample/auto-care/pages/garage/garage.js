const db = require('../../utils/db.js');

Page({
  data: { cars: [], editing: null, km: '', kmErr: '' },
  onShow() { this.setData({ cars: db.getCars() }); },
  add() { wx.navigateTo({ url: '/pages/car-edit/car-edit' }); },
  edit(e) { wx.navigateTo({ url: '/pages/car-edit/car-edit?id=' + e.currentTarget.dataset.id }); },
  setDefault(e) {
    db.setDefaultCar(e.currentTarget.dataset.id);
    this.setData({ cars: db.getCars() });
    wx.showToast({ title: '已设为默认车辆', icon: 'none' });
  },
  more(e) {
    const car = this.data.cars.find((c) => c.id === e.currentTarget.dataset.id);
    wx.showActionSheet({
      itemList: ['更新当前里程', '查询违章', '删除车辆'],
      itemColor: '#1d2433',
      success: (r) => {
        if (r.tapIndex === 0) this.setData({ editing: car, km: String(car.mileage), kmErr: '' });
        else if (r.tapIndex === 1) wx.navigateTo({ url: '/pages/violation/violation?carId=' + car.id });
        else this.remove(car);
      }
    });
  },
  remove(car) {
    wx.showModal({
      title: '删除车辆', content: '删除「' + car.plateText + ' ' + car.name + '」后，保养提醒也会一并删除。', confirmColor: '#f0443a', confirmText: '删除',
      success: (r) => { if (r.confirm) { db.removeCar(car.id); this.setData({ cars: db.getCars() }); } }
    });
  },
  // 更新里程（showModal 的 editable 在桌面端没有输入框，这里用页内弹层）
  onKm(e) { this.setData({ km: e.detail.value, kmErr: '' }); },
  closeKm() { this.setData({ editing: null }); },
  saveKm() {
    const car = this.data.editing, km = parseInt(this.data.km, 10);
    if (!(km >= car.mileage) || km > 999999) { this.setData({ kmErr: '里程不能小于上次记录的 ' + car.mileage + ' km' }); return; }
    const raw = db.getCars().find((c) => c.id === car.id);
    raw.mileage = km;
    db.upsertCar(raw);
    this.setData({ editing: null, cars: db.getCars() });
    wx.showToast({ title: '里程已更新', icon: 'success' });
  }
});
