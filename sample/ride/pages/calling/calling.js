const app = getApp();
const DRIVERS = [
  { name: '王师傅', surname: '王', plate: '京A·D7K29', model: '比亚迪 秦PLUS DM-i', color: '白色', rating: '4.96', orders: 2316, distance: '1.2公里', eta: 3 },
  { name: '刘师傅', surname: '刘', plate: '京N·5T8E2', model: '丰田 凯美瑞', color: '黑色', rating: '4.92', orders: 4108, distance: '800米', eta: 2 },
  { name: '张师傅', surname: '张', plate: '京Q·M2R67', model: '大众 帕萨特', color: '灰色', rating: '4.98', orders: 1587, distance: '1.6公里', eta: 4 }
];

function pad(n) { return n < 10 ? '0' + n : '' + n; }

Page({
  data: { stage: 'waiting', elapsed: 0, elapsedText: '00:00', nearby: 12, order: null, driver: DRIVERS[0] },
  onLoad() {
    const order = wx.getStorageSync('ride_order') || {
      from: { name: '望京SOHO T1-东门' }, to: { name: '三里屯太古里' }, type: '快车', km: 8.6, min: 25, price: '33.7'
    };
    const driver = order.type === '专车' ? DRIVERS[1] : order.type === '拼车' ? DRIVERS[2] : DRIVERS[0];
    this.setData({ order, driver });
    this.timer = setInterval(() => {
      const elapsed = this.data.elapsed + 1;
      this.setData({ elapsed, elapsedText: pad(Math.floor(elapsed / 60)) + ':' + pad(elapsed % 60) });
      if (elapsed >= 4 && this.data.stage === 'waiting') this.accept();
    }, 1000);
  },
  onUnload() { clearInterval(this.timer); },
  accept() {
    clearInterval(this.timer);
    this.setData({ stage: 'accepted' });
    wx.setNavigationBarTitle({ title: '等待接驾' });
    wx.vibrateShort && wx.vibrateShort({ type: 'light' });
  },
  onCancel() {
    const accepted = this.data.stage === 'accepted';
    wx.showModal({
      title: accepted ? '确定取消行程？' : '确定取消呼叫？',
      content: accepted ? '司机已在赶来的路上，接单 2 分钟后取消可能产生 5 元取消费' : '附近车辆较多，再等一会儿很快就能应答',
      confirmText: '确定取消', cancelText: '再等等',
      success: (r) => {
        if (!r.confirm) return;
        clearInterval(this.timer);
        wx.showToast({ title: '已取消', icon: 'none' });
        setTimeout(() => wx.navigateBack(), 600);
      }
    });
  },
  onCallDriver() { wx.showToast({ title: '正在呼叫 ' + this.data.driver.name + '（虚拟号）', icon: 'none' }); },
  onMessage() {
    wx.showActionSheet({
      itemList: ['我在路边等你', '请稍等，马上到', '我在上车点了', '麻烦开到小区门口'],
      success: (r) => wx.showToast({ title: '已发送', icon: 'success' })
    });
  },
  onSafety() {
    wx.showActionSheet({ itemList: ['分享行程给紧急联系人', '一键报警 110', '录音保护（已开启）'] });
  },
  onMore() { wx.showActionSheet({ itemList: ['修改目的地', '联系客服', '查看计价规则'] }); },
  onFinish() {
    const o = this.data.order, d = this.data.driver;
    const now = new Date();
    const trip = {
      id: 'T' + now.getTime(), type: o.type, from: o.from.name, to: o.to.name,
      date: (now.getMonth() + 1) + '月' + now.getDate() + '日 ' + pad(now.getHours()) + ':' + pad(now.getMinutes()),
      km: o.km, min: o.min, status: 'done', plate: d.plate, driver: d.name, car: d.model + ' · ' + d.color
    };
    app.addTrip(trip);
    wx.removeStorageSync('ride_to');
    wx.redirectTo({ url: '/pages/trip-detail/trip-detail?id=' + trip.id + '&paid=1' });
  }
});
