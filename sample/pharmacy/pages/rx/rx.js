// 处方上传与在线问诊：处方药必须二选一 —— 上传医生开具的处方照片（wx.chooseImage，拿不到相册时可用示例处方单），
// 或填写复诊问诊信息（互联网诊疗只能复诊，首诊引导线下就医）。提交后药师审核，通过的处方 7 天内对这些药有效
const { BY_ID, DRUGS, card } = require('../../utils/drugs.js');
const S = require('../../utils/store.js');

const DISEASES = ['高血压', '2型糖尿病', '高脂血症', '冠心病', '上呼吸道感染', '胃食管反流', '慢性胃炎'];

Page({
  data: {
    tab: 0, from: '', pick: false, drugs: [], genders: ['男', '女'], yesno: ['是', '否'], noyes: ['无', '有'], name: '', age: '', gender: '', imgs: [],
    diseases: DISEASES.map(t => ({ t, on: false })), diag: '', allergy: '无', allergyText: '', desc: '', promise: false, errs: {}
  },
  onLoad(q) {
    let ids = (q.ids || '').split(',').filter(id => BY_ID[id]);
    if (!ids.length) ids = S.cartDetail().filter(c => c.type === 'RX').map(c => c.id);
    ids = Array.from(new Set(ids));
    // 从首页入口进来、购物车里也没有处方药：列出全部处方药让用户勾选要开的药
    const pick = !ids.length;
    const list = pick ? DRUGS.filter(d => d.type === 'RX').map(d => d.id) : ids;
    this.setData({ from: q.from || '', pick, drugs: list.map(id => Object.assign(card(BY_ID[id]), { on: !pick })) });
  },
  onPick(e) {
    if (!this.data.pick) return; // 从购物车 / 详情页带进来的药是固定的，不能取消
    const i = +e.currentTarget.dataset.i;
    this.setData({ ['drugs[' + i + '].on']: !this.data.drugs[i].on, 'errs.drugs': '' });
  },
  onTab(e) { this.setData({ tab: +e.currentTarget.dataset.t, errs: {} }); },
  onField(e) { this.setData({ [e.currentTarget.dataset.f]: e.detail.value, ['errs.' + e.currentTarget.dataset.f]: '' }); },
  onGender(e) { this.setData({ gender: e.currentTarget.dataset.v, 'errs.gender': '' }); },
  addImg() {
    const left = 3 - this.data.imgs.length;
    if (left <= 0) { wx.showToast({ title: '最多上传 3 张', icon: 'none' }); return; }
    wx.chooseImage({
      count: left, sizeType: ['compressed'], sourceType: ['album', 'camera'],
      success: r => this.setData({ imgs: this.data.imgs.concat(r.tempFilePaths).slice(0, 3), 'errs.imgs': '' }),
      fail: err => {
        // 用户取消（桌面端没有相册时也回 cancel）：不打扰，页面上一直有「使用示例处方单」入口
        if (/cancel/.test(err.errMsg || '')) { wx.showToast({ title: '未选择图片', icon: 'none' }); return; }
        wx.showModal({
          title: '无法打开相册', content: '当前环境无法选择图片。是否使用示例处方单继续？（仅供演示，实际购药请上传真实处方）',
          confirmText: '使用示例', cancelText: '取消', confirmColor: '#12a37f',
          success: m => { if (m.confirm) this.setData({ imgs: this.data.imgs.concat('/images/rx-sample.jpg'), 'errs.imgs': '' }); }
        });
      }
    });
  },
  useSample() {
    if (this.data.imgs.indexOf('/images/rx-sample.jpg') >= 0) { wx.showToast({ title: '示例处方单已添加', icon: 'none' }); return; }
    if (this.data.imgs.length >= 3) { wx.showToast({ title: '最多上传 3 张', icon: 'none' }); return; }
    this.setData({ imgs: this.data.imgs.concat('/images/rx-sample.jpg'), 'errs.imgs': '' });
  },
  delImg(e) {
    const imgs = this.data.imgs.slice();
    imgs.splice(+e.currentTarget.dataset.i, 1);
    this.setData({ imgs });
  },
  preview(e) { wx.previewImage({ urls: this.data.imgs, current: this.data.imgs[+e.currentTarget.dataset.i] }); },
  onDisease(e) {
    const i = +e.currentTarget.dataset.i;
    this.setData({ ['diseases[' + i + '].on']: !this.data.diseases[i].on, 'errs.disease': '' });
  },
  onDiag(e) { this.setData({ diag: e.currentTarget.dataset.v, 'errs.diag': '' }); },
  onAllergy(e) { this.setData({ allergy: e.currentTarget.dataset.v, 'errs.allergyText': '' }); },
  onPromise(e) { this.setData({ promise: e.detail.value.length > 0, 'errs.promise': '' }); },
  validate() {
    const d = this.data, errs = {};
    if (!d.drugs.some(x => x.on)) errs.drugs = '请勾选需要开具的处方药';
    if (!/^[\u4e00-\u9fa5·]{2,10}$/.test(d.name.trim())) errs.name = '请填写用药人真实姓名';
    const age = +d.age;
    if (!/^\d{1,3}$/.test(d.age) || age < 1 || age > 120) errs.age = '请填写 1~120 的年龄';
    if (!d.gender) errs.gender = '请选择性别';
    if (d.tab === 0) {
      if (!d.imgs.length) errs.imgs = '请上传处方照片（需含医院印章、医师签名、开具日期）';
    } else {
      if (!d.diseases.some(x => x.on)) errs.disease = '请选择已确诊的疾病';
      if (!d.diag) errs.diag = '请选择是否已在线下医院确诊';
      else if (d.diag === '否') errs.diag = '在线问诊仅限复诊开药，首次发病请先到线下医院就诊';
      if (d.allergy === '有' && !d.allergyText.trim()) errs.allergyText = '请填写过敏药物或食物';
    }
    if (!d.promise) errs.promise = '请确认信息真实';
    this.setData({ errs });
    return Object.keys(errs).length === 0;
  },
  submit() {
    if (!this.validate()) { wx.showToast({ title: '请完善标红的信息', icon: 'none' }); return; }
    wx.showLoading({ title: this.data.tab === 0 ? '药师审方中' : '医生问诊中', mask: true });
    setTimeout(() => {
      wx.hideLoading();
      const now = new Date();
      const no = 'RX' + now.getFullYear() + ('0' + (now.getMonth() + 1)).slice(-2) + ('0' + now.getDate()).slice(-2) + String(Math.floor(1000 + Math.random() * 9000));
      const d = this.data;
      const rec = {
        no, ids: d.drugs.filter(x => x.on).map(x => x.id), method: d.tab === 0 ? '上传处方' : '在线问诊', at: Date.now(), status: '已通过',
        patient: d.name.trim()[0] + '*' + ' · ' + (d.gender) + ' · ' + d.age + '岁',
        diseases: d.diseases.filter(x => x.on).map(x => x.t), pharmacist: '执业药师 周敏'
      };
      S.set('rx', [rec].concat(S.get('rx', [])).slice(0, 20));
      wx.showToast({ title: '审核通过', icon: 'success' });
      setTimeout(() => {
        if (d.from === 'cart' || d.from === 'buy') wx.redirectTo({ url: '/pages/confirm/confirm?from=' + d.from });
        else wx.navigateBack({ fail: () => wx.switchTab({ url: '/pages/cart/cart' }) });
      }, 600);
    }, 1200);
  }
});
