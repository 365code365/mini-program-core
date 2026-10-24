// 演示数据与本地存储（全部离线，写进 storage 后各页共用）
const PWD = '123456';

const ACCOUNTS = [
  { id: 'a1', name: '一卡通', type: '借记卡 · 金卡', no: '6225881234567788', balance: 86432.56, bg: 'linear-gradient(135deg, #F0534A, #B8122B)' },
  { id: 'a2', name: '工资卡', type: '借记卡 · 普卡', no: '6225889876543021', balance: 23560.08, bg: 'linear-gradient(135deg, #3E7BFA, #1D4ED8)' },
  { id: 'a3', name: '朝朝宝', type: '活钱管理', no: '6225880011223366', balance: 152000.0, bg: 'linear-gradient(135deg, #F7A23B, #D9731C)' }
];

const BANKS = ['招财银行', '工商银行', '建设银行', '农业银行', '中国银行', '交通银行', '浦发银行', '兴业银行', '中信银行', '民生银行', '平安银行', '邮储银行'];

const PAYEES = [
  { name: '陈雨桐', card: '6222021588903366', bank: '工商银行', letter: 'C' },
  { name: '曹文杰', card: '6217003810002288', bank: '建设银行', letter: 'C' },
  { name: '杜思远', card: '6228480402564890', bank: '农业银行', letter: 'D' },
  { name: '方静怡', card: '6225887612340915', bank: '招财银行', letter: 'F' },
  { name: '郭明轩', card: '6013820800012345', bank: '中国银行', letter: 'G' },
  { name: '黄嘉欣', card: '6222600110023456', bank: '交通银行', letter: 'H' },
  { name: '何子涵', card: '6217920100088812', bank: '浦发银行', letter: 'H' },
  { name: '李晓梅', card: '6229090012347766', bank: '兴业银行', letter: 'L' },
  { name: '林浩然', card: '6217680700001234', bank: '中信银行', letter: 'L' },
  { name: '刘佳琪', card: '6226220123456789', bank: '民生银行', letter: 'L' },
  { name: '马一鸣', card: '6230580000123987', bank: '平安银行', letter: 'M' },
  { name: '宋雅琳', card: '6217995800001122', bank: '邮储银行', letter: 'S' },
  { name: '孙浩宇', card: '6225880099887766', bank: '招财银行', letter: 'S' },
  { name: '王梓萱', card: '6222081234560001', bank: '工商银行', letter: 'W' },
  { name: '吴俊杰', card: '6217001234005678', bank: '建设银行', letter: 'W' },
  { name: '徐晨曦', card: '6228481234009999', bank: '农业银行', letter: 'X' },
  { name: '杨思琪', card: '6013821900056789', bank: '中国银行', letter: 'Y' },
  { name: '张伟', card: '6222600987654321', bank: '交通银行', letter: 'Z' },
  { name: '赵子墨', card: '6225887700112233', bank: '招财银行', letter: 'Z' },
  { name: '周雨萌', card: '6229091100224455', bank: '兴业银行', letter: 'Z' }
];

const PRODUCTS = [
  { id: 'p1', name: '朝朝宝 · 活钱管理', type: '活钱管理', risk: 'R1', minBuy: 1, maxBuy: 500000, desc: '随存随取，T+0 快速赎回，适合日常零钱打理', periods: [{ days: 1, rate: 2.05 }, { days: 30, rate: 2.12 }], tag: '随时可取' },
  { id: 'p2', name: '招财稳享 90 天', type: '定期理财', risk: 'R2', minBuy: 10000, maxBuy: 1000000, desc: '固定期限，到期自动赎回，历史业绩稳健', periods: [{ days: 90, rate: 3.05 }, { days: 180, rate: 3.25 }], tag: '热销' },
  { id: 'p3', name: '季季盈 · 固收增强', type: '稳健理财', risk: 'R2', minBuy: 1000, maxBuy: 500000, desc: '以债券为主，少量增强资产，波动较小', periods: [{ days: 91, rate: 3.38 }, { days: 182, rate: 3.52 }, { days: 364, rate: 3.70 }], tag: '新客专享' },
  { id: 'p4', name: '半年享 · 稳健精选', type: '定期理财', risk: 'R2', minBuy: 50000, maxBuy: 2000000, desc: '半年封闭，起购门槛较高，收益较高', periods: [{ days: 182, rate: 3.65 }], tag: '' },
  { id: 'p5', name: '年年鑫 365 天', type: '定期理财', risk: 'R3', minBuy: 10000, maxBuy: 1000000, desc: '一年期，配置少量权益资产，力争更高回报', periods: [{ days: 365, rate: 4.10 }, { days: 730, rate: 4.35 }], tag: '高收益' },
  { id: 'p6', name: '招财沪深 300 指数', type: '基金', risk: 'R4', minBuy: 10, maxBuy: 300000, desc: '跟踪沪深 300 指数，长期定投更合适', periods: [{ days: 365, rate: 6.80 }, { days: 1095, rate: 7.20 }], tag: '定投' },
  { id: 'p7', name: '短债优选 A', type: '基金', risk: 'R2', minBuy: 100, maxBuy: 500000, desc: '投资短久期债券，持有 7 天以上免赎回费', periods: [{ days: 7, rate: 2.60 }, { days: 30, rate: 2.85 }, { days: 90, rate: 3.00 }], tag: '' },
  { id: 'p8', name: '月月盈 30 天', type: '稳健理财', risk: 'R1', minBuy: 1000, maxBuy: 300000, desc: '每 30 天一个开放期，到期可选择续投', periods: [{ days: 30, rate: 2.45 }], tag: '' },
  { id: 'p9', name: '科技先锋混合', type: '基金', risk: 'R4', minBuy: 10, maxBuy: 200000, desc: '精选科技成长股，波动较大，适合进取型', periods: [{ days: 365, rate: 9.60 }, { days: 1095, rate: 10.40 }], tag: '近一年 +18%' },
  { id: 'p10', name: '养老目标 2045', type: '稳健理财', risk: 'R3', minBuy: 1, maxBuy: 1000000, desc: '目标日期策略，随时间逐步降低权益比例', periods: [{ days: 365, rate: 4.60 }, { days: 1825, rate: 5.20 }], tag: '养老' }
];

const RISK_NAME = { R1: '低风险', R2: '中低风险', R3: '中风险', R4: '中高风险' };

// 交易流水：从 2026-10-24 往前按固定步长铺开，保证每次生成一样
function seedBills() {
  const outs = [
    ['美团外卖', '餐饮', 38.5], ['滴滴出行', '交通', 26.0], ['盒马鲜生', '购物', 156.8], ['国家电网', '缴费', 210.35],
    ['星巴克', '餐饮', 42.0], ['京东商城', '购物', 899.0], ['中国移动', '缴费', 100.0], ['信用卡还款', '还款', 3560.2],
    ['房租', '转账', 4500.0], ['瑞幸咖啡', '餐饮', 19.9], ['地铁出行', '交通', 6.0], ['山姆会员店', '购物', 645.3]
  ];
  const ins = [['工资收入', '工资', 18650.0], ['朝朝宝收益', '理财', 8.36], ['陈雨桐 转入', '转账', 500.0], ['退款 · 京东', '退款', 129.0]];
  const list = [];
  // 以「今天 8:15」为最新一笔往前铺，近期交易与本月支出才有数
  const d0 = new Date();
  let t = new Date(d0.getFullYear(), d0.getMonth(), d0.getDate(), 8, 15).getTime();
  for (let i = 0; i < 46; i++) {
    const isIn = i % 5 === 2;
    const src = isIn ? ins[i % ins.length] : outs[i % outs.length];
    const amount = src[2] + (i % 3) * (isIn ? 0 : 1.1);
    list.push({
      id: 'b' + (1000 + i), acct: ACCOUNTS[i % 7 === 3 ? 1 : 0].id, type: isIn ? 'in' : 'out',
      title: src[0], cat: src[1], amount: Math.round(amount * 100) / 100, time: t,
      counter: isIn ? '对方账户 6222 **** 3366' : src[0] + ' 商户', remark: ''
    });
    t -= (37 + (i * 13) % 41) * 3600 * 1000;
  }
  return list;
}

function get(key, def) {
  const v = wx.getStorageSync('bank_' + key);
  return v === '' || v === undefined || v === null ? def : v;
}
function set(key, v) { wx.setStorageSync('bank_' + key, v); }

function accounts() {
  let a = get('accounts', null);
  if (!a) { a = ACCOUNTS.map(x => Object.assign({}, x)); set('accounts', a); }
  return a;
}
function bills() {
  let b = get('bills', null);
  if (!b) { b = seedBills(); set('bills', b); }
  return b;
}
function addBill(bill) {
  const b = bills();
  b.unshift(bill);
  set('bills', b);
}
function holdings() { return get('holdings', []); }
function pwdState() { return get('pwd', { errors: 0, locked: false }); }
function payees() {
  // 最近转过的收款人放前面（转账成功时写入）
  return get('recentPayees', []);
}
function resetAll() {
  ['accounts', 'bills', 'holdings', 'pwd', 'recentPayees', 'hide', 'curAcct', 'nopwd'].forEach(k => wx.removeStorageSync('bank_' + k));
}

module.exports = { PWD, BANKS, PAYEES, PRODUCTS, RISK_NAME, get, set, accounts, bills, addBill, holdings, pwdState, payees, resetAll };
