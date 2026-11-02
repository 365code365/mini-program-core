const store = require('../../utils/store.js');
// 添加健康记录（疫苗 / 驱虫 / 体检）或记一次体重（?mode=weight）。?pet= 哪只宠物，?type= 预选类型
const TYPES = ['疫苗', '驱虫', '体检'];
const NAMES = {
  疫苗: { 猫: ['猫三联', '狂犬疫苗', '猫白血病疫苗'], 狗: ['犬八联', '狂犬疫苗', '犬窝咳疫苗'], 其他: ['狂犬疫苗'] },
  驱虫: { 猫: ['体内驱虫', '体外驱虫', '内外同驱'], 狗: ['体内驱虫', '体外驱虫', '内外同驱'], 其他: ['体内驱虫', '体外驱虫'] },
  体检: { 猫: ['年度体检', '血常规', '牙齿检查'], 狗: ['年度体检', '血常规', '牙齿检查'], 其他: ['年度体检'] }
};
const CYCLES = [{ d: 30, n: '每月' }, { d: 90, n: '每季度' }, { d: 180, n: '每半年' }, { d: 365, n: '每年' }];
const DEFAULT_CYCLE = { 疫苗: 365, 驱虫: 90, 体检: 365 };
Page({
  data: {
    mode: 'record', pet: null, today: '',
    // nameSeq：点快捷名称时改写输入框；打过字的输入框 setData 改 value 画面不更新，换 id 让它重建
    types: TYPES, type: '疫苗', names: [], name: '', nameSeq: 0, date: '', cycles: CYCLES, cycle: 365, next: '',
    weight: '', last: ''
  },
  onLoad(q) {
    const pet = store.findPet(q.pet) || store.currentPet();
    const t = store.today();
    const type = TYPES.indexOf(q.type) >= 0 ? q.type : '疫苗';
    this.setData({ mode: q.mode === 'weight' ? 'weight' : 'record', pet, today: t, date: t });
    if (!pet) return;
    if (this.data.mode === 'weight') {
      const w = pet.weights && pet.weights.length ? pet.weights[pet.weights.length - 1] : null;
      this.setData({ last: w ? '上次 ' + w.date + ' 称重 ' + w.v + ' kg' : '还没有称重记录' });
      wx.setNavigationBarTitle({ title: '给' + pet.name + '记体重' });
    } else {
      this.setType(type);
      wx.setNavigationBarTitle({ title: '给' + pet.name + '添加记录' });
    }
  },
  setType(type) {
    const cycle = DEFAULT_CYCLE[type];
    this.setData({ type, names: NAMES[type][this.data.pet.kind] || NAMES[type]['其他'], cycle, next: store.addDays(this.data.date, cycle) });
  },
  pickType(e) { this.setType(e.currentTarget.dataset.t); },
  pickName(e) { this.setData({ name: String(e.currentTarget.dataset.n), nameSeq: this.data.nameSeq + 1 }); },
  onName(e) { this.setData({ name: e.detail.value }); },
  onDate(e) { this.setData({ date: e.detail.value, next: store.addDays(e.detail.value, this.data.cycle) }); },
  pickCycle(e) {
    const d = Number(e.currentTarget.dataset.d);
    this.setData({ cycle: d, next: store.addDays(this.data.date, d) });
  },
  onWeight(e) { this.setData({ weight: e.detail.value }); },
  save() {
    const d = this.data;
    if (d.mode === 'weight') {
      const v = Number(d.weight);
      if (!(v > 0) || v > 80) return wx.showToast({ title: '体重请填 0.1~80 之间的数字', icon: 'none' });
      store.addWeight(d.pet.id, Math.round(v * 10) / 10, d.date);
      wx.showToast({ title: '已记录 ' + v + ' kg', icon: 'success' });
    } else {
      const name = d.name.trim();
      if (name.length < 2) return wx.showToast({ title: '请填写或选择记录名称', icon: 'none' });
      store.addRecord(d.pet.id, { type: d.type, name, date: d.date, cycle: d.cycle });
      wx.showToast({ title: '已添加，' + d.next + ' 前会提醒你', icon: 'none' });
    }
    setTimeout(() => wx.navigateBack(), 500);
  }
});
