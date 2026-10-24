const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

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
    banners: [
      { title: '秋季流感疫苗开约', sub: '成人四价 · 门诊 1 楼预防接种门诊', btn: '立即预约', bg: 'linear-gradient(120deg, #3b8cff, #22c1c3)' },
      { title: '护心行动 · 免费义诊', sub: '本周六 08:30 门诊大厅 心内科专家', btn: '查看详情', bg: 'linear-gradient(120deg, #ff8a5b, #ff5f7e)' },
      { title: '体检报告免费解读', sub: '全科医生在线 1 对 1 · 30 分钟内回复', btn: '去咨询', bg: 'linear-gradient(120deg, #7b6cf6, #4f8cff)' }
    ],
    experts: [],
    news: data.news,
    noticeText: ''
  },
  onLoad() {
    const experts = data.doctors.filter(d => d.title === '主任医师').map(d => Object.assign({}, d, { deptName: data.findDept(d.dept).name }));
    this.setData({ experts });
  },
  onShow() {
    const pending = store.getOrders().filter(o => o.status === 'wait');
    const text = pending.length
      ? '待就诊 ' + pending.length + ' 个：' + pending[0].date.slice(5) + ' ' + pending[0].time.slice(0, 5) + ' ' + pending[0].dept + ' ' + pending[0].doctor
      : '就诊当天请携带医保卡，提前 15 分钟到诊区签到';
    this.setData({ noticeText: text });
  },
  onEntry(e) {
    const key = e.currentTarget.dataset.key;
    if (key === 'book' || key === 'today' || key === 'guide') {
      wx.switchTab({ url: '/pages/dept/dept' });
    } else if (key === 'patients') {
      wx.navigateTo({ url: '/pages/patients/patients' });
    } else if (key === 'report') {
      wx.showModal({ title: '报告查询', content: '张明远 · 2024-10-15 血常规报告已出，建议复诊时携带。', confirmText: '知道了', showCancel: false });
    } else if (key === 'consult') {
      wx.showActionSheet({
        itemList: ['图文问诊 ¥19 起', '视频问诊 ¥39 起', '复诊续方'],
        success: (res) => { wx.showToast({ title: '已为您匹配医生', icon: 'success' }); }
      });
    } else if (key === 'pay') {
      wx.showToast({ title: '暂无待缴费用', icon: 'none' });
    } else {
      wx.showToast({ title: '住院服务即将上线', icon: 'none' });
    }
  },
  goDept() { wx.switchTab({ url: '/pages/dept/dept' }); },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  goDoctor(e) { wx.navigateTo({ url: '/pages/doctor/doctor?id=' + e.currentTarget.dataset.id }); },
  callHos() {
    wx.showModal({ title: '拨打医院电话', content: data.hospital.phone, confirmText: '呼叫' });
  },
  openNews(e) {
    wx.showModal({ title: '健康资讯', content: e.currentTarget.dataset.title, showCancel: false });
  }
});
