// 私信：文本 + 出价卡片 + 交易状态消息；卖家回复是本地模拟（按出价比例同意 / 还价）。
// 新消息后 scroll-into-view 滚到最后一条；顶部交易卡显示宝贝与订单状态
const store = require('../../utils/store.js');
const D = require('../../utils/data.js');
const AUTO = [
  [/在吗|还在|有吗/, '在的，还没出，喜欢可以直接拍'],
  [/便宜|少点|优惠|刀/, '已经很实在了，诚心要的话可以出个价'],
  [/包邮|邮费|快递/, '同城可以自提，外地发顺丰到付或者包邮加 10 元'],
  [/验|视频|照片|图/, '可以的，晚上下班给你拍个视频'],
  [/面交|自提|当面|地铁/, '可以面交，工作日晚上 7 点后、周末都行']
];
Page({
  data: { conv: null, item: null, peer: null, me: D.ME, msgs: [], text: '', last: '', order: null, offerShow: false, offerPrice: '' },
  onLoad(q) { this.cid = q.id; this.load(); store.markRead(this.cid); },
  onUnload() { clearTimeout(this.replyTimer); },
  load() {
    const conv = store.convOf(this.cid);
    if (!conv) return;
    const item = store.findItem(conv.item);
    const peer = D.SELLERS[conv.peer];
    const msgs = conv.msgs.map((m, i) => Object.assign({}, m, { i, key: 'm' + i, timeText: store.timeText(m.ts) }));
    const o = store.orderForItem(conv.item);
    const order = o ? Object.assign({}, o, { statusText: store.STATUS[o.status] }) : null;
    const quickList = item ? [['95 折', 0.95], ['9 折', 0.9], ['85 折', 0.85]].map(q => ({ label: q[0], v: Math.floor(item.price * q[1]) })) : [];
    this.setData({ conv, item, peer, msgs, quickList, last: msgs.length ? 'm' + (msgs.length - 1) : '', order });
    wx.setNavigationBarTitle({ title: peer ? peer.name : '私信' });
  },
  onInput(e) { this.setData({ text: e.detail.value }); },
  send() {
    const t = this.data.text.trim();
    if (!t) return;
    store.pushMsg(this.cid, { from: 'me', type: 'text', text: t });
    this.setData({ text: '' });
    this.load();
    this.autoReply(t);
  },
  autoReply(t) {
    const hit = AUTO.find(a => a[0].test(t));
    const reply = hit ? hit[1] : '好的，收到～';
    clearTimeout(this.replyTimer);
    this.replyTimer = setTimeout(() => { store.pushMsg(this.cid, { from: 'peer', type: 'text', text: reply }); store.markRead(this.cid); this.load(); }, 1200);
  },
  openOffer() { this.setData({ offerShow: true, offerPrice: '' }); },
  closeOffer() { this.setData({ offerShow: false }); },
  onOffer(e) { this.setData({ offerPrice: e.detail.value }); },
  quick(e) { this.setData({ offerPrice: String(e.currentTarget.dataset.v) }); },
  sendOffer() {
    const p = Math.round(parseFloat(this.data.offerPrice));
    const it = this.data.item;
    if (!(p > 0)) { wx.showToast({ title: '请输入出价', icon: 'none' }); return; }
    if (p >= it.price) { wx.showToast({ title: '出价不低于标价，可以直接购买', icon: 'none' }); return; }
    store.pushMsg(this.cid, { from: 'me', type: 'offer', price: p, state: 'pending' });
    const idx = store.convOf(this.cid).msgs.length - 1;
    this.setData({ offerShow: false });
    this.load();
    // 卖家：≥ 九折同意；八折以上还价到中间；再低婉拒
    this.replyTimer = setTimeout(() => {
      const r = p / it.price;
      if (r >= 0.9) {
        store.updateMsg(this.cid, idx, { state: 'accepted' });
        store.pushMsg(this.cid, { from: 'peer', type: 'text', text: '可以，¥' + p + ' 出给你，直接按这个价拍就行' });
      } else if (r >= 0.8) {
        const counter = Math.round((p + it.price) / 2);
        store.updateMsg(this.cid, idx, { state: 'declined' });
        store.pushMsg(this.cid, { from: 'peer', type: 'offer', price: counter, state: 'counter' });
      } else {
        store.updateMsg(this.cid, idx, { state: 'declined' });
        store.pushMsg(this.cid, { from: 'peer', type: 'text', text: '这个价太低了哈，最低 ¥' + Math.round(it.price * 0.92) });
      }
      store.markRead(this.cid);
      this.load();
    }, 1200);
  },
  buyAt(e) {
    const it = this.data.item;
    if (this.data.order) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + this.data.order.id }); return; }
    if (it.sold) { wx.showToast({ title: '宝贝已经卖出了', icon: 'none' }); return; }
    const price = e.currentTarget.dataset.price || it.price;
    wx.navigateTo({ url: '/pages/order/order?id=' + it.id + '&price=' + price });
  },
  goItem() { wx.navigateTo({ url: '/pages/detail/detail?id=' + this.data.item.id }); },
  onShow() { if (this.data.conv) this.load(); }
});
