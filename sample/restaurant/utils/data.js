// 菜单数据（价格单位：分）。缩略图用 CSS 渐变 + 一个字画，不用图片。
const CATS = [
  { id: 'c0', name: '招牌推荐' },
  { id: 'c1', name: '现炒热菜' },
  { id: 'c2', name: '精品盖饭' },
  { id: 'c3', name: '面食粉类' },
  { id: 'c4', name: '小吃点心' },
  { id: 'c5', name: '汤羹炖品' },
  { id: 'c6', name: '饮品甜点' }
];

const C = {
  red: 'linear-gradient(135deg, #ff8a5c, #e8432d)',
  amber: 'linear-gradient(135deg, #ffc56b, #f08a24)',
  green: 'linear-gradient(135deg, #8bd17c, #3ea35a)',
  brown: 'linear-gradient(135deg, #c99a6b, #8a5a32)',
  pink: 'linear-gradient(135deg, #ffb3c1, #f06b8a)',
  blue: 'linear-gradient(135deg, #8ec5ff, #4a86e8)'
};

const DISHES = [
  { id: 'd01', cat: 'c0', name: '招牌小炒黄牛肉', desc: '现切黄牛肉 · 小米辣爆炒', price: 2600, sales: 1286, bg: C.red, ch: '牛', spec: true, tag: '招牌' },
  { id: 'd02', cat: 'c0', name: '台式卤肉饭', desc: '慢卤五花 · 溏心卤蛋', price: 1600, sales: 2310, bg: C.brown, ch: '卤', spec: true, tag: '热卖' },
  { id: 'd03', cat: 'c0', name: '酸汤肥牛', desc: '贵州红酸汤 · 金针菇', price: 2800, sales: 986, bg: C.amber, ch: '酸', spec: true },
  { id: 'd04', cat: 'c1', name: '宫保鸡丁', desc: '鸡腿肉 · 花生米', price: 1800, sales: 864, bg: C.red, ch: '宫', spec: true },
  { id: 'd05', cat: 'c1', name: '鱼香肉丝', desc: '木耳 · 笋丝 · 微甜微辣', price: 1700, sales: 742, bg: C.amber, ch: '鱼', spec: true },
  { id: 'd06', cat: 'c1', name: '手撕包菜', desc: '猪油渣爆香', price: 1200, sales: 655, bg: C.green, ch: '菜', spec: false },
  { id: 'd07', cat: 'c1', name: '番茄炒蛋', desc: '现剥番茄 · 土鸡蛋', price: 1100, sales: 1520, bg: C.amber, ch: '蛋', spec: false },
  { id: 'd08', cat: 'c1', name: '干锅花菜', desc: '五花肉片 · 青红椒', price: 1500, sales: 433, bg: C.green, ch: '锅', spec: true },
  { id: 'd09', cat: 'c2', name: '黑椒牛柳盖饭', desc: '黑椒汁 · 洋葱 · 彩椒', price: 2200, sales: 812, bg: C.brown, ch: '椒', spec: true },
  { id: 'd10', cat: 'c2', name: '咖喱鸡排饭', desc: '日式咖喱 · 现炸鸡排', price: 2000, sales: 977, bg: C.amber, ch: '咖', spec: true },
  { id: 'd11', cat: 'c2', name: '梅菜扣肉饭', desc: '客家梅干菜 · 五花肉', price: 1900, sales: 568, bg: C.brown, ch: '扣', spec: false },
  { id: 'd12', cat: 'c2', name: '香菇滑鸡饭', desc: '鲜香菇 · 嫩滑鸡腿', price: 1800, sales: 690, bg: C.amber, ch: '菇', spec: false },
  { id: 'd13', cat: 'c3', name: '红烧牛肉面', desc: '大块牛腩 · 手工碱面', price: 2100, sales: 1105, bg: C.red, ch: '面', spec: true },
  { id: 'd14', cat: 'c3', name: '重庆小面', desc: '红油 · 豌杂 · 香脆花生', price: 1300, sales: 880, bg: C.red, ch: '渝', spec: true },
  { id: 'd15', cat: 'c3', name: '螺蛳粉', desc: '酸笋 · 腐竹 · 花生', price: 1600, sales: 761, bg: C.amber, ch: '螺', spec: true },
  { id: 'd16', cat: 'c3', name: '葱油拌面', desc: '老上海葱油 · 开洋', price: 1000, sales: 534, bg: C.green, ch: '葱', spec: false },
  { id: 'd17', cat: 'c4', name: '鲜肉小笼包', desc: '6 只 · 现包现蒸', price: 1200, sales: 1388, bg: C.brown, ch: '笼', spec: false },
  { id: 'd18', cat: 'c4', name: '玉米蒸饺', desc: '8 只 · 甜玉米猪肉', price: 1000, sales: 620, bg: C.amber, ch: '饺', spec: false },
  { id: 'd19', cat: 'c4', name: '香酥春卷', desc: '4 根 · 荠菜肉馅', price: 800, sales: 410, bg: C.amber, ch: '卷', spec: false },
  { id: 'd20', cat: 'c5', name: '番茄菌菇汤', desc: '五种菌菇 · 番茄打底', price: 900, sales: 512, bg: C.red, ch: '汤', spec: false },
  { id: 'd21', cat: 'c5', name: '玉米排骨汤', desc: '小火慢炖 3 小时', price: 1500, sales: 466, bg: C.amber, ch: '炖', spec: false },
  { id: 'd22', cat: 'c5', name: '紫菜蛋花汤', desc: '清淡 · 暖胃', price: 500, sales: 903, bg: C.green, ch: '紫', spec: false },
  { id: 'd23', cat: 'c6', name: '鲜榨橙汁', desc: '现榨不加水 · 400ml', price: 900, sales: 377, bg: C.amber, ch: '橙', spec: false },
  { id: 'd24', cat: 'c6', name: '杨枝甘露', desc: '芒果 · 西柚 · 椰浆', price: 1200, sales: 845, bg: C.pink, ch: '芒', spec: false },
  { id: 'd25', cat: 'c6', name: '冰镇酸梅汤', desc: '乌梅 · 山楂 · 桂花', price: 600, sales: 1032, bg: C.brown, ch: '梅', spec: false },
  { id: 'd26', cat: 'c6', name: '双皮奶', desc: '水牛奶 · 红豆', price: 800, sales: 488, bg: C.blue, ch: '奶', spec: false }
];

// 规格：份量单选（差价）、辣度单选、加料多选
const SIZES = [
  { id: 's', name: '小份', delta: -200 },
  { id: 'm', name: '标准', delta: 0 },
  { id: 'l', name: '大份', delta: 400 }
];
const SPICY = ['不辣', '微辣', '中辣', '特辣'];
const EXTRAS = [
  { id: 'egg', name: '加煎蛋', price: 200 },
  { id: 'meat', name: '加肉', price: 500 },
  { id: 'veg', name: '加青菜', price: 200 },
  { id: 'rice', name: '加米饭', price: 100 }
];

// 桌台：A 大厅、B 窗边、C 包间；occupied 为已占用
const ZONES = [
  { id: 'A', name: '大厅', count: 20, occupied: [3, 7, 8, 12, 16] },
  { id: 'B', name: '窗边', count: 12, occupied: [2, 5, 9] },
  { id: 'C', name: '包间', count: 6, occupied: [4] }
];

const CANTEEN = { name: '禾味食堂 · 二楼', phone: '0571-8697 2210', hours: '10:30-14:00 / 17:00-20:30' };

// 宿主没有扫码能力时，页内模拟取景框「识别」的那张演示桌贴（内容与真桌贴同一格式）
const DEMO_STICKER = 'https://hewei.example.com/canteen/t?table=B06';

// 饭卡充值档位（分）
const RECHARGES = [5000, 10000, 20000];

const RATE_TAGS = ['味道好', '分量足', '出餐快', '干净卫生', '服务热情'];

const FEEDBACK_TYPES = ['菜品口味', '分量价格', '环境卫生', '服务态度', '其他建议'];

const FAQS = [
  { q: '怎么扫码点餐？', a: '在「首页」点「扫桌贴二维码点餐」，对准桌角的二维码即可识别桌号；扫不出来时，手动输入桌贴下方印的桌号（如 B06），或在下方直接选一张空闲桌。' },
  { q: '下单后还能取消吗？', a: '待支付的订单随时可以取消；已付款的订单在后厨开做前（订单详情显示「已下单」）可以取消，饭卡支付的钱立即退回饭卡，微信支付原路退回，开做之后就不能取消了。' },
  { q: '可以加菜吗？', a: '可以。在订单详情点「加菜」回到点餐页，桌号不变，加的菜单独成一单、单独出餐。' },
  { q: '饭卡怎么充值？', a: '「我的」→「饭卡」选金额后微信支付即可，到账后马上能用；每一笔充值、消费、退款都在饭卡明细里。' },
  { q: '打包要收费吗？', a: '堂食不收费；选「打包」每份收 1 元打包盒费用，满减照常享受。' }
];

module.exports = { CATS, DISHES, SIZES, SPICY, EXTRAS, ZONES, CANTEEN, DEMO_STICKER, RECHARGES, RATE_TAGS, FEEDBACK_TYPES, FAQS };
