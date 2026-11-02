const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: {
    hospital: data.hospital,
    entries: [
      { key: 'book', name: '预约挂号', ch: '挂', bg: 'linear-gradient(135deg, #5aa2ff, #1f7cf0)' },
      { key: 'today', name: '当日挂号', ch: '今', bg: 'linear-gradient(135deg, #37d2c0, #13a8a8)' },
      { key: 'report', name: '报告查询', ch: '报', bg: 'linear-gradient(135deg, #a18cff, #7b6cf6)' },
      { key: 'consult', name: '在线问诊', ch: '诊', bg: 'linear-gradient(135deg, #ffb26b, #ff8240)' },
      { key: 'pay', name: '门诊缴费', ch: '缴', bg: 'linear-gradient(135deg, #ff8e9e, #f2557a)' },
      { key: 'inpatient', name: '住院服务', ch: '住', bg: 'linear-gradient(135deg, #63c2ff, #2a9df4)' },
      { key: 'patients', name: '就诊人', ch: '人', bg: 'linear-gradient(135deg, #7fd88e, #3cb35a)' },
      { key: 'guide', name: '智能导诊', ch: '导', bg: 'linear-gradient(135deg, #ffd36b, #f5a623)' }
    ],
    // go：点横幅（或横幅上的按钮）去哪
    banners: [
      { title: '秋季流感疫苗开约', sub: '成人四价 · 门诊 1 楼预防接种门诊', btn: '立即预约', bg: 'linear-gradient(120deg, #3b8cff, #22c1c3)', go: '/pages/doctors/doctors?dept=ym' },
      { title: '护心行动 · 免费义诊', sub: '本周六 08:30 门诊大厅 心内科专家', btn: '查看详情', bg: 'linear-gradient(120deg, #ff8a5b, #ff5f7e)', go: '/pages/news/news?id=2' },
      { title: '体检报告免费解读', sub: '全科医生在线 1 对 1 · 30 分钟内回复', btn: '去咨询', bg: 'linear-gradient(120deg, #7b6cf6, #4f8cff)', go: '/pages/consult/consult' }
    ],
    experts: [],
    news: data.news,
    noticeText: '',
    noticeTag: '提醒',
    badges: {}
  },
  onLoad() {
    const experts = data.doctors.filter(d => d.star).map(d => Object.assign({}, d, { deptName: data.findDept(d.dept).name }));
    this.setData({ experts });
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    store.tick();
    const unpaid = store.unpaidBills();
    const active = store.getOrders().filter(store.isActive);
    const checked = active.find(o => o.status === 'checked');
    let text = '就诊当天请携带医保卡，提前 15 分钟到诊区签到';
    let tag = '提醒';
    this.noticeGo = '';
    if (unpaid.length) {
      tag = '缴费';
      text = '有 ' + unpaid.length + ' 笔待缴费用：' + unpaid[0].title + ' ¥' + unpaid[0].self;
      this.noticeGo = '/pages/bills/bills';
    } else if (checked) {
      const st = store.stageOf(checked);
      tag = '候诊';
      text = checked.patient + ' · ' + checked.dept + '：' + st.tip;
      this.noticeGo = '/pages/order-detail/order-detail?id=' + checked.id;
    } else if (active.length) {
      const o = active[0];
      text = '待就诊 ' + active.length + ' 个：' + o.date.slice(5) + ' ' + o.time.slice(0, 5) + ' ' + o.dept + ' ' + o.doctor;
      this.noticeGo = '/pages/order-detail/order-detail?id=' + o.id;
    }
    this.setData({
      noticeText: text,
      noticeTag: tag,
      badges: { pay: unpaid.length, report: store.unreadReports() }
    });
  },
  onEntry(e) {
    const key = e.currentTarget.dataset.key;
    const pages = {
      today: '/pages/doctors/doctors?today=1',
      report: '/pages/reports/reports',
      consult: '/pages/consult/consult',
      pay: '/pages/bills/bills',
      inpatient: '/pages/inpatient/inpatient',
      patients: '/pages/patients/patients',
      guide: '/pages/guide/guide'
    };
    if (pages[key]) wx.navigateTo({ url: pages[key] });
    else wx.switchTab({ url: '/pages/dept/dept' });
  },
  goDept() { wx.switchTab({ url: '/pages/dept/dept' }); },
  goNotice() {
    if (this.noticeGo) wx.navigateTo({ url: this.noticeGo });
    else wx.switchTab({ url: '/pages/orders/orders' });
  },
  goDoctor(e) { wx.navigateTo({ url: '/pages/doctor/doctor?id=' + e.currentTarget.dataset.id }); },
  onBanner(e) { wx.navigateTo({ url: this.data.banners[e.currentTarget.dataset.i].go }); },
  callHos() { host.call(data.hospital.phone, '医院总机'); },
  goNavi() { wx.navigateTo({ url: '/pages/navi/navi' }); },
  openNews(e) { wx.navigateTo({ url: '/pages/news/news?id=' + e.currentTarget.dataset.id }); }
});
