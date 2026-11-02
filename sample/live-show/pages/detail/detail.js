const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');
const store = require('../../utils/store.js');

const TABS = ['场次票档', '演出介绍', '购票须知', '场馆'];
const TMPL = 'XC_SALE_REMIND_TMPL_01';

Page({
  data: {
    show: null, tabs: TABS, active: 0, fixedTabs: false,
    ses: 0, tier: -1, qty: 1, total: 0,
    onSale: true, cd: null, reminded: false, want: false, soldOut: false, notice: D.NOTICE, wantText: ''
  },
  onLoad(q) {
    const show = D.getShow(q.id) || D.SHOWS[0];
    this.show = show;
    const soldOut = show.tiers.every((t) => t.soldOut);
    this.setData({
      show, soldOut, want: store.isWant(show.id), wantText: F.wantText(show.want),
      tier: show.seat ? -1 : show.tiers.findIndex((t) => !t.soldOut)
    });
    wx.setNavigationBarTitle({ title: show.artist.length > 10 ? '演出详情' : show.artist });
    this.navDark = true;
    this.lastNav = 0;
    this.tick();
    this.calc();
  },
  onReady() {
    // 量吸顶 tab 的原始位置；引擎不支持 position: sticky 时靠它切到 fixed 的那份
    wx.createSelectorQuery().select('#tabs').boundingClientRect((r) => { if (r) this.tabsTop = r.top; }).exec();
    const winH = wx.getWindowInfo().windowHeight || 667;
    // 视口顶部 44~120px 这条带子里出现的分区就是当前分区
    this.io = this.createIntersectionObserver({ observeAll: true });
    this.io.relativeToViewport({ top: -44, bottom: -(winH - 120) }).observe('.sec', (res) => {
      if (res.intersectionRatio > 0 && !this.jumping) this.setData({ active: Number(res.dataset.i) });
    });
  },
  onShow() {
    this.setData({ reminded: !!store.reminds()[this.show.id] });
    if (!this.timer && !this.data.onSale) this.timer = setInterval(() => this.tick(), 1000);
  },
  onHide() { this.stopTimer(); },
  onUnload() { this.stopTimer(); if (this.io) this.io.disconnect(); },
  stopTimer() { if (this.timer) { clearInterval(this.timer); this.timer = null; } },
  tick() {
    const left = this.show.saleAt - Date.now();
    if (left <= 0) {
      this.stopTimer();
      if (!this.data.onSale || this.data.cd) this.setData({ onSale: true, cd: null });
      return;
    }
    this.setData({ onSale: false, cd: F.countdown(left), saleText: F.monthDay(this.show.saleAt) });
    if (!this.timer) this.timer = setInterval(() => this.tick(), 1000);
  },
  // 导航栏随滚动变色（节流 100ms，只在跨过阈值时调一次）
  onPageScroll(e) {
    const top = e.scrollTop;
    const fixed = this.tabsTop !== undefined && top >= this.tabsTop;
    if (fixed !== this.data.fixedTabs) this.setData({ fixedTabs: fixed });
    const now = Date.now();
    if (now - this.lastNav < 100) return;
    this.lastNav = now;
    const dark = top < 160;
    if (dark === this.navDark) return;
    this.navDark = dark;
    wx.setNavigationBarColor({
      frontColor: dark ? '#ffffff' : '#000000',
      backgroundColor: dark ? '#1b1b24' : '#ffffff',
      animation: { duration: 200, timingFunc: 'easeIn' }
    });
  },
  jump(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.jumping = true;
    this.setData({ active: i });
    wx.pageScrollTo({ selector: '#sec-' + i, offsetTop: -50, duration: 300 });
    setTimeout(() => { this.jumping = false; }, 450);
  },
  pickSes(e) { this.setData({ ses: Number(e.currentTarget.dataset.i) }); },
  pickTier(e) {
    const i = Number(e.currentTarget.dataset.i);
    const t = this.show.tiers[i];
    if (t.soldOut) { wx.showToast({ title: '该票档已售罄', icon: 'none' }); return; }
    if (this.show.seat) { wx.showToast({ title: '在选座页按区域选座', icon: 'none' }); return; }
    this.setData({ tier: i });
    this.calc();
  },
  minus() { if (this.data.qty > 1) { this.setData({ qty: this.data.qty - 1 }); this.calc(); } },
  plus() {
    if (this.data.qty >= 4) { wx.showToast({ title: '每单限购 4 张', icon: 'none' }); return; }
    this.setData({ qty: this.data.qty + 1 }); this.calc();
  },
  calc() {
    const t = this.show.tiers[this.data.tier];
    this.setData({ total: t ? t.price * this.data.qty : 0 });
  },
  toggleWant() {
    const on = store.toggleWant(this.show.id);
    this.setData({ want: on });
    wx.showToast({ title: on ? '已加入想看' : '已取消想看', icon: 'none' });
  },
  remind() {
    if (this.data.reminded) {
      wx.showModal({
        title: '取消开售提醒', content: '取消后开售时将不再通知你', confirmText: '取消提醒', cancelText: '保留',
        success: (r) => { if (r.confirm) { store.clearRemind(this.show.id); this.setData({ reminded: false }); } }
      });
      return;
    }
    const id = this.show.id;
    wx.requestSubscribeMessage({
      tmplIds: [TMPL],
      success: (res) => {
        const ok = res[TMPL] === 'accept';
        store.setRemind(id, ok ? 'wechat' : 'local');
        this.setData({ reminded: true });
        wx.showToast({ title: ok ? '开售时微信通知你' : '已设本机提醒', icon: 'none' });
      },
      fail: () => {
        // 订阅消息不可用：记在本机，下次打开开售日历 / 想看页会标出来
        store.setRemind(id, 'local');
        if (!store.isWant(id)) { store.toggleWant(id); this.setData({ want: true }); }
        this.setData({ reminded: true });
        wx.showToast({ title: '已设本机开售提醒', icon: 'none' });
      }
    });
  },
  buy() {
    const s = this.show;
    if (this.data.soldOut) return;
    if (!this.data.onSale) { this.remind(); return; }
    const ses = s.sessions[this.data.ses];
    if (s.seat) {
      wx.navigateTo({ url: '/pages/seat/seat?id=' + s.id + '&ses=' + ses.id });
      return;
    }
    const t = s.tiers[this.data.tier];
    if (!t) { wx.showToast({ title: '请选择票档', icon: 'none' }); return; }
    const items = [];
    for (let i = 0; i < this.data.qty; i++) items.push({ tier: t.id, tierName: t.name, price: t.price, label: t.name });
    store.setDraft({ showId: s.id, sessionId: ses.id, items });
    wx.navigateTo({ url: '/pages/confirm/confirm' });
  },
  venueMore() {
    const s = this.show;
    wx.showActionSheet({
      itemList: ['复制场馆地址', '拨打场馆电话 ' + s.phone],
      success: (r) => {
        if (r.tapIndex === 0) wx.setClipboardData({ data: s.venue + ' ' + s.addr });
        else wx.makePhoneCall({ phoneNumber: s.phone, fail: () => wx.setClipboardData({ data: s.phone, success: () => wx.showToast({ title: '号码已复制', icon: 'none' }) }) });
      }
    });
  },
  onShareAppMessage() {
    return { title: this.show.title, path: '/pages/detail/detail?id=' + this.show.id, imageUrl: this.show.poster };
  }
});
