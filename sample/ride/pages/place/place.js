const POIS = [
  { name: '北京首都国际机场T3航站楼', addr: '顺义区首都机场路', km: 22.6 },
  { name: '北京大兴国际机场', addr: '大兴区大兴机场高速', km: 52.3 },
  { name: '北京南站', addr: '丰台区永外大街车站路12号', km: 19.8 },
  { name: '北京西站', addr: '丰台区莲花池东路118号', km: 20.4 },
  { name: '三里屯太古里', addr: '朝阳区三里屯路19号', km: 8.6 },
  { name: '国贸商城', addr: '朝阳区建国门外大街1号', km: 11.2 },
  { name: '颐和园', addr: '海淀区新建宫门路19号', km: 21.5 },
  { name: '五道口购物中心', addr: '海淀区成府路28号', km: 13.9 },
  { name: '望京凯德MALL', addr: '朝阳区广顺北大街33号', km: 1.6 },
  { name: '798艺术区', addr: '朝阳区酒仙桥路4号', km: 3.4 },
  { name: '中关村软件园二期', addr: '海淀区西北旺东路10号', km: 19.1 },
  { name: '朝阳大悦城', addr: '朝阳区朝阳北路101号', km: 9.7 },
  { name: '北京协和医院（东单院区）', addr: '东城区帅府园1号', km: 13.5 }
];
const COMMON = [
  { tag: '回家', glyph: '家', bg: 'linear-gradient(135deg, #FFB067, #FF7A1A)', name: '望京西园三区', addr: '朝阳区望京西园三区312号楼', km: 1.9 },
  { tag: '去公司', glyph: '司', bg: 'linear-gradient(135deg, #6BA8FF, #3D7BFF)', name: '中关村软件园二期', addr: '海淀区西北旺东路10号', km: 19.1 }
];

Page({
  data: { type: 'to', keyword: '', results: [], history: [], common: COMMON, nearby: POIS.filter(p => p.km < 10).slice(0, 4) },
  onLoad(q) {
    const type = q.type === 'from' ? 'from' : 'to';
    this.setData({ type, history: wx.getStorageSync('ride_history') || [] });
    wx.setNavigationBarTitle({ title: type === 'from' ? '选择上车点' : '选择目的地' });
  },
  onInput(e) {
    const kw = (e.detail.value || '').trim();
    const results = kw ? POIS.filter(p => p.name.indexOf(kw) >= 0 || p.addr.indexOf(kw) >= 0) : [];
    this.setData({ keyword: kw, results });
  },
  onClear() { this.setData({ keyword: '', results: [] }); },
  onCommon(e) { this.choose(COMMON[e.currentTarget.dataset.idx]); },
  onChoose(e) {
    const { src, idx } = e.currentTarget.dataset;
    this.choose(this.data[src][idx]);
  },
  choose(p) {
    const place = { name: p.name, addr: p.addr, km: p.km };
    if (this.data.type === 'from') place.km = 0;
    wx.setStorageSync(this.data.type === 'from' ? 'ride_from' : 'ride_to', place);
    const history = (wx.getStorageSync('ride_history') || []).filter(h => h.name !== p.name);
    history.unshift({ name: p.name, addr: p.addr, km: p.km });
    wx.setStorageSync('ride_history', history.slice(0, 5));
    wx.navigateBack();
  },
  onClearHistory() {
    wx.showModal({
      title: '清空历史记录', content: '确定清空全部搜索历史吗？',
      success: (r) => { if (r.confirm) { wx.removeStorageSync('ride_history'); this.setData({ history: [] }); } }
    });
  }
});
