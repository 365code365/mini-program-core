const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
// 新增 / 编辑宠物档案（?id= 编辑）。健康记录与体重曲线在档案页单独维护，这里只管基本信息
Page({
  data: {
    // initial：头像上的字。WXML 里 {{name[0]}} 取字符串下标在引擎里取不到，在 JS 里算好
    id: '', editing: false, today: '', initial: '宠',
    kinds: ['猫', '狗', '其他'], genders: ['公', '母'], colors: data.COLORS,
    form: { name: '', kind: '猫', breed: '', gender: '公', neutered: false, birth: '', weight: '', chip: '', color: data.COLORS[0] }
  },
  onLoad(q) {
    this.setData({ today: store.today() });
    const p = q.id ? store.findPet(q.id) : null;
    if (p) {
      this.setData({
        id: p.id, editing: true, initial: p.name.charAt(0),
        form: { name: p.name, kind: p.kind, breed: p.breed, gender: p.gender, neutered: !!p.neutered, birth: p.birth, weight: String(p.weight), chip: p.chip || '', color: p.color }
      });
      wx.setNavigationBarTitle({ title: '编辑' + p.name + '的档案' });
    } else {
      // 新宠物默认挑一个还没用过的颜色
      const used = store.getPets().map(x => x.color);
      this.setData({ 'form.color': data.COLORS.find(c => used.indexOf(c) < 0) || data.COLORS[0] });
      wx.setNavigationBarTitle({ title: '添加宠物' });
    }
  },
  onField(e) {
    const f = e.currentTarget.dataset.f;
    const patch = { ['form.' + f]: e.detail.value };
    if (f === 'name') patch.initial = e.detail.value.trim().charAt(0) || '宠';
    this.setData(patch);
  },
  pick(e) { this.setData({ ['form.' + e.currentTarget.dataset.f]: e.currentTarget.dataset.v }); },
  onNeutered(e) { this.setData({ 'form.neutered': e.detail.value }); },
  onBirth(e) { this.setData({ 'form.birth': e.detail.value }); },
  save() {
    const f = this.data.form;
    const name = f.name.trim();
    const weight = Number(f.weight);
    let err = '';
    if (!name) err = '给毛孩子起个名字吧';
    else if (name.length > 8) err = '名字最多 8 个字';
    else if (!f.birth) err = '请选择生日（不确定可选大概日期）';
    else if (!(weight > 0) || weight > 80) err = '体重请填 0.1~80 之间的数字';
    else if (f.chip && !/^[\d\s]{9,20}$/.test(f.chip.trim())) err = '芯片号为 9~15 位数字';
    if (err) return wx.showToast({ title: err, icon: 'none' });
    const p = {
      id: this.data.id || 'p' + Date.now().toString(36), name, kind: f.kind, breed: f.breed.trim() || (f.kind === '其他' ? '小宠' : '中华田园' + f.kind),
      gender: f.gender, neutered: f.neutered, birth: f.birth, weight: Math.round(weight * 10) / 10, chip: f.chip.trim(), color: f.color
    };
    const msg = store.upsertPet(p);
    if (msg) return wx.showToast({ title: msg, icon: 'none' });
    store.setCurrent(p.id);
    wx.showToast({ title: this.data.editing ? '档案已更新' : '已添加 ' + name, icon: 'success' });
    setTimeout(() => wx.navigateBack(), 500);
  },
  remove() {
    wx.showModal({
      title: '删除档案',
      content: '删除「' + this.data.form.name + '」的档案后，健康记录与体重曲线一并清除，确定吗？',
      confirmText: '删除',
      confirmColor: '#ff5b35',
      success: r => {
        if (!r.confirm) return;
        const msg = store.removePet(this.data.id);
        if (msg) return wx.showModal({ title: '暂时不能删除', content: msg, showCancel: false, confirmText: '知道了' });
        wx.showToast({ title: '已删除', icon: 'none' });
        setTimeout(() => wx.navigateBack(), 500);
      }
    });
  }
});
