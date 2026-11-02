const S = require('../../utils/store.js');
const host = require('../../utils/host.js');
const { FAQS, FEEDBACK_TYPES, CANTEEN } = require('../../utils/data.js');
const MAX = 200;

Page({
  data: {
    faqs: FAQS.map((f, i) => ({ i, q: f.q, a: f.a, open: false })),
    canteen: CANTEEN,
    types: FEEDBACK_TYPES,
    type: '',
    text: '',
    textLen: 0,
    max: MAX,
    // 提交后清空时 +1：输入框换个 id 重建（用户打过字的 textarea，setData 改 value 画面不跟）
    seq: 0,
    list: []
  },
  onShow() { this.loadList(); },
  loadList() {
    this.setData({ list: S.getFeedback().map(f => Object.assign({}, f, { timeText: S.fmtTime(f.time) })) });
  },
  onFaq(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ ['faqs[' + i + '].open']: !this.data.faqs[i].open });
  },
  onCall() { host.call(CANTEEN.phone, '联系食堂'); },
  onType(e) {
    const t = String(e.currentTarget.dataset.t);
    this.setData({ type: this.data.type === t ? '' : t });
  },
  onText(e) {
    const v = (e.detail.value || '').slice(0, MAX);
    this.setData({ text: v, textLen: v.length });
  },
  onSubmit() {
    const text = this.data.text.trim();
    if (!this.data.type) { wx.showToast({ title: '请选择反馈类型', icon: 'none' }); return; }
    if (text.length < 5) { wx.showToast({ title: '至少写 5 个字，方便食堂了解情况', icon: 'none' }); return; }
    S.addFeedback({ id: 'F' + Date.now().toString(36), type: this.data.type, text, time: Date.now(), table: S.getTable() });
    this.setData({ type: '', text: '', textLen: 0, seq: this.data.seq + 1 });
    this.loadList();
    wx.showToast({ title: '已提交，食堂会在 1 个工作日内回复', icon: 'none' });
  }
});
