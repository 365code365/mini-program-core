// 缴费类型、城市与缴费单位、话费面额：纯离线数据
const types = [
  { k: 'water', name: '水费', ch: '水', color: '#3c8cff', bg: 'linear-gradient(135deg, #6aaeff, #3c7cff)', light: '#eaf3ff' },
  { k: 'power', name: '电费', ch: '电', color: '#f59a00', bg: 'linear-gradient(135deg, #ffcf5c, #ff9d1a)', light: '#fff5e0' },
  { k: 'gas', name: '燃气费', ch: '气', color: '#f2603d', bg: 'linear-gradient(135deg, #ff9a7a, #f2603d)', light: '#ffefea' },
  { k: 'net', name: '宽带', ch: '网', color: '#7b5cf0', bg: 'linear-gradient(135deg, #a993ff, #7b5cf0)', light: '#f2eeff' },
  { k: 'phone', name: '话费', ch: '话', color: '#14a06f', bg: 'linear-gradient(135deg, #4fd6a0, #14a06f)', light: '#e6f7f0' }
];

const cities = ['杭州市', '上海市', '北京市', '南京市', '广州市'];

const units = {
  '杭州市': { water: ['杭州市水务集团有限公司'], power: ['国网浙江省电力有限公司杭州供电公司'], gas: ['杭州市燃气集团有限公司', '杭州萧山燃气有限公司'], net: ['中国电信杭州分公司', '中国联通杭州分公司'] },
  '上海市': { water: ['上海城投水务（集团）有限公司', '上海市自来水奉贤有限公司'], power: ['国网上海市电力公司'], gas: ['上海燃气有限公司'], net: ['中国电信上海分公司', '中国移动上海分公司'] },
  '北京市': { water: ['北京市自来水集团有限责任公司'], power: ['国网北京市电力公司'], gas: ['北京市燃气集团有限责任公司'], net: ['中国联通北京分公司', '歌华有线宽带'] },
  '南京市': { water: ['南京水务集团有限公司'], power: ['国网江苏省电力有限公司南京供电分公司'], gas: ['南京港华燃气有限公司'], net: ['中国电信南京分公司'] },
  '广州市': { water: ['广州市自来水有限公司'], power: ['广东电网有限责任公司广州供电局'], gas: ['广州燃气集团有限公司'], net: ['中国电信广州分公司', '中国移动广州分公司'] }
};

// 话费面额：price 为实付（含优惠）
const faces = [
  { v: 10, price: 9.98 }, { v: 20, price: 19.96 }, { v: 30, price: 29.94 }, { v: 50, price: 49.8 },
  { v: 100, price: 99.5 }, { v: 200, price: 199 }, { v: 300, price: 298.5 }, { v: 500, price: 497.5 }
];

function typeOf(k) { return types.find(t => t.k === k) || types[0]; }

module.exports = { types, cities, units, faces, typeOf };
