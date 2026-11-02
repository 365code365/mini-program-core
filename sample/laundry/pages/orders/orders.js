const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const tab = require('../../utils/tab.js');

// cur：-2 全部，0~3 对应 data.STATUS
Page({
  data: { tabs: [], cur: -2, list: [], loading: true, skel: [1, 2, 3] },

  onShow() {
    tab.sync(this, 2);
    const app = getApp();
    if (app.globalData.orderTab !== -2) { this.setData({ cur: app.globalData.orderTab }); app.globalData.orderTab = -2; }
    this.load(!this.loaded);
    this.loaded = true;
  },

  onPullDownRefresh() {
    this.load(true, () => wx.stopPullDownRefresh());
  },

  onTab(e) {
    const key = e.currentTarget.dataset.key;
    if (key === this.data.cur) return;
    this.setData({ cur: key });
    this.load(true);
  },

  // 首次进入 / 切状态先出 300ms 骨架（真实场景是拉接口）
  load(withSkeleton, done) {
    const build = () => {
      const all = store.orders();
      const counts = [0, 0, 0, 0];
      all.forEach(o => { const m = store.mainStatus(o); if (m >= 0 && m < 3) counts[m]++; });
      const tabs = [{ key: -2, name: '全部', n: 0 }].concat(data.STATUS.map(s => ({ key: s.key, name: s.name, n: s.key < 3 ? counts[s.key] : 0 })));
      const cur = this.data.cur;
      const list = all.filter(o => cur === -2 || store.mainStatus(o) === cur).map(o => ({
        id: o.id, bagNo: o.bagNo, count: o.count, payable: o.payable, main: store.mainStatus(o), statusName: store.statusName(o),
        names: o.lines.map(l => l.name + '×' + l.qty).slice(0, 3).join('、') + (o.lines.length > 3 ? ' 等' : ''),
        stepText: o.cancelled ? '已取消 · ' + (o.cancelReason || '') : data.STEPS[o.step].text,
        timeText: store.timeStr(o.createdAt).slice(5),
        canReview: !o.cancelled && o.step === 8 && !o.review
      }));
      this.setData({ tabs, list, loading: false });
      done && done();
    };
    if (!withSkeleton) return build();
    this.setData({ loading: true });
    clearTimeout(this.t);
    this.t = setTimeout(build, 300);
  },

  open(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  review(e) { wx.navigateTo({ url: '/pages/review/review?id=' + e.currentTarget.dataset.id }); },
  goPrice() { wx.switchTab({ url: '/pages/price/price' }); }
});
