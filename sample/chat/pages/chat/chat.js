const store = require('../../utils/store.js');
const EMOJI = ['微笑', '撇嘴', '色', '发呆', '得意', '流泪', '害羞', '闭嘴', '睡', '大哭', '尴尬', '发怒', '调皮', '呲牙', '惊讶', '难过', '酷', '冷汗', '抓狂', '偷笑', '愉快', '白眼', '傲慢', '困'];
const EMOJI_COLORS = ['#FFC53D', '#FFD666', '#FFB020', '#FFCF5C'];
const PHRASES = ['好的，收到', '稍等，我马上来', '在开会，晚点回复你', '谢谢！辛苦了', '周末一起吃饭吗？', '路上堵车，晚到十分钟'];

Page({
  data: {
    rows: [], draft: '', canSend: false, intoView: '', typing: false,
    panel: false, panelTab: 0, scrollH: 500, barH: 56, kbH: 0, me: {}, peer: {},
    emojis: EMOJI.map((n, i) => ({ name: n, short: n.slice(0, 2), color: EMOJI_COLORS[i % 4] })),
    phrases: PHRASES
  },
  onLoad(q) {
    this.id = q.id || 'c19';
    const c = store.contact(this.id) || store.CONTACTS[0];
    const p = store.profile();
    this.peerName = c.remark ? c.name + '（' + c.remark + '）' : c.name;
    wx.setNavigationBarTitle({ title: this.peerName });
    const sys = wx.getSystemInfoSync();
    this.rpx = sys.windowWidth / 750;
    this.winH = sys.windowHeight;
    this.timers = [];
    this.pending = null;
    this.setData({ peer: { initial: c.initial, color: c.color }, me: { initial: p.nickname.slice(0, 1), color: p.color } });
    this.msgs = store.messages(this.id);
    this.markRead();
    this.layout();
    this.render(true);
  },
  onUnload() {
    this.timers.forEach((t) => clearTimeout(t));
    // 离开时对方还没回完：回复照样到达，只是记成未读
    if (this.pending) this.deliver(this.pending, true);
  },
  markRead() {
    const all = store.sessions(); const s = all.find((x) => x.id === this.id);
    if (s && s.unread) { s.unread = 0; store.saveSessions(all); }
  },
  layout() {
    const bar = Math.round(112 * this.rpx);
    const panel = this.data.panel ? Math.round(440 * this.rpx) : 0;
    // 键盘弹着时列表让出键盘那么高（输入框 adjust-position=false，画面不整体上移）：
    // 输入条贴着键盘、最新消息还在眼前，像原生聊天
    const below = Math.max(panel, this.data.kbH);
    this.setData({ barH: bar, scrollH: this.winH - bar - below });
  },
  onKbHeight(e) {
    const h = Math.max(0, Math.round(Number(e.detail.height) || 0));
    if (h === this.data.kbH) return;
    this.setData({ kbH: h });
    this.layout();
    if (h > 0) this.scrollToEnd();
  },
  render(scroll) {
    const now = Date.now();
    const rows = [];
    let prev = 0;
    this.msgs.forEach((m) => {
      if (m.time - prev > 5 * 60 * 1000) rows.push({ key: 't' + m.id, kind: 'time', text: store.fmtFull(m.time, now) });
      prev = m.time;
      if (m.recalled) rows.push({ key: m.id, kind: 'sys', text: m.from === 'me' ? '你撤回了一条消息' : '对方撤回了一条消息' });
      else rows.push({ key: m.id, kind: 'msg', mine: m.from === 'me', text: m.text });
    });
    const data = { rows };
    if (scroll) data.intoView = this.data.typing ? 'typing' : (rows.length ? rows[rows.length - 1].key : '');
    this.setData(data);
  },
  scrollToEnd() {
    // 先清空再设：scroll-into-view 值不变时不会再滚
    const last = this.data.typing ? 'typing' : (this.data.rows.length ? this.data.rows[this.data.rows.length - 1].key : '');
    this.setData({ intoView: '' });
    this.setData({ intoView: last });
  },
  onInput(e) {
    const v = e.detail.value;
    this.setData({ draft: v, canSend: v.trim().length > 0 });
  },
  onFocus() { if (this.data.panel) { this.setData({ panel: false }); this.layout(); this.scrollToEnd(); } },
  togglePanel() {
    this.setData({ panel: !this.data.panel });
    this.layout();
    this.scrollToEnd();
  },
  switchPanelTab(e) { this.setData({ panelTab: Number(e.currentTarget.dataset.i) }); },
  addEmoji(e) {
    const v = this.data.draft + '[' + e.currentTarget.dataset.name + ']';
    this.setData({ draft: v, canSend: true });
  },
  backspace() {
    let v = this.data.draft;
    if (!v) return;
    const m = v.match(/\[[^\[\]]+\]$/);
    v = m ? v.slice(0, v.length - m[0].length) : v.slice(0, -1);
    this.setData({ draft: v, canSend: v.trim().length > 0 });
  },
  sendPhrase(e) { this.push(this.data.phrases[e.currentTarget.dataset.i]); },
  send() {
    if (!this.data.canSend) return;
    const text = this.data.draft.trim();
    this.setData({ draft: '', canSend: false });
    this.push(text);
  },
  push(text) {
    const msg = { id: 'm' + Date.now() + '_' + this.msgs.length, from: 'me', text, time: Date.now() };
    this.msgs.push(msg);
    store.saveMessages(this.id, this.msgs);
    store.touchSession(this.id, msg, false);
    this.render(true);
    this.scheduleReply(text);
  },
  pickReply(text) {
    const c = store.contact(this.id);
    const KEY = [['几点', '上午十点，3 楼大会议室'], ['在吗', '在的，怎么了？'], ['吃饭', '好呀，去哪吃？'], ['周末', '周末我有空'], ['收到', '[OK]']];
    const hit = KEY.find((k) => text.indexOf(k[0]) >= 0);
    if (hit) return hit[1];
    if (/[?？吗呢]$/.test(text)) return c.replies[0];
    if (text.indexOf('[') >= 0) return '[呲牙]';
    if (text.indexOf('谢') >= 0) return '不客气～';
    this.replyIdx = ((this.replyIdx || 0) + 1) % c.replies.length;
    return c.replies[this.replyIdx];
  },
  scheduleReply(text) {
    this.timers.forEach((t) => clearTimeout(t));
    this.timers = [];
    const reply = this.pickReply(text);
    this.pending = reply;
    this.timers.push(setTimeout(() => {
      this.setData({ typing: true });
      wx.setNavigationBarTitle({ title: '对方正在输入…' });
      this.scrollToEnd();
    }, 400));
    this.timers.push(setTimeout(() => {
      this.pending = null;
      this.setData({ typing: false });
      wx.setNavigationBarTitle({ title: this.peerName });
      this.deliver(reply, false);
    }, 400 + Math.min(1600, 1000 + reply.length * 40)));
  },
  deliver(text, away) {
    const msg = { id: 'm' + Date.now() + '_' + this.msgs.length, from: 'them', text, time: Date.now() };
    this.msgs.push(msg);
    store.saveMessages(this.id, this.msgs);
    store.touchSession(this.id, msg, away);
    this.pending = null;
    if (!away) this.render(true);
  },
  // 同会话列表：长按记下，抬手再弹菜单（引擎会把长按那次抬手当成点遮罩，菜单一弹就关）
  onMsgLong(e) { this.longKey = e.currentTarget.dataset.key; },
  onMsgEnd() {
    if (!this.longKey) return;
    const key = this.longKey; this.longKey = '';
    this.msgMenu(key);
  },
  msgMenu(key) {
    const m = this.msgs.find((x) => x.id === key);
    if (!m) return;
    const canRecall = m.from === 'me' && Date.now() - m.time < 2 * 60 * 1000;
    const items = canRecall ? ['复制', '撤回', '删除'] : ['复制', '删除'];
    wx.showActionSheet({
      itemList: items,
      success: (r) => {
        const act = items[r.tapIndex];
        if (act === '复制') wx.setClipboardData({ data: m.text });
        else if (act === '撤回') { m.recalled = true; this.afterEdit('你撤回了一条消息'); }
        else { this.msgs = this.msgs.filter((x) => x.id !== key); this.afterEdit(null); }
      }
    });
  },
  afterEdit(summary) {
    store.saveMessages(this.id, this.msgs);
    const all = store.sessions(); const s = all.find((x) => x.id === this.id);
    const last = this.msgs[this.msgs.length - 1];
    if (s) { s.lastText = last ? (last.recalled ? (last.from === 'me' ? '你撤回了一条消息' : '对方撤回了一条消息') : last.text) : ''; store.saveSessions(all); }
    this.render(false);
  },
  openPeer() { wx.navigateTo({ url: '/pages/contact/contact?id=' + this.id }); }
});
