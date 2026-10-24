const app = getApp();
Page({
  data: { trip: {}, fee: {}, surname: '', paid: false, rated: false },
  onLoad(q) {
    const trips = app.getTrips();
    const trip = trips.find(t => t.id === q.id) || trips[0];
    const f = app.fees(trip);
    const fee = { base: f.base.toFixed(1), mileage: f.mileage.toFixed(1), time: f.time.toFixed(1), far: f.far, coupon: f.coupon, total: f.total.toFixed(1) };
    const rated = (wx.getStorageSync('ride_rated') || []).indexOf(trip.id) >= 0;
    this.setData({ trip, fee, surname: trip.driver ? trip.driver.charAt(0) : '', paid: q.paid === '1', rated });
  },
  onRate() {
    if (this.data.rated) return;
    wx.showActionSheet({
      itemList: ['★★★★★ 非常满意', '★★★★ 满意', '★★★ 一般', '★★ 不满意'],
      success: () => {
        const list = wx.getStorageSync('ride_rated') || [];
        list.push(this.data.trip.id);
        wx.setStorageSync('ride_rated', list);
        this.setData({ rated: true });
        wx.showToast({ title: '感谢评价', icon: 'success' });
      }
    });
  },
  onRule() {
    wx.showModal({ title: this.data.trip.type + '计价规则', content: '起步价含 0 公里；超出部分按里程费 + 时长费计算；单程超过 15 公里加收远途费 0.8 元/公里。', showCancel: false });
  },
  onInvoice() { wx.showToast({ title: '发票将发送至邮箱', icon: 'none' }); },
  onComplain() { wx.showActionSheet({ itemList: ['司机绕路', '车内有异味', '多收费用', '其他问题'] }); }
});
