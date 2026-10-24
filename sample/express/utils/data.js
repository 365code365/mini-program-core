// 快递公司：按单号前缀识别
const CARRIERS = {
  sf: { name: '顺丰速运', short: '顺', color: '#1f1f1f', phone: '95338' },
  jd: { name: '京东快递', short: '京', color: '#e1251b', phone: '950616' },
  zto: { name: '中通快递', short: '中', color: '#1d4fa3', phone: '95311' },
  yto: { name: '圆通速递', short: '圆', color: '#6a2c91', phone: '95554' },
  yd: { name: '韵达快递', short: '韵', color: '#f5a300', phone: '95546' },
  sto: { name: '申通快递', short: '申', color: '#ef7d00', phone: '95543' },
  ems: { name: '中国邮政EMS', short: '邮', color: '#0a7a3e', phone: '11183' }
}

function detectCarrier(no) {
  const s = (no || '').trim().toUpperCase()
  if (s.length < 8) return ''
  if (s.indexOf('SF') === 0) return 'sf'
  if (s.indexOf('JD') === 0) return 'jd'
  if (s.indexOf('YT') === 0) return 'yto'
  if (s.indexOf('E') === 0) return 'ems'
  if (/^7[35]/.test(s)) return 'zto'
  if (/^77/.test(s)) return 'sto'
  if (/^(43|46|31)/.test(s)) return 'yd'
  return 'zto'
}

const STATUS = {
  transit: { text: '运输中', cls: 'tag-transit' },
  deliver: { text: '派送中', cls: 'tag-deliver' },
  signed: { text: '已签收', cls: 'tag-signed' },
  pending: { text: '待揽收', cls: 'tag-pending' },
  problem: { text: '疑难件', cls: 'tag-problem' }
}

const SEED = [
  {
    no: 'SF1428930675512', carrier: 'sf', status: 'deliver', remark: 'iPhone 16 Pro 保护壳',
    from: '深圳', to: '杭州', eta: '今天 18:00 前送达',
    courier: { name: '王师傅', phone: '138 5712 6630', rate: '4.9', count: '2.3万' },
    traces: [
      { time: '10-24 09:12', text: '【杭州西湖区文三路营业点】快递员 王师傅 正在派件，电话 13857126630，请保持电话畅通' },
      { time: '10-24 06:40', text: '快件已到达【杭州西湖区文三路营业点】' },
      { time: '10-23 23:58', text: '快件已从【杭州萧山中转场】发出，下一站【文三路营业点】' },
      { time: '10-23 15:20', text: '快件已从【深圳华南航空枢纽】发出，下一站【杭州萧山中转场】' },
      { time: '10-23 11:05', text: '顺丰速运 已收取快件，揽收网点【深圳南山科技园营业点】' }
    ]
  },
  {
    no: 'JD0098812345678', carrier: 'jd', status: 'transit', remark: '戴森吹风机 HD15',
    from: '北京', to: '杭州', eta: '预计明天送达',
    courier: { name: '李晓峰', phone: '186 0011 2468', rate: '5.0', count: '1.1万' },
    traces: [
      { time: '10-24 08:30', text: '您的快件已到达【上海嘉定分拣中心】，准备发往【杭州余杭分拣中心】' },
      { time: '10-23 21:12', text: '您的快件已从【北京亚洲一号】发出' },
      { time: '10-23 17:46', text: '您的订单已打包完成，京东快递 已揽收' }
    ]
  },
  {
    no: '75382910476635', carrier: 'zto', status: 'signed', remark: '三只松鼠 坚果礼盒',
    from: '芜湖', to: '杭州', eta: '10-22 14:36 已签收',
    courier: { name: '陈建国', phone: '159 8820 3317', rate: '4.8', count: '8600' },
    traces: [
      { time: '10-22 14:36', text: '已签收，签收人凭取件码在【菜鸟驿站·文二西路店】取件，感谢使用中通快递' },
      { time: '10-22 09:15', text: '快件已送达【菜鸟驿站·文二西路店】，取件码 3-2-1086' },
      { time: '10-21 20:02', text: '快件已到达【杭州中转部】' },
      { time: '10-21 08:44', text: '快件已从【芜湖中转部】发出' },
      { time: '10-20 18:30', text: '【芜湖弋江区】已揽收' }
    ]
  },
  {
    no: 'YT8812765340021', carrier: 'yto', status: 'pending', remark: '退货：优衣库 摇粒绒外套',
    from: '杭州', to: '上海', eta: '等待快递员上门揽收',
    courier: { name: '赵磊', phone: '177 0571 8852', rate: '4.7', count: '5200' },
    traces: [
      { time: '10-24 10:02', text: '订单已提交，快递员 赵磊 将于 14:00-16:00 上门取件' }
    ]
  }
]

function decorate(item) {
  const c = CARRIERS[item.carrier] || CARRIERS.zto
  const st = STATUS[item.status] || STATUS.transit
  return Object.assign({}, item, {
    carrierName: c.name, short: c.short, color: c.color,
    statusText: st.text, statusCls: st.cls,
    last: item.traces && item.traces.length ? item.traces[0] : { time: '', text: '' }
  })
}

function loadList() {
  let list = wx.getStorageSync('express_list')
  if (!list || !list.length) {
    list = SEED
    wx.setStorageSync('express_list', list)
  }
  return list
}

function saveList(list) {
  wx.setStorageSync('express_list', list)
}

// 新查询的单号：生成一条可信的物流轨迹
function mockTrack(no, carrier) {
  const c = CARRIERS[carrier]
  return {
    no, carrier, status: 'transit', remark: '', from: '广州', to: '杭州', eta: '预计 2 天后送达',
    courier: { name: '周师傅', phone: '135 0571 2290', rate: '4.9', count: '1.6万' },
    traces: [
      { time: '10-24 07:18', text: '快件已从【广州白云转运中心】发出，下一站【杭州转运中心】' },
      { time: '10-23 22:40', text: '快件已到达【广州白云转运中心】' },
      { time: '10-23 18:06', text: c.name + ' 已揽收，揽收网点【广州天河区体育西营业部】' }
    ]
  }
}

const ADDRESSES = [
  { id: 1, name: '林小鹿', phone: '138****6630', region: '浙江省 杭州市 西湖区', detail: '文三路 478 号华星时代广场 A 座 1203', tag: '公司', isDefault: true },
  { id: 2, name: '林小鹿', phone: '138****6630', region: '浙江省 杭州市 余杭区', detail: '五常街道 西溪北苑 12 幢 2 单元 501', tag: '家', isDefault: false },
  { id: 3, name: '林建华', phone: '139****2087', region: '福建省 厦门市 思明区', detail: '莲前西路 168 号 瑞景新村 8 号楼 602', tag: '父母', isDefault: false },
  { id: 4, name: '苏晴', phone: '186****9015', region: '上海市 上海市 徐汇区', detail: '漕溪北路 88 号 圣爱广场 2201 室', tag: '朋友', isDefault: false }
]

function loadAddresses() {
  let list = wx.getStorageSync('express_addr')
  if (!list || !list.length) {
    list = ADDRESSES
    wx.setStorageSync('express_addr', list)
  }
  return list
}

module.exports = { CARRIERS, STATUS, detectCarrier, decorate, loadList, saveList, mockTrack, loadAddresses }
