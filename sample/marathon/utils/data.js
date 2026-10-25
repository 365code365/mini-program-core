// 赛事与组别（演示数据，不联网）
const EVENT = {
  name: '2027 滨江城市马拉松',
  raceDate: '2027-04-12',
  startTime: '07:30',
  place: '滨江奥体中心 · 起点北广场',
  // 报名截止：本地时间
  deadline: '2027-03-31T23:59:59'
}

// code：报名号前缀；minAge / maxAge 按比赛日当天的周岁算
const GROUPS = [
  { id: 'full', code: 'F', name: '全程马拉松', short: '全马', dist: '42.195 公里', fee: 200, quota: 15000, taken: 12680, minAge: 18, maxAge: 65, cutoff: '6 小时' },
  { id: 'half', code: 'H', name: '半程马拉松', short: '半马', dist: '21.0975 公里', fee: 150, quota: 12000, taken: 11880, minAge: 16, maxAge: 70, cutoff: '3 小时' },
  { id: 'ten', code: 'T', name: '10 公里跑', short: '10 公里', dist: '10 公里', fee: 100, quota: 8000, taken: 3200, minAge: 12, maxAge: 75, cutoff: '1 小时 40 分' },
  { id: 'mini', code: 'M', name: '迷你欢乐跑', short: '迷你跑', dist: '5 公里', fee: 60, quota: 5000, taken: 5000, minAge: 6, maxAge: 80, cutoff: '1 小时' }
]

const ID_TYPES = ['居民身份证', '护照', '港澳居民来往内地通行证', '台湾居民来往大陆通行证']
const SIZES = ['XS', 'S', 'M', 'L', 'XL', 'XXL']
const STEPS = ['选择组别', '个人信息', '紧急联系人', '免责声明']

const DISCLAIMER = [
  '本人自愿参赛，确认身体健康，近一年内无心脏病、高血压等不宜参加长跑的疾病；',
  '比赛中如身体不适将立即停止比赛并寻求医疗救助，因自身原因造成的损伤自行承担；',
  '同意组委会使用本人肖像、姓名及成绩用于赛事宣传；',
  '报名信息真实有效，参赛号码布不转让、不代跑，违者取消成绩并禁赛两年。'
]

module.exports = { EVENT, GROUPS, ID_TYPES, SIZES, STEPS, DISCLAIMER }
