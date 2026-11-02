const S = require('../../utils/store.js');
const R = require('../../utils/reply.js');

const TAGS = ['回复及时', '专业细致', '通俗易懂', '有耐心', '给出可行方案', '态度一般'];

Page({
  data: {
    c: null, msgs: [], toView: '', text: '', kb: 0, typing: false,
    stars: [1, 2, 3, 4, 5], star: 5, tags: TAGS.map(t => ({ t, on: false })), rateText: '', rating: false
  },
  onLoad(q) {
    this.id = q.id || (S.consults()[0] && S.consults()[0].id);
    this.timers = [];
    // 输入框 adjust-position=false：页面不被系统顶起，按键盘高度自己把输入栏抬上去
    this.onKb = res => {
      this.setData({ kb: res.height || 0 });
      this.scrollEnd();
    };
    wx.onKeyboardHeightChange(this.onKb);
    this.refresh();
    const c = this.data.c;
    if (c && c.status === 'active' && !c.messages.some(m => m.from === 'lawyer')) this.lawyerReply(R.first(c.question));
  },
  onUnload() {
    this.timers.forEach(clearTimeout);
    wx.offKeyboardHeightChange && wx.offKeyboardHeightChange(this.onKb);
  },
  refresh() {
    const c = S.getConsult(this.id);
    if (!c) return;
    wx.setNavigationBarTitle({ title: c.lawyerName + ' 律师' });
    this.setData({ c, msgs: c.messages, statusText: S.STATUS[c.status] });
    this.scrollEnd();
  },
  scrollEnd() {
    const m = this.data.msgs;
    // 先清空再设：同一个 id 连设两次 scroll-into-view 不会再滚
    this.setData({ toView: '' });
    if (m.length) this.setData({ toView: this.data.typing ? 'typing' : m[m.length - 1].id });
  },
  lawyerReply(lines) {
    this.setData({ typing: true });
    this.scrollEnd();
    lines.forEach((line, i) => {
      this.timers.push(setTimeout(() => {
        S.pushMessage(this.id, { from: 'lawyer', type: 'text', content: line });
        if (i === lines.length - 1) this.setData({ typing: false });
        this.refresh();
      }, 1200 + i * 1500));
    });
  },
  onInput(e) { this.setData({ text: e.detail.value }); },
  // confirm-type="send" + confirm-hold：回车发送且键盘不收起
  send() {
    const t = this.data.text.trim();
    if (!t || this.data.c.status !== 'active') return;
    S.pushMessage(this.id, { from: 'me', type: 'text', content: t });
    this.setData({ text: '' });
    this.refresh();
    this.followN = (this.followN || 0) + 1;
    this.lawyerReply([R.follow(this.followN - 1)]);
  },
  paste() {
    wx.getClipboardData({
      success: res => {
        if (!res.data) return wx.showToast({ title: '剪贴板是空的', icon: 'none' });
        this.setData({ text: (this.data.text + res.data).slice(0, 500) });
      }
    });
  },
  sendImage() {
    const push = src => {
      S.pushMessage(this.id, { from: 'me', type: 'image', content: src });
      this.refresh();
      this.lawyerReply(['图片收到了，我看一下合同条款。押金退还和违约金条款写得比较清楚，对你是有利的。']);
    };
    wx.chooseImage({
      count: 1,
      success: res => push(res.tempFilePaths[0]),
      fail: () => wx.showActionSheet({
        itemList: ['发送示例证据图（租赁合同）'],
        success: r => { if (r.tapIndex === 0) push('/images/evidence.jpg'); }
      })
    });
  },
  preview(e) { wx.previewImage({ urls: [e.currentTarget.dataset.src] }); },
  msgMenu(e) {
    const m = this.data.msgs.find(x => x.id === e.currentTarget.dataset.id);
    if (!m || m.type !== 'text') return;
    wx.showActionSheet({
      itemList: ['复制', '引用回复'],
      success: r => {
        if (r.tapIndex === 0) wx.setClipboardData({ data: m.content });
        else this.setData({ text: '「' + m.content.slice(0, 20) + '…」' });
      }
    });
  },
  finish() {
    wx.showModal({
      title: '结束咨询',
      content: '结束后不能再追问，确认问题已经解决了吗？',
      confirmText: '结束',
      success: r => {
        if (!r.confirm) return;
        S.pushMessage(this.id, { from: 'sys', type: 'text', content: '咨询已结束，欢迎对律师的服务做出评价' });
        S.updateConsult(this.id, { status: 'done' });
        this.refresh();
        this.setData({ rating: true });
      }
    });
  },
  openRate() { this.setData({ rating: true }); },
  closeRate() { this.setData({ rating: false }); },
  setStar(e) { this.setData({ star: e.currentTarget.dataset.s }); },
  toggleTag(e) {
    const i = e.currentTarget.dataset.i;
    this.setData({ ['tags[' + i + '].on']: !this.data.tags[i].on });
  },
  onRateText(e) { this.setData({ rateText: e.detail.value }); },
  submitRate() {
    const tags = this.data.tags.filter(t => t.on).map(t => t.t);
    if (!tags.length && !this.data.rateText.trim()) return wx.showToast({ title: '选个标签或写几句吧', icon: 'none' });
    const d = new Date();
    const date = `${d.getFullYear()}-${('0' + (d.getMonth() + 1)).slice(-2)}-${('0' + d.getDate()).slice(-2)}`;
    S.updateConsult(this.id, { status: 'rated', rating: { star: this.data.star, tags, text: this.data.rateText.trim(), date } });
    this.setData({ rating: false });
    this.refresh();
    wx.showToast({ title: '感谢评价', icon: 'success' });
  }
});
