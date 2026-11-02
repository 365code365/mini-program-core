const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { BRANDS, PROVINCES, CITY_LETTERS, COLORS } = require('../../utils/data.js');

// 字母索引：品牌表里出现过的首字母
const LETTERS = BRANDS.reduce((a, b) => (a.indexOf(b.letter) < 0 ? a.concat([b.letter]) : a), []);

Page({
  data: {
    isEdit: false, brands: BRANDS, letters: LETTERS, letter: 'A', intoView: '',
    brandIdx: -1, models: [], modelIdx: 0, colors: COLORS,
    provinces: PROVINCES, cityLetters: CITY_LETTERS,
    form: { year: '', color: 'white', prov: 0, city: 0, plateNo: '', vin: '', mileage: '', buyDate: '', lastCareDate: '', lastCareKm: '', isDefault: false },
    errors: {}, today: '', vinDialog: false, vinDraft: ''
  },
  onLoad(q) {
    const today = util.ymd(new Date());
    this.setData({ today, 'form.buyDate': today, 'form.lastCareDate': today });
    if (!q.id) return;
    const c = db.getCar(q.id);
    if (!c) return;
    this.carId = c.id;
    const bi = BRANDS.findIndex((b) => b.id === c.brandId);
    const models = bi >= 0 ? BRANDS[bi].models : [];
    this.setData({
      isEdit: true, brandIdx: bi, models, modelIdx: Math.max(0, models.indexOf(c.model)),
      letter: bi >= 0 ? BRANDS[bi].letter : 'A', intoView: bi >= 0 ? 'br-' + BRANDS[bi].id : '',
      form: {
        year: c.year, color: c.color, prov: Math.max(0, PROVINCES.indexOf(c.plate.slice(0, 1))),
        city: Math.max(0, CITY_LETTERS.indexOf(c.plate.slice(1, 2))), plateNo: c.plate.slice(2), vin: c.vin,
        mileage: String(c.mileage), buyDate: c.buyDate, lastCareDate: c.lastCareDate, lastCareKm: String(c.lastCareKm), isDefault: c.isDefault
      }
    });
    wx.setNavigationBarTitle({ title: '编辑车辆' });
  },
  // 点字母：横向品牌条滚到该字母的第一个品牌
  onLetter(e) {
    const l = e.currentTarget.dataset.l;
    const b = BRANDS.find((x) => x.letter === l);
    this.setData({ letter: l, intoView: 'br-' + b.id });
  },
  onBrand(e) {
    const i = +e.currentTarget.dataset.i;
    this.setData({ brandIdx: i, models: BRANDS[i].models, modelIdx: 0, letter: BRANDS[i].letter, 'errors.brand': '' });
  },
  onModel(e) { this.setData({ modelIdx: +e.detail.value }); },
  onProv(e) { this.setData({ 'form.prov': +e.detail.value }); },
  onCity(e) { this.setData({ 'form.city': +e.detail.value }); },
  onColor(e) { this.setData({ 'form.color': e.currentTarget.dataset.k }); },
  onDate(e) { this.setData({ ['form.' + e.currentTarget.dataset.k]: e.detail.value }); },
  onInput(e) {
    const k = e.currentTarget.dataset.k;
    let v = e.detail.value;
    if (k === 'plateNo' || k === 'vin') v = v.toUpperCase();
    this.setData({ ['form.' + k]: v, ['errors.' + k]: '' });
  },
  onDefault(e) { this.setData({ 'form.isDefault': e.detail.value }); },
  // 扫行驶证 / 前挡风玻璃下的 VIN 条码；扫不了就手动输入
  scanVin() {
    wx.scanCode({
      scanType: ['barCode', 'qrCode'],
      success: (r) => {
        const vin = String(r.result || '').toUpperCase().replace(/[^A-Z0-9]/g, '');
        if (util.checkVin(vin)) this.setData({ 'form.vin': vin, 'errors.vin': '' });
        else this.setData({ vinDialog: true, vinDraft: vin });
      },
      fail: () => this.setData({ vinDialog: true, vinDraft: this.data.form.vin })
    });
  },
  onVinDraft(e) { this.setData({ vinDraft: e.detail.value.toUpperCase() }); },
  closeVin() { this.setData({ vinDialog: false }); },
  confirmVin() {
    if (!util.checkVin(this.data.vinDraft)) { wx.showToast({ title: 'VIN 应为 17 位字母数字（不含 I/O/Q）', icon: 'none' }); return; }
    this.setData({ 'form.vin': this.data.vinDraft, vinDialog: false, 'errors.vin': '' });
  },
  validate() {
    const f = this.data.form, errors = {};
    if (this.data.brandIdx < 0) errors.brand = '请选择品牌';
    const plate = PROVINCES[f.prov] + CITY_LETTERS[f.city] + f.plateNo;
    if (!util.checkPlate(plate)) errors.plateNo = '车牌号应为 5 位（燃油车）或 6 位（新能源）字母数字';
    if (f.vin && !util.checkVin(f.vin)) errors.vin = 'VIN 应为 17 位，不含 I、O、Q';
    const km = parseInt(f.mileage, 10), care = parseInt(f.lastCareKm || '0', 10);
    if (!(km >= 0) || km > 999999) errors.mileage = '请填写当前里程';
    else if (care > km) errors.lastCareKm = '上次保养里程不能大于当前里程';
    if (f.buyDate > this.data.today) errors.buyDate = '购车日期不能晚于今天';
    // 车牌不能和已有车辆重复
    if (!errors.plateNo && db.getCars().some((c) => c.plate === plate && c.id !== this.carId)) errors.plateNo = '这辆车已经在爱车档案里了';
    this.setData({ errors });
    return Object.keys(errors).length ? null : plate;
  },
  save() {
    const plate = this.validate();
    if (!plate) { wx.showToast({ title: '请检查标红的信息', icon: 'none' }); return; }
    const f = this.data.form, b = BRANDS[this.data.brandIdx];
    const car = {
      id: this.carId || 'c' + Date.now(), brandId: b.id, brand: b.name, model: b.models[this.data.modelIdx],
      year: f.year.trim() || '—', color: f.color, plate, vin: f.vin, mileage: parseInt(f.mileage, 10),
      buyDate: f.buyDate, lastCareDate: f.lastCareDate, lastCareKm: parseInt(f.lastCareKm || '0', 10) || 0, isDefault: f.isDefault
    };
    db.upsertCar(car);
    wx.showToast({ title: this.carId ? '已保存' : '添加成功', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 500);
  },
  remove() {
    wx.showModal({
      title: '删除车辆', content: '确定删除这辆车？', confirmColor: '#f0443a',
      success: (r) => { if (r.confirm) { db.removeCar(this.carId); wx.navigateBack(); } }
    });
  }
});
