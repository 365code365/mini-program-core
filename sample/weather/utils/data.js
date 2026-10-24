// 离线天气数据：城市实况写死在表里，逐小时与 7 日预报按城市参数确定性地推出来
const COND = {
  sunny: { name: '晴', bg: 'linear-gradient(180deg, #2f7ff0 0%, #5aa9ff 55%, #8fd0ff 100%)' },
  cloudy: { name: '多云', bg: 'linear-gradient(180deg, #4c7fc9 0%, #77a5df 55%, #a9c9ef 100%)' },
  overcast: { name: '阴', bg: 'linear-gradient(180deg, #5d6d82 0%, #7f8ea2 55%, #a7b3c2 100%)' },
  rain: { name: '小雨', bg: 'linear-gradient(180deg, #34506f 0%, #56739a 55%, #7f97b8 100%)' },
  storm: { name: '雷阵雨', bg: 'linear-gradient(180deg, #2b3a55 0%, #4a5b7c 55%, #6f7f9c 100%)' },
  snow: { name: '小雪', bg: 'linear-gradient(180deg, #6f93c2 0%, #9ab8dc 55%, #cddcef 100%)' }
}

const CITIES = [
  { id: 'beijing', name: '北京', prov: '北京', cond: 'sunny', hi: 21, lo: 8, hum: 32, wind: '西北风', lv: 3, aqi: 68, seed: 3 },
  { id: 'shanghai', name: '上海', prov: '上海', cond: 'cloudy', hi: 24, lo: 17, hum: 68, wind: '东南风', lv: 2, aqi: 42, seed: 5 },
  { id: 'guangzhou', name: '广州', prov: '广东', cond: 'storm', hi: 30, lo: 23, hum: 84, wind: '南风', lv: 2, aqi: 35, seed: 7 },
  { id: 'shenzhen', name: '深圳', prov: '广东', cond: 'cloudy', hi: 29, lo: 23, hum: 78, wind: '东风', lv: 3, aqi: 28, seed: 11 },
  { id: 'hangzhou', name: '杭州', prov: '浙江', cond: 'rain', hi: 22, lo: 16, hum: 88, wind: '东北风', lv: 2, aqi: 46, seed: 13 },
  { id: 'chengdu', name: '成都', prov: '四川', cond: 'overcast', hi: 20, lo: 15, hum: 80, wind: '北风', lv: 1, aqi: 96, seed: 17 },
  { id: 'xian', name: '西安', prov: '陕西', cond: 'sunny', hi: 19, lo: 7, hum: 40, wind: '东北风', lv: 2, aqi: 118, seed: 19 },
  { id: 'harbin', name: '哈尔滨', prov: '黑龙江', cond: 'snow', hi: 2, lo: -7, hum: 70, wind: '西北风', lv: 4, aqi: 75, seed: 23 },
  { id: 'sanya', name: '三亚', prov: '海南', cond: 'sunny', hi: 31, lo: 25, hum: 72, wind: '东南风', lv: 3, aqi: 18, seed: 29 },
  { id: 'kunming', name: '昆明', prov: '云南', cond: 'cloudy', hi: 22, lo: 11, hum: 58, wind: '西南风', lv: 3, aqi: 32, seed: 31 },
  { id: 'wuhan', name: '武汉', prov: '湖北', cond: 'overcast', hi: 21, lo: 14, hum: 74, wind: '北风', lv: 2, aqi: 82, seed: 37 },
  { id: 'nanjing', name: '南京', prov: '江苏', cond: 'cloudy', hi: 22, lo: 13, hum: 62, wind: '东风', lv: 2, aqi: 58, seed: 41 },
  { id: 'chongqing', name: '重庆', prov: '重庆', cond: 'rain', hi: 21, lo: 16, hum: 90, wind: '北风', lv: 1, aqi: 64, seed: 43 },
  { id: 'lasa', name: '拉萨', prov: '西藏', cond: 'sunny', hi: 16, lo: 1, hum: 22, wind: '西风', lv: 3, aqi: 22, seed: 47 },
  { id: 'qingdao', name: '青岛', prov: '山东', cond: 'cloudy', hi: 18, lo: 11, hum: 66, wind: '东南风', lv: 4, aqi: 48, seed: 53 },
  { id: 'xiamen', name: '厦门', prov: '福建', cond: 'sunny', hi: 27, lo: 20, hum: 70, wind: '东北风', lv: 3, aqi: 26, seed: 59 },
  { id: 'tianjin', name: '天津', prov: '天津', cond: 'overcast', hi: 19, lo: 9, hum: 46, wind: '西南风', lv: 2, aqi: 104, seed: 61 },
  { id: 'changsha', name: '长沙', prov: '湖南', cond: 'rain', hi: 23, lo: 17, hum: 86, wind: '北风', lv: 2, aqi: 52, seed: 67 },
  { id: 'urumqi', name: '乌鲁木齐', prov: '新疆', cond: 'sunny', hi: 9, lo: -2, hum: 30, wind: '西北风', lv: 3, aqi: 88, seed: 71 },
  { id: 'dalian', name: '大连', prov: '辽宁', cond: 'cloudy', hi: 15, lo: 7, hum: 60, wind: '北风', lv: 5, aqi: 44, seed: 73 }
]

const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
const NEXT = { sunny: ['sunny', 'cloudy', 'sunny', 'cloudy', 'overcast', 'sunny', 'sunny'], cloudy: ['cloudy', 'overcast', 'rain', 'cloudy', 'sunny', 'sunny', 'cloudy'],
  overcast: ['overcast', 'rain', 'rain', 'cloudy', 'cloudy', 'sunny', 'overcast'], rain: ['rain', 'rain', 'overcast', 'cloudy', 'sunny', 'cloudy', 'rain'],
  storm: ['storm', 'rain', 'cloudy', 'storm', 'cloudy', 'sunny', 'cloudy'], snow: ['snow', 'overcast', 'sunny', 'sunny', 'snow', 'cloudy', 'overcast'] }

function rng(seed) {
  let s = seed * 9301 + 49297
  return () => { s = (s * 9301 + 49297) % 233280; return s / 233280 }
}

function byId(id) { return CITIES.filter(c => c.id === id)[0] || CITIES[0] }

function aqiLevel(v) {
  if (v <= 50) return { name: '优', color: '#3ccf7c', tip: '空气很好，适合户外活动' }
  if (v <= 100) return { name: '良', color: '#f2c230', tip: '空气可接受，极少数敏感人群应减少户外活动' }
  if (v <= 150) return { name: '轻度污染', color: '#f58d1f', tip: '儿童、老人及心脏病、呼吸系统疾病患者应减少长时间户外锻炼' }
  if (v <= 200) return { name: '中度污染', color: '#f04848', tip: '一般人群减少户外运动，外出佩戴口罩' }
  if (v <= 300) return { name: '重度污染', color: '#9b4dca', tip: '停止户外运动，关好门窗' }
  return { name: '严重污染', color: '#7e2a3d', tip: '避免外出' }
}

// 当前温度：按一天里的时刻在最低/最高之间插值（14 点最高、5 点最低）
function tempAt(c, hour) {
  const x = Math.cos((hour - 14) / 24 * Math.PI * 2)
  return Math.round(c.lo + (c.hi - c.lo) * (x + 1) / 2)
}

function hourly(c, now) {
  const r = rng(c.seed)
  const list = []
  const h0 = now.getHours()
  for (let i = 0; i < 24; i++) {
    const h = (h0 + i) % 24
    let cond = c.cond
    const v = r()
    if (v < 0.18) cond = NEXT[c.cond][1]
    if ((h >= 19 || h < 6) && (cond === 'sunny')) cond = 'night'
    list.push({ i, label: i === 0 ? '现在' : (h < 10 ? '0' + h : h) + ':00', cond, t: tempAt(c, h), pop: (cond === 'rain' || cond === 'storm') ? Math.round(50 + r() * 45) : 0 })
  }
  return list
}

function daily(c, now) {
  const r = rng(c.seed + 100)
  const list = []
  for (let i = 0; i < 7; i++) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + i)
    const drift = i === 0 ? 0 : Math.round((r() - 0.5) * 6)
    const hi = c.hi + drift
    const lo = Math.min(hi - 3, c.lo + Math.round((r() - 0.5) * 4))
    const cond = NEXT[c.cond][i]
    list.push({ i, day: i === 0 ? '今天' : i === 1 ? '明天' : WEEK[d.getDay()], date: (d.getMonth() + 1) + '/' + d.getDate(), cond, condName: COND[cond].name, hi, lo })
  }
  return list
}

function pollutants(c) {
  const r = rng(c.seed + 7)
  const pm25 = Math.round(c.aqi * 0.62 + r() * 6)
  return [
    { k: 'PM2.5', n: '细颗粒物', v: pm25, max: 150 },
    { k: 'PM10', n: '可吸入颗粒物', v: Math.round(pm25 * 1.55 + r() * 10), max: 250 },
    { k: 'O₃', n: '臭氧', v: Math.round(60 + r() * 70), max: 200 },
    { k: 'NO₂', n: '二氧化氮', v: Math.round(15 + c.aqi * 0.25 + r() * 10), max: 120 },
    { k: 'SO₂', n: '二氧化硫', v: Math.round(4 + r() * 8), max: 50 },
    { k: 'CO', n: '一氧化碳', v: Math.round((0.4 + r() * 0.6) * 10) / 10, max: 4 }
  ]
}

function lifeIndex(c) {
  const t = c.hi
  const wet = c.cond === 'rain' || c.cond === 'storm' || c.cond === 'snow'
  const sun = c.cond === 'sunny'
  return [
    { key: 'dress', name: '穿衣', ch: '衣', color: '#ff8a4c', level: t >= 28 ? '炎热' : t >= 22 ? '舒适' : t >= 15 ? '较舒适' : t >= 5 ? '较冷' : '寒冷',
      desc: t >= 28 ? '短袖、短裙、薄型 T 恤，注意防暑' : t >= 22 ? '衬衫、薄外套即可，早晚稍凉' : t >= 15 ? '建议穿长袖衬衫加单裤，或薄款风衣' : t >= 5 ? '建议穿毛衣、夹克或薄羽绒' : '天气寒冷，羽绒服、帽子手套必备' },
    { key: 'uv', name: '紫外线', ch: '紫', color: '#9b6bff', level: sun ? (t > 25 ? '很强' : '中等') : '弱', desc: sun ? '外出请涂抹 SPF30 以上防晒霜，戴遮阳帽' : '紫外线较弱，无需特别防护' },
    { key: 'car', name: '洗车', ch: '车', color: '#4c9bff', level: wet ? '不宜' : '适宜', desc: wet ? '未来有降水，洗车后容易被弄脏' : '未来两天无雨，适合洗车' },
    { key: 'sport', name: '运动', ch: '动', color: '#2fbf8f', level: wet ? '较不宜' : c.aqi > 100 ? '较不宜' : '适宜', desc: wet ? '有降水，建议室内运动' : c.aqi > 100 ? '空气质量一般，减少户外剧烈运动' : '天气不错，适合跑步、骑行等户外运动' },
    { key: 'cold', name: '感冒', ch: '感', color: '#f25b5b', level: c.hi - c.lo >= 10 ? '较易发' : '少发', desc: c.hi - c.lo >= 10 ? '昼夜温差大，注意及时增减衣物' : '气温平稳，感冒概率较低' },
    { key: 'dry', name: '晾晒', ch: '晒', color: '#f5a623', level: wet ? '不宜' : c.hum > 70 ? '一般' : '适宜', desc: wet ? '有降水，请在室内晾晒' : c.hum > 70 ? '湿度偏大，衣物干得慢' : '天气干燥晴好，适合晾晒被褥' },
    { key: 'fish', name: '钓鱼', ch: '钓', color: '#36c2d9', level: c.lv >= 4 || wet ? '不宜' : '适宜', desc: c.lv >= 4 || wet ? '风力较大或有降水，不适合垂钓' : '风力较小，适合钓鱼' },
    { key: 'travel', name: '旅游', ch: '游', color: '#e8657b', level: wet ? '一般' : '适宜', desc: wet ? '有降水，出行请带伞' : '天气晴好，适合出游' }
  ]
}

// 温度单位：C / F
function unit() { return wx.getStorageSync('weather_unit') === 'F' ? 'F' : 'C' }
function fmtT(c, u) {
  u = u || unit()
  return u === 'F' ? Math.round(c * 9 / 5 + 32) : c
}

function myCities() {
  const ids = wx.getStorageSync('weather_cities')
  return Array.isArray(ids) && ids.length ? ids : ['beijing', 'shanghai', 'guangzhou', 'hangzhou']
}
function setMyCities(ids) { wx.setStorageSync('weather_cities', ids) }
function current() {
  const id = wx.getStorageSync('weather_current')
  const mine = myCities()
  return mine.indexOf(id) >= 0 ? id : mine[0]
}
function setCurrent(id) { wx.setStorageSync('weather_current', id) }

module.exports = { COND, CITIES, byId, aqiLevel, tempAt, hourly, daily, pollutants, lifeIndex, unit, fmtT, myCities, setMyCities, current, setCurrent }
