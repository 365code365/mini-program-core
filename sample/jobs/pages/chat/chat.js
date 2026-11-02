// 与 HR 的会话：发消息、发简历（等同投递）、交换电话、常用语；HR 自动回复推动投递状态（被查看 → 邀面试）
const data = require('../../utils/data.js');
const S = require('../../utils/store.js');
const host = require('../../utils/host.js');
const PHRASES = ['您好，我对这个职位很感兴趣，希望能进一步沟通', '请问这个岗位目前还在招聘吗？', '我可以随时到岗，方便的话可以约个面试', '请问团队规模和技术栈是怎样的？', '感谢您的回复，期待您的消息'];
Page({
  data: { s: null, job: null, msgs: [], text: '', seq: 0, app: null },
  onLoad(q) {
    this.sid = q.sid || 's1';
    const s = S.getSession(this.sid);
    if (!s) return;
    S.markRead([this.sid]);
    wx.setNavigationBarTitle({ title: s.hrInfo.name + ' · ' + s.co.name });
    this.setData({ s, job: data.getJob(s.job) });
    this.refresh(true);
    // 最后一条是我发的（比如在职位详情投递时发出的简历）还没人回：HR 稍后回复
    const l = S.getMsgs(this.sid);
    const last = l[l.length - 1];
    if (last && last.from === 'me') this.timer = setTimeout(() => this.reply(last.resume ? 'resume' : 'text'), 800);
  },
  onShow() {
    if (this.data.s) this.refresh(false);
  },
  onUnload() {
    clearTimeout(this.timer);
  },
  refresh(scroll) {
    const msgs = S.getMsgs(this.sid).map((m, i, l) => {
      const out = Object.assign({}, m, { time: S.shortTime(m.ts) });
      // 相隔 5 分钟以上才显示时间
      out.showTime = i === 0 || m.ts - l[i - 1].ts > 5 * 60000;
      if (m.invite) out.iv = S.getInterview(m.invite);
      return out;
    });
    // 等新消息排好版再滚到底，不然滚到的是上一版的高度
    this.setData({ msgs, app: S.getApp(this.data.s.job) }, () => { if (scroll) this.toBottom(); });
  },
  toBottom() {
    wx.pageScrollTo({ scrollTop: 99999, duration: 0 });
  },
  onInput(e) { this.setData({ text: e.detail.value }); },
  send(msg, kind) {
    S.addMsg(this.sid, Object.assign({ from: 'me' }, msg));
    this.refresh(true);
    clearTimeout(this.timer);
    this.timer = setTimeout(() => this.reply(kind), 800);
  },
  onSend() {
    const text = this.data.text.trim();
    if (!text) return wx.showToast({ title: '说点什么吧', icon: 'none' });
    this.setData({ text: '', seq: this.data.seq + 1 });
    this.send({ text }, 'text');
  },
  onPhrase() {
    wx.showActionSheet({
      itemList: PHRASES,
      success: r => this.send({ text: PHRASES[r.tapIndex] }, 'text')
    });
  },
  onResume() {
    if (!S.resumeReady()) {
      return wx.showModal({
        title: '先完善简历', content: '发送简历需要填写姓名和手机号。', confirmText: '去完善', confirmColor: '#2E6BFF',
        success: r => r.confirm && wx.navigateTo({ url: '/pages/resume/resume' })
      });
    }
    const r = S.getResume();
    const first = !!S.apply(this.data.s.job);
    this.send({ text: '[简历] ' + r.name + '的在线简历', resume: true }, 'resume');
    if (first) wx.showToast({ title: '简历已发送，视为投递', icon: 'none' });
  },
  onPhone() {
    const r = S.getResume();
    if (S.getSettings().hidePhone) {
      return wx.showModal({
        title: '交换电话', content: '你开启了「隐藏手机号」，交换后对方能看到你的号码，继续吗？', confirmText: '继续', confirmColor: '#2E6BFF',
        success: x => x.confirm && this.send({ text: '[交换电话] 我的手机号 ' + r.phone }, 'phone')
      });
    }
    this.send({ text: '[交换电话] 我的手机号 ' + r.phone }, 'phone');
  },
  // HR 的自动回复：发简历 → 已查看；查看过之后再聊 → 发面试邀请
  reply(kind) {
    const s = this.data.s;
    const hr = s.hrInfo;
    const app = S.getApp(s.job);
    if (kind === 'resume') {
      if (app && app.status === 0) S.setAppStatus(s.job, 1);
      S.addMsg(this.sid, { from: 'hr', text: '收到你的简历了，我先看一下，合适的话会尽快约你面试～' });
    } else if (kind === 'phone') {
      S.addMsg(this.sid, { from: 'hr', text: '好的，这是我的电话 ' + hr.phone + '，工作日 9:00-18:30 都可以打', phone: hr.phone });
    } else if (!app) {
      S.addMsg(this.sid, { from: 'hr', text: '可以先发一份简历给我看看～' });
    } else if (app.status >= 1 && !S.getInterviews().some(v => v.job === s.job)) {
      const v = S.createInterview(s.job, this.sid);
      S.addMsg(this.sid, { from: 'hr', text: '你的背景和我们挺匹配的，约个面试吧，记得在邀请里确认时间哦。' });
      S.addMsg(this.sid, { from: 'hr', text: '[面试邀请] ' + v.mode, invite: v.id });
    } else {
      S.addMsg(this.sid, { from: 'hr', text: '好的，有进展我会第一时间联系你。' });
    }
    this.refresh(true);
  },
  onCall(e) {
    host.call(e.currentTarget.dataset.phone, this.data.s.hrInfo.name);
  },
  onInvite(e) {
    wx.navigateTo({ url: '/pages/interviews/interviews?id=' + e.currentTarget.dataset.id });
  },
  onJob() {
    // 从职位详情点进来的就退回去，别在栈里叠两份职位详情
    const pages = getCurrentPages();
    const prev = pages[pages.length - 2];
    if (prev && prev.route === 'pages/job/job' && prev.id === this.data.s.job) wx.navigateBack();
    else wx.navigateTo({ url: '/pages/job/job?id=' + this.data.s.job });
  }
});
