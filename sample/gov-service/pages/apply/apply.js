// 在线申报向导：申请人信息 → 事项信息 → 上传材料 → 确认提交。
// 每步校验后才能前进；上一步 / 点步骤条回退不丢数据；任意一步都能「保存草稿」（写 draft.json）。
// 填写中返回会弹确认（enableAlertBeforeUnload），存过草稿且之后没改动就不再拦。
const { getItem } = require('../../utils/items');
const store = require('../../utils/store');
const draft = require('../../utils/draft');
const { check } = require('../../utils/validate');

const STEPS = ['申请人信息', '事项信息', '上传材料', '确认提交'];
const REASONS = ['首次申请', '到期续办', '信息变更', '遗失补办'];
const WAYS = ['窗口领取', '邮寄送达'];

function emptyForm(itemId) {
  return { itemId, name: '', idcard: '', phone: '', region: [], address: '', reasonIdx: -1, way: '', remark: '', files: {}, agree: false };
}

Page({
  data: {
    steps: STEPS, reasons: REASONS, ways: WAYS,
    step: 0, item: null, form: null, errors: {}, saved: false
  },
  onLoad(q) {
    if (q.draft) {
      draft.load((d) => {
        if (!d) { this.init(emptyForm(q.id || 'G0301'), 0); return; }
        this.init(Object.assign(emptyForm(d.form.itemId), d.form), d.step || 0);
        this.setData({ saved: true });
        wx.showToast({ title: '已恢复草稿', icon: 'none' });
      });
    } else {
      this.init(emptyForm(q.id || 'G0301'), 0);
    }
  },
  init(form, step) {
    const item = getItem(form.itemId) || getItem('G0301');
    form.itemId = item.id;
    this.setData({ item, form, step });
    this.updateTitle();
    this.guard(true);
  },
  updateTitle() {
    wx.setNavigationBarTitle({ title: `申报（${this.data.step + 1}/4）${STEPS[this.data.step]}` });
  },
  // 有未保存改动时拦返回
  guard(on) {
    if (on) wx.enableAlertBeforeUnload({ message: '申报内容尚未提交，离开后未保存的内容将丢失。可先点「保存草稿」。' });
    else wx.disableAlertBeforeUnload();
  },
  set(field, value) {
    const errors = Object.assign({}, this.data.errors);
    delete errors[field];
    this.setData({ ['form.' + field]: value, errors });
    if (this.data.saved) { this.setData({ saved: false }); this.guard(true); }
  },
  onInput(e) { this.set(e.currentTarget.dataset.f, e.detail.value); },
  onRegion(e) { this.set('region', e.detail.value); },
  onReason(e) { this.set('reasonIdx', Number(e.detail.value)); },
  onWay(e) { this.set('way', e.detail.value); },
  onAgree(e) { this.set('agree', e.detail.value.length > 0); },
  fillMine() {
    const u = store.USER;
    const f = Object.assign({}, this.data.form, { name: u.name, idcard: u.idcard, phone: u.phone });
    if (!f.region.length) f.region = ['云南省', '昆明市', '五华区'];
    this.setData({ form: f, errors: {} });
  },
  pickFile(e) {
    const i = e.currentTarget.dataset.i;
    const mat = this.data.item.materials[i];
    const attach = (file) => this.set('files', Object.assign({}, this.data.form.files, { [i]: file }));
    wx.chooseMessageFile({
      count: 1, type: 'file', extension: ['pdf', 'jpg', 'jpeg', 'png'],
      success: (r) => {
        const f = r.tempFiles[0];
        if (f.size > 10 * 1024 * 1024) { wx.showToast({ title: '单个文件不超过 10MB', icon: 'none' }); return; }
        attach({ name: f.name, size: Math.round(f.size / 1024) + ' KB', path: f.path, sample: false });
      },
      // 用户取消与宿主选不了文件（桌面引擎、模拟器）都是 fail cancel，分不开：问一句要不要用示例材料
      fail: () => {
        wx.showModal({
          title: '未选择文件', content: `是否使用「${mat.name}」的示例材料继续？正式办理请上传本人材料。`,
          confirmText: '用示例', cancelText: '稍后再传',
          success: (r) => {
            if (r.confirm) attach({ name: `${mat.name}（示例）.pdf`, size: (180 + i * 37) + ' KB', path: '', sample: true });
          }
        });
      }
    });
  },
  removeFile(e) {
    const files = Object.assign({}, this.data.form.files);
    delete files[e.currentTarget.dataset.i];
    this.set('files', files);
  },
  validate() {
    const errors = check(this.data.step, this.data.form, this.data.item.materials);
    this.setData({ errors });
    const keys = Object.keys(errors);
    if (keys.length) wx.showToast({ title: errors[keys[0]], icon: 'none' });
    return !keys.length;
  },
  next() {
    if (!this.validate()) return;
    this.setData({ step: this.data.step + 1 });
    this.updateTitle();
    wx.pageScrollTo({ scrollTop: 0, duration: 0 });
  },
  prev() {
    if (this.data.step === 0) return;
    this.setData({ step: this.data.step - 1, errors: {} });
    this.updateTitle();
  },
  onJump(e) {
    this.setData({ step: e.detail.index, errors: {} });
    this.updateTitle();
  },
  saveDraft() {
    draft.save({ step: this.data.step, form: this.data.form }, (err) => {
      if (err) { wx.showToast({ title: '草稿保存失败：' + err, icon: 'none' }); return; }
      this.setData({ saved: true });
      this.guard(false);
      wx.showToast({ title: '草稿已保存，可从首页继续', icon: 'none' });
    });
  },
  submit() {
    if (this.data.submitting || !this.validate()) return;
    this.setData({ submitting: true });
    wx.showLoading({ title: '正在提交', mask: true });
    setTimeout(() => {
      const f = this.data.form;
      const order = store.addOrder(Object.assign({}, f, { reason: REASONS[f.reasonIdx] }));
      draft.remove();
      this.guard(false);
      wx.hideLoading();
      // 提交后清掉整条页面栈：申报页不该再能返回回来重复提交
      wx.reLaunch({ url: '/pages/result/result?id=' + order.id });
    }, 600);
  }
});
