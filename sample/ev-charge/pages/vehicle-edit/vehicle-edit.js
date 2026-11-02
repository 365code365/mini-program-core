// 新增 / 编辑车辆：车牌（可调微信车牌键盘）、车型（带出电池容量）、默认车辆
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { id: '', plate: '', plateSeq: 0, models: data.MODELS.map(m => m.name), mIdx: -1, battery: '', batSeq: 0, isDefault: false, editing: false },
  onLoad(q) {
    const c = q.id ? store.findCar(String(q.id)) : null;
    if (c) {
      this.setData({
        id: c.id, plate: c.plate, battery: String(c.battery), isDefault: !!c.isDefault, editing: true,
        mIdx: Math.max(0, data.MODELS.findIndex(m => m.name === c.model))
      });
      wx.setNavigationBarTitle({ title: '编辑车辆' });
    } else {
      this.setData({ isDefault: store.getCars().length === 0 });
    }
  },
  onPlate(e) { this.setData({ plate: e.detail.value }); },
  onPlateKb() {
    host.plate().then(p => { if (p) this.setData({ plate: p, plateSeq: this.data.plateSeq + 1 }); });
  },
  onModel(e) {
    const i = Number(e.detail.value);
    this.setData({ mIdx: i, battery: String(data.MODELS[i].battery), batSeq: this.data.batSeq + 1 });
  },
  onBattery(e) { this.setData({ battery: e.detail.value }); },
  onDefault(e) { this.setData({ isDefault: !!e.detail.value }); },
  onSave() {
    const d = this.data;
    const plate = store.normPlate(d.plate);
    const battery = Number(d.battery);
    let err = '';
    if (!plate) err = '请填写正确的车牌，如 浙AD88520';
    else if (d.mIdx < 0) err = '请选择车型';
    else if (!(battery >= 10 && battery <= 200)) err = '电池容量需在 10~200 度';
    if (err) { wx.showToast({ title: err, icon: 'none' }); return; }
    const msg = store.saveCar({ id: d.id, plate, model: data.MODELS[d.mIdx].name, battery: store.round2(battery), isDefault: d.isDefault });
    if (msg) { wx.showToast({ title: msg, icon: 'none' }); return; }
    wx.showToast({ title: d.editing ? '已保存' : '已添加 ' + plate, icon: 'none' });
    setTimeout(() => wx.navigateBack(), 500);
  },
  onRemove() {
    const s = store.getSession();
    if (s && s.carId === this.data.id) { wx.showToast({ title: '这辆车正在充电，结束后再删除', icon: 'none' }); return; }
    wx.showModal({
      title: '删除车辆', content: '确定删除 ' + this.data.plate + ' 吗？', confirmText: '删除', confirmColor: '#F5455C',
      success: r => {
        if (!r.confirm) return;
        store.removeCar(this.data.id);
        wx.navigateBack();
      }
    });
  }
});
