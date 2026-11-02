// 国内通航城市：名称、全拼、简拼、城市三字码、机场（IATA 三字码真实）、经纬度（算航程与票价）
const RAW = [
  ['北京', 'beijing', 'bj', 'BJS', [['首都', 'PEK'], ['大兴', 'PKX']], 39.9, 116.4, 1],
  ['上海', 'shanghai', 'sh', 'SHA', [['虹桥', 'SHA'], ['浦东', 'PVG']], 31.2, 121.5, 1],
  ['广州', 'guangzhou', 'gz', 'CAN', [['白云', 'CAN']], 23.1, 113.3, 1],
  ['深圳', 'shenzhen', 'sz', 'SZX', [['宝安', 'SZX']], 22.6, 114.1, 1],
  ['成都', 'chengdu', 'cd', 'CTU', [['天府', 'TFU'], ['双流', 'CTU']], 30.6, 104.1, 1],
  ['杭州', 'hangzhou', 'hz', 'HGH', [['萧山', 'HGH']], 30.3, 120.2, 1],
  ['重庆', 'chongqing', 'cq', 'CKG', [['江北', 'CKG']], 29.6, 106.5, 1],
  ['西安', 'xian', 'xa', 'SIA', [['咸阳', 'XIY']], 34.3, 108.9, 1],
  ['昆明', 'kunming', 'km', 'KMG', [['长水', 'KMG']], 25.0, 102.7, 1],
  ['三亚', 'sanya', 'sy', 'SYX', [['凤凰', 'SYX']], 18.3, 109.5, 1],
  ['厦门', 'xiamen', 'xm', 'XMN', [['高崎', 'XMN']], 24.5, 118.1, 1],
  ['南京', 'nanjing', 'nj', 'NKG', [['禄口', 'NKG']], 32.1, 118.8, 1],
  ['武汉', 'wuhan', 'wh', 'WUH', [['天河', 'WUH']], 30.6, 114.3, 0],
  ['长沙', 'changsha', 'cs', 'CSX', [['黄花', 'CSX']], 28.2, 113.0, 0],
  ['青岛', 'qingdao', 'qd', 'TAO', [['胶东', 'TAO']], 36.1, 120.4, 0],
  ['海口', 'haikou', 'hk', 'HAK', [['美兰', 'HAK']], 20.0, 110.3, 0],
  ['大连', 'dalian', 'dl', 'DLC', [['周水子', 'DLC']], 38.9, 121.6, 0],
  ['沈阳', 'shenyang', 'sy', 'SHE', [['桃仙', 'SHE']], 41.8, 123.4, 0],
  ['哈尔滨', 'haerbin', 'heb', 'HRB', [['太平', 'HRB']], 45.8, 126.5, 0],
  ['长春', 'changchun', 'cc', 'CGQ', [['龙嘉', 'CGQ']], 43.9, 125.3, 0],
  ['乌鲁木齐', 'wulumuqi', 'wlmq', 'URC', [['天山', 'URC']], 43.8, 87.6, 0],
  ['拉萨', 'lasa', 'ls', 'LXA', [['贡嘎', 'LXA']], 29.6, 91.1, 0],
  ['贵阳', 'guiyang', 'gy', 'KWE', [['龙洞堡', 'KWE']], 26.6, 106.7, 0],
  ['南宁', 'nanning', 'nn', 'NNG', [['吴圩', 'NNG']], 22.8, 108.4, 0],
  ['郑州', 'zhengzhou', 'zz', 'CGO', [['新郑', 'CGO']], 34.7, 113.6, 0],
  ['济南', 'jinan', 'jn', 'TNA', [['遥墙', 'TNA']], 36.7, 117.0, 0],
  ['天津', 'tianjin', 'tj', 'TSN', [['滨海', 'TSN']], 39.1, 117.2, 0],
  ['福州', 'fuzhou', 'fz', 'FOC', [['长乐', 'FOC']], 26.1, 119.3, 0],
  ['合肥', 'hefei', 'hf', 'HFE', [['新桥', 'HFE']], 31.8, 117.2, 0],
  ['南昌', 'nanchang', 'nc', 'KHN', [['昌北', 'KHN']], 28.7, 115.9, 0],
  ['太原', 'taiyuan', 'ty', 'TYN', [['武宿', 'TYN']], 37.9, 112.6, 0],
  ['兰州', 'lanzhou', 'lz', 'LHW', [['中川', 'LHW']], 36.1, 103.8, 0],
  ['呼和浩特', 'huhehaote', 'hhht', 'HET', [['白塔', 'HET']], 40.8, 111.7, 0],
  ['银川', 'yinchuan', 'yc', 'INC', [['河东', 'INC']], 38.5, 106.3, 0],
  ['西宁', 'xining', 'xn', 'XNN', [['曹家堡', 'XNN']], 36.6, 101.8, 0],
  ['桂林', 'guilin', 'gl', 'KWL', [['两江', 'KWL']], 25.3, 110.3, 0],
  ['丽江', 'lijiang', 'lj', 'LJG', [['三义', 'LJG']], 26.9, 100.2, 0],
  ['西双版纳', 'xishuangbanna', 'xsbn', 'JHG', [['嘎洒', 'JHG']], 22.0, 100.8, 0],
  ['珠海', 'zhuhai', 'zh', 'ZUH', [['金湾', 'ZUH']], 22.3, 113.6, 0],
  ['温州', 'wenzhou', 'wz', 'WNZ', [['龙湾', 'WNZ']], 28.0, 120.7, 0],
  ['宁波', 'ningbo', 'nb', 'NGB', [['栎社', 'NGB']], 29.9, 121.6, 0],
  ['石家庄', 'shijiazhuang', 'sjz', 'SJW', [['正定', 'SJW']], 38.0, 114.5, 0],
  ['烟台', 'yantai', 'yt', 'YNT', [['蓬莱', 'YNT']], 37.5, 121.4, 0],
  ['张家界', 'zhangjiajie', 'zjj', 'DYG', [['荷花', 'DYG']], 29.1, 110.5, 0],
  ['敦煌', 'dunhuang', 'dh', 'DNH', [['莫高', 'DNH']], 40.1, 94.7, 0],
  ['延吉', 'yanji', 'yj', 'YNJ', [['朝阳川', 'YNJ']], 42.9, 129.5, 0]
];

const CITIES = RAW.map(r => ({
  name: r[0], py: r[1], jp: r[2], code: r[3],
  airports: r[4].map(a => ({ name: r[0] + a[0], short: a[0], code: a[1] })),
  lat: r[5], lng: r[6], hot: !!r[7], letter: r[1][0].toUpperCase()
}));

const byCode = {};
CITIES.forEach(c => { byCode[c.code] = c; });

function getCity(code) { return byCode[code] || CITIES[0]; }

// 字母分组（只出有城市的字母）
function groups() {
  const map = {};
  CITIES.forEach(c => { (map[c.letter] = map[c.letter] || []).push(c); });
  return Object.keys(map).sort().map(k => ({ letter: k, list: map[k].sort((a, b) => a.py < b.py ? -1 : 1) }));
}

// 名称 / 全拼 / 简拼 / 城市或机场三字码 搜索
function search(kw) {
  const k = String(kw || '').trim().toLowerCase();
  if (!k) return [];
  return CITIES.filter(c => c.name.indexOf(k) >= 0 || c.py.indexOf(k) === 0 || c.jp.indexOf(k) === 0 ||
    c.code.toLowerCase() === k || c.airports.some(a => a.code.toLowerCase() === k || a.name.indexOf(k) >= 0));
}

// 两城之间的大圆距离（km）
function distance(a, b) {
  const R = 6371, rad = Math.PI / 180;
  const dLat = (b.lat - a.lat) * rad, dLng = (b.lng - a.lng) * rad;
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(a.lat * rad) * Math.cos(b.lat * rad) * Math.sin(dLng / 2) ** 2;
  return Math.round(2 * R * Math.asin(Math.sqrt(h)));
}

module.exports = { CITIES, getCity, groups, search, distance };
