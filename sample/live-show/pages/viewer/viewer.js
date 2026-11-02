// 观演人管理：新增（实名校验）、删除；从确认订单进来（?pick=N）时可直接勾选本单的观演人
const store = require('../../utils/store.js');
const TYPES = ['身份证', '港澳居民来往内地通行证', '台湾居民来往大陆通行证', '护照'];
const W = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
const C = '10X98765432';

function checkId(no) {
  if (!/^\d{17}[\dX]$/.test(no)) return false;
  let s = 0;
  for (let i = 0; i < 17; i++) s += Number(no[i]) * W[i];
  return C[s % 11] === no[17];
}
function mask(no) { return no.length > 8 ? no.slice(0, 4) + '**********'.slice(0, no.length - 8) + no.slice(-4) : no; }

Page({
  data: { list: [], pickN: 0, picked: [], types: TYPES, typeIdx: 0, name: '', no: '', errs: {}, adding: false },
  onLoad(q) {
    this.setData({ pickN: Number(q.pick || 0), adding: !store.viewers().length });
    this.load();
  },
  load() {
    const pick = store.pick();
    this.setData({ list: store.viewers().map((v) => ({ id: v.id, name: v.name, type: v.type, mask: v.mask, on: pick.indexOf(v.id) >= 0 })), picked: pick });
  },
  toggle(e) {
    if (!this.data.pickN) return;
    const id = e.currentTarget.dataset.id;
    let pick = store.pick();
    if (pick.indexOf(id) >= 0) pick = pick.filter((x) => x !== id);
    else {
      if (pick.length >= this.data.pickN) { wx.showToast({ title: '本单只需 ' + this.data.pickN + ' 位观演人', icon: 'none' }); return; }
      pick.push(id);
    }
    store.setPick(pick);
    this.load();
  },
  remove(e) {
    const id = e.currentTarget.dataset.id;
    const v = store.viewers().find((x) => x.id === id);
    wx.showModal({
      title: '删除观演人', content: '确定删除「' + v.name + '」？已购订单不受影响', confirmColor: '#ff3b5c',
      success: (r) => {
        if (!r.confirm) return;
        store.saveViewers(store.viewers().filter((x) => x.id !== id));
        store.setPick(store.pick().filter((x) => x !== id));
        this.load();
      }
    });
  },
  openAdd() { this.setData({ adding: true, name: '', no: '', errs: {} }); },
  cancelAdd() { this.setData({ adding: false }); },
  onName(e) { this.setData({ name: e.detail.value.trim(), 'errs.name': '' }); },
  onNo(e) { this.setData({ no: e.detail.value.trim().toUpperCase(), 'errs.no': '' }); },
  onType(e) { this.setData({ typeIdx: Number(e.detail.value), 'errs.no': '' }); },
  save() {
    const { name, no, typeIdx } = this.data;
    const errs = {};
    if (!/^[\u4e00-\u9fa5·A-Za-z ]{2,20}$/.test(name)) errs.name = '请填写证件上的姓名（2~20 个字）';
    if (typeIdx === 0 ? !checkId(no) : !/^[A-Z0-9]{8,11}$/.test(no)) errs.no = typeIdx === 0 ? '身份证号码不正确，请核对 18 位号码' : '证件号码应为 8~11 位字母或数字';
    const list = store.viewers();
    if (!errs.no && list.some((v) => v.no === no)) errs.no = '该证件已添加';
    if (errs.name || errs.no) { this.setData({ errs }); return; }
    const v = { id: 'v' + Date.now(), name, type: TYPES[typeIdx], no, mask: mask(no) };
    list.push(v);
    store.saveViewers(list);
    // 从确认订单进来且还差人时，新加的直接勾上
    const pick = store.pick();
    if (this.data.pickN && pick.length < this.data.pickN) { pick.push(v.id); store.setPick(pick); }
    this.setData({ adding: false });
    this.load();
    wx.showToast({ title: '已添加', icon: 'success' });
  },
  done() { wx.navigateBack(); }
});
