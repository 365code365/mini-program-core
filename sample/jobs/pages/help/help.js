// 帮助与反馈：常见问题（点开看答案）、提交反馈（类型 + 描述）、我的反馈、客服电话
const S = require('../../utils/store.js');
const host = require('../../utils/host.js');
const FAQ = [
  { q: '投递后多久能收到回复？', a: 'HR 一般在 3 个工作日内查看简历。在「我的 → 投递记录」里可以看到每份投递的状态，被查看后也可以直接在沟通里追问。' },
  { q: '怎么撤回已经投递的简历？', a: '在投递记录里，HR 还没约面试之前的投递都可以点「撤回」。撤回后可以重新投递。' },
  { q: '面试邀请怎么改期？', a: '在面试邀约里点「拒绝」并选择「时间冲突，希望改期」，HR 会在沟通里和你重新约时间。' },
  { q: '不想让现在的公司看到我的简历？', a: '在「隐私设置 → 屏蔽公司」里打开对应公司的开关，对方就搜不到你的简历了。' }
];
const TYPES = ['功能建议', '职位信息有误', '遭遇骚扰', '其他'];
Page({
  data: { faq: FAQ, open: -1, types: TYPES, type: 0, text: '', seq: 0, list: [] },
  onShow() { this.setData({ list: S.getFeedback() }); },
  onQ(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ open: this.data.open === i ? -1 : i });
  },
  onType(e) { this.setData({ type: Number(e.currentTarget.dataset.i) }); },
  onInput(e) { this.setData({ text: e.detail.value }); },
  onSubmit() {
    const text = this.data.text.trim();
    if (text.length < 5) return wx.showToast({ title: '请至少写 5 个字描述问题', icon: 'none' });
    S.addFeedback({ type: TYPES[this.data.type], text });
    this.setData({ text: '', seq: this.data.seq + 1, list: S.getFeedback() });
    wx.showToast({ title: '感谢反馈，我们会尽快处理', icon: 'none' });
  },
  onCall() { host.call('400 600 8899', '职聘直通客服'); }
});
