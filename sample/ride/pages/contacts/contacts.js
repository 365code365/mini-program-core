// 安全中心：紧急联系人（最多 5 位，可从通讯录导入，宿主不支持时手填）、行程录音 / 夜间自动分享开关、一键报警
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const MAX = 5;
const RELATIONS = ['家人', '朋友', '同事', '其他'];

Page({
  data: { list: [], adding: false, name: '', phone: '', formSeq: 0, relations: RELATIONS, relation: '家人', recording: true, autoShare: true, max: MAX },
  onShow() {
    this.load();
  },
  load() {
    this.setData({
      list: store.getContacts().map(c => Object.assign({}, c, { mask: store.maskPhone(c.phone), initial: c.name.charAt(0) })),
      recording: store.recording(),
      autoShare: store.autoShare()
    });
  },
  onAdd() {
    if (this.data.list.length >= MAX) {
      wx.showToast({ title: '最多添加 5 位紧急联系人', icon: 'none' });
      return;
    }
    this.setData({ adding: true, name: '', phone: '', relation: '家人', formSeq: this.data.formSeq + 1 });
  },
  onCancelAdd() {
    this.setData({ adding: false, name: '', phone: '', relation: '家人' });
  },
  onImport() {
    if (this.data.list.length >= MAX) {
      wx.showToast({ title: '最多添加 5 位紧急联系人', icon: 'none' });
      return;
    }
    host.contact().then(c => {
      // 选到了就填进表单让用户确认关系再保存；没选到（不支持 / 取消）也把表单打开手填
      this.setData({ adding: true, name: c ? c.name : this.data.name, phone: c ? c.phone : this.data.phone, formSeq: this.data.formSeq + (c || !this.data.adding ? 1 : 0) });
    });
  },
  onName(e) {
    this.setData({ name: e.detail.value });
  },
  onPhone(e) {
    this.setData({ phone: e.detail.value });
  },
  onRelation(e) {
    this.setData({ relation: e.currentTarget.dataset.r });
  },
  onSave() {
    const name = this.data.name.trim();
    const phone = String(this.data.phone).replace(/\s/g, '');
    const list = store.getContacts();
    let err = '';
    if (!name) err = '请填写联系人姓名';
    else if (name.length > 10) err = '姓名最多 10 个字';
    else if (!/^1\d{10}$/.test(phone)) err = '请填写 11 位手机号';
    else if (phone === store.getProfile().phone) err = '不能添加自己的手机号';
    else if (list.some(c => c.phone === phone)) err = '这个号码已经是紧急联系人了';
    else if (list.length >= MAX) err = '最多添加 5 位紧急联系人';
    if (err) {
      wx.showToast({ title: err, icon: 'none' });
      return;
    }
    list.push({ id: 'p' + Date.now(), name, phone, relation: this.data.relation });
    store.saveContacts(list);
    this.setData({ adding: false, name: '', phone: '', relation: '家人' });
    this.load();
    wx.showToast({ title: '已添加', icon: 'success' });
  },
  onRemove(e) {
    const c = this.data.list[Number(e.currentTarget.dataset.i)];
    if (!c) return;
    wx.showModal({
      title: '删除紧急联系人',
      content: '删除后行程分享、夜间自动分享都不会再发给 ' + c.name,
      confirmText: '删除',
      confirmColor: '#F5455C',
      success: r => {
        if (!r.confirm) return;
        store.saveContacts(store.getContacts().filter(x => x.id !== c.id));
        this.load();
      }
    });
  },
  onCallContact(e) {
    const c = this.data.list[Number(e.currentTarget.dataset.i)];
    if (c) host.call(c.phone, c.name + '（' + c.relation + '）');
  },
  onRecording(e) {
    store.setRecording(e.detail.value);
    this.setData({ recording: !!e.detail.value });
  },
  onAutoShare(e) {
    store.setAutoShare(e.detail.value);
    this.setData({ autoShare: !!e.detail.value });
  },
  onAlarm() {
    wx.showModal({
      title: '一键报警',
      content: '将拨打 110，并把你的当前位置同步给平台安全专员',
      confirmText: '拨打 110',
      confirmColor: '#F5455C',
      success: r => {
        if (r.confirm) host.call('110', '报警电话');
      }
    });
  }
});
