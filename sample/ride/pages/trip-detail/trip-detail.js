// 行程详情：已完成（费用明细、支付方式、评价、开票、投诉）/ 已取消（原因、取消费）/ 待支付（去支付）。
// 评价、投诉、开票都写回 store 里同一份行程
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const STAR_TEXT = ['', '非常差', '不满意', '一般', '满意', '非常满意'];
const STAGE = { waiting: '司机接单前', booked: '预约单司机出发前', accepted: '司机赶来途中', arrived: '司机到达上车点后' };

Page({
  data: {
    trip: null, hero: {}, lines: [], payName: '', paidTime: '', surname: '', canCall: false, due: 0, cancelText: '', cancelLines: [],
    invoice: null, starList: [1, 2, 3, 4, 5], stars: 5, starText: STAR_TEXT[5], rateTags: [], rateText: ''
  },
  onLoad(q) {
    this.id = q.id || '';
    this.wantRate = q.rate === '1';
    this.setData({ rateTags: data.RATE_TAGS.map(name => ({ name, on: false })) });
  },
  onShow() {
    this.load();
  },
  load() {
    const t = store.findTrip(this.id);
    if (!t) {
      this.setData({ trip: null });
      return;
    }
    const lines = [];
    if (t.fare) {
      lines.push({ name: '起步价', value: '¥' + t.fare.base });
      lines.push({ name: '里程费（' + t.km + ' 公里）', value: '¥' + t.fare.mileage });
      lines.push({ name: '时长费（' + t.min + ' 分钟）', value: '¥' + t.fare.time });
      if (t.fare.far) lines.push({ name: '远途费', value: '¥' + t.fare.far });
      if (t.fare.discount) lines.push({ name: '拼成 82 折', value: '-¥' + t.fare.discount, cut: true });
      if (t.status === 'done' && t.couponCut) lines.push({ name: t.couponTitle || '优惠券', value: '-¥' + t.couponCut, cut: true });
    }
    let hero = { label: '行程已完成', num: t.pay, sub: t.date + ' · ' + t.type };
    if (t.status === 'unpaid') hero = { label: t.cancelled ? '取消费待支付' : '行程已结束 · 待支付', num: store.dueOf(t), sub: hero.sub };
    else if (t.status === 'cancel') hero = { label: t.cancelFee ? '行程已取消 · 已付取消费' : '行程已取消', num: t.cancelFee ? t.cancelFee : '', sub: hero.sub };
    // 取消说明分行显示：引擎的中文换行不避头，整段排时句号会落到行首
    let cancelLines = [];
    if (t.cancelled || t.status === 'cancel') {
      const when = STAGE[t.cancelStage] || '';
      cancelLines = [
        (when ? when + '取消' : '已取消') + (t.cancelReason ? '：' + t.cancelReason : ''),
        t.cancelFee ? '按规则收取取消费 ¥' + t.cancelFee + (t.status === 'cancel' ? '，已支付' : '，尚未支付') : '未产生任何费用'
      ];
    }
    const cancelText = cancelLines.join('；');
    const inv = t.invoiceId ? store.getInvoices().find(i => i.id === t.invoiceId) || null : null;
    // 隐私号在行程结束 24 小时内有效（遗落物品找司机）
    const end = t.paidAt || t.finishedAt || 0;
    this.setData({
      trip: t, hero, lines, cancelText, cancelLines, invoice: inv, due: store.dueOf(t),
      payName: t.payBy === 'wallet' ? '钱包余额' : '微信支付',
      paidTime: t.paidAt ? store.dateTime(t.paidAt) : '',
      surname: t.driver && t.driver !== '-' ? t.driver.charAt(0) : '',
      canCall: !!t.driverPhone && end > 0 && Date.now() - end < 86400000
    });
    if (this.wantRate && t.status === 'done' && !t.rating) {
      this.wantRate = false;
      wx.pageScrollTo({ selector: '.rate', duration: 200 });
    }
  },
  // ───────── 评价 ─────────
  // 四星以上选「哪里好」，三星及以下换成「哪里不满意」（换组时清掉已选）
  onStar(e) {
    const stars = Number(e.currentTarget.dataset.n);
    const good = stars >= 4;
    const patch = { stars, starText: STAR_TEXT[stars] };
    if (good !== this.data.stars >= 4) patch.rateTags = (good ? data.RATE_TAGS : data.RATE_BAD).map(name => ({ name, on: false }));
    this.setData(patch);
  },
  onRateTag(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ ['rateTags[' + i + '].on']: !this.data.rateTags[i].on });
  },
  onRateText(e) {
    this.setData({ rateText: e.detail.value });
  },
  onSubmitRate() {
    const d = this.data;
    const tags = d.rateTags.filter(t => t.on).map(t => t.name);
    if (d.stars <= 3 && !tags.length && !d.rateText.trim()) {
      wx.showToast({ title: '选一下哪里不满意，帮我们改进', icon: 'none' });
      return;
    }
    store.updateTrip(this.id, { rating: { stars: d.stars, tags, text: d.rateText.trim(), time: store.dateTime(Date.now()) } });
    wx.showToast({ title: '感谢评价', icon: 'success' });
    this.load();
  },
  // ───────── 费用 / 支付 ─────────
  onRule() {
    const t = this.data.trip;
    wx.showModal({ title: t.type + '计价规则', content: data.ruleText(t.typeKey), showCancel: false, confirmText: '知道了' });
  },
  onPay() {
    wx.redirectTo({ url: '/pages/pay/pay?id=' + this.id });
  },
  // ───────── 开票 / 投诉 / 联系 ─────────
  onInvoice() {
    const d = this.data;
    if (d.invoice) {
      wx.showModal({
        title: '已开具电子发票',
        content: '发票抬头：' + d.invoice.title + '\n金额：¥' + d.invoice.amount + '\n已发送至 ' + d.invoice.email,
        confirmText: '开票记录',
        cancelText: '关闭',
        success: r => {
          if (r.confirm) wx.navigateTo({ url: '/pages/invoice/invoice?tab=list' });
        }
      });
      return;
    }
    wx.navigateTo({ url: '/pages/invoice/invoice?ids=' + this.id });
  },
  onComplain() {
    const t = this.data.trip;
    if (t.complaint) {
      wx.showModal({
        title: '投诉处理中',
        content: '你在 ' + t.complaint.time + ' 反馈了「' + t.complaint.reason + '」，客服会在 24 小时内联系你。',
        confirmText: '联系客服',
        cancelText: '关闭',
        success: r => {
          if (r.confirm) host.call(data.SERVICE_PHONE, '滴达客服（7×24 小时）');
        }
      });
      return;
    }
    wx.showActionSheet({
      itemList: data.COMPLAIN_REASONS,
      success: r => {
        const reason = data.COMPLAIN_REASONS[r.tapIndex];
        store.updateTrip(this.id, { complaint: { reason, time: store.dateTime(Date.now()), status: '处理中' } });
        wx.showToast({ title: reason === '多收费用' ? '已提交，核实后差额原路退回' : '已提交，客服 24 小时内联系你', icon: 'none' });
        this.load();
      }
    });
  },
  onCallDriver() {
    const t = this.data.trip;
    host.call(t.driverPhone, '联系' + t.driver + '（隐私号）');
  },
  onCopy() {
    host.copy(this.data.trip.id);
  },
  // 再叫一单 / 返程：起终点、车型带回首页
  onAgain(e) {
    const t = this.data.trip;
    const back = e.currentTarget.dataset.back === 'yes';
    store.setFrom(back ? t.to : t.from);
    store.setTo(back ? t.from : t.to);
    store.setType(t.typeKey);
    wx.switchTab({ url: '/pages/index/index' });
  }
});
