const db = require('../../utils/db.js');

const TAGS = ['家', '公司', '父母家', '其他'];
const RULES = {
  name: [{ required: true, msg: '请填写联系人' }, { min: 2, msg: '联系人至少 2 个字' }, { max: 12, msg: '联系人最多 12 个字' }],
  phone: [{ required: true, msg: '请填写手机号' }, { pattern: '^1[3-9]\\d{9}$', msg: '手机号格式不对' }],
  region: [{ required: true, msg: '请选择所在地区' }],
  detail: [{ required: true, msg: '请填写详细地址' }, { min: 5, msg: '详细地址至少 5 个字，写到门牌号' }]
};

Page({
  data: { rules: RULES, tags: TAGS, form: { name: '', phone: '', region: [], detail: '', tag: '家', isDefault: false }, isEdit: false },
  onLoad(q) {
    this.from = q.from || '';
    if (!q.id) return;
    const a = db.getAddr(q.id);
    if (!a) return;
    this.addrId = a.id;
    this.setData({ isEdit: true, form: { name: a.name, phone: a.phone, region: a.region, detail: a.detail, tag: a.tag, isDefault: a.isDefault } });
    wx.setNavigationBarTitle({ title: '编辑地址' });
  },
  onInput(e) { this.setData({ ['form.' + e.currentTarget.dataset.k]: e.detail.value }); },
  onRegion(e) { this.setData({ 'form.region': e.detail.value }); },
  onTag(e) { this.setData({ 'form.tag': e.currentTarget.dataset.t }); },
  onDefault(e) { this.setData({ 'form.isDefault': e.detail.value }); },
  // 本引擎没有 relations：先把页面里的 form-item 交给 form-group（微信里 linked 已经收过，adopt 去重）
  group() {
    const g = this.selectComponent('#addrForm');
    if (g && g.count() < 4) g.adopt(this.selectAllComponents('.fi'));
    return g;
  },
  save() {
    const g = this.group();
    const r = g.validate();
    if (!r.ok) { wx.showToast({ title: r.first, icon: 'none' }); return; }
    const f = this.data.form;
    const a = { id: this.addrId || 'ad' + Date.now(), name: f.name.trim(), phone: f.phone, region: f.region, detail: f.detail.trim(), tag: f.tag, isDefault: f.isDefault };
    db.saveAddr(a);
    wx.showToast({ title: '已保存', icon: 'success' });
    setTimeout(() => {
      // 下单页 → 选择地址 → 新增：保存后跳过地址列表，直接回下单页并选中新地址
      if (this.from === 'select') { db.setPicked(a.id); wx.navigateBack({ delta: 2 }); }
      else wx.navigateBack();
    }, 500);
  },
  remove() {
    wx.showModal({
      title: '删除地址', content: '确定删除这个地址？', confirmColor: '#e5484d',
      success: (r) => { if (r.confirm) { db.removeAddr(this.addrId); wx.navigateBack(); } }
    });
  }
});
