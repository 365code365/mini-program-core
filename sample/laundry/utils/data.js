// 价目表（单位：元/件，窗帘按平方米），参考一线城市上门洗衣的常见定价
const CATS = [
  { key: 'wash', name: '衣物洗护', icon: '/images/svc/wash.png', tip: '水洗 / 干洗按面料自动匹配' },
  { key: 'down', name: '羽绒服', icon: '/images/svc/down.png', tip: '专用羽绒洗剂，蓬松度检测' },
  { key: 'shoe', name: '鞋靴清洗', icon: '/images/svc/shoe.png', tip: '刷洗 + 除臭 + 鞋型固定' },
  { key: 'home', name: '家纺窗帘', icon: '/images/svc/home.png', tip: '窗帘可预约上门拆装' },
  { key: 'luxury', name: '奢品护理', icon: '/images/svc/luxury.png', tip: '一对一护理师，全程影像留档' }
];

const ITEMS = [
  { id: 'w1', cat: 'wash', name: 'T恤 / 衬衫', price: 15, unit: '件', desc: '含熨烫' },
  { id: 'w2', cat: 'wash', name: '毛衣 / 针织衫', price: 25, unit: '件', desc: '低温平铺晾干' },
  { id: 'w3', cat: 'wash', name: '西裤 / 休闲裤', price: 20, unit: '件', desc: '裤线定型' },
  { id: 'w4', cat: 'wash', name: '连衣裙', price: 30, unit: '件', desc: '按面料水洗 / 干洗' },
  { id: 'w5', cat: 'wash', name: '西装上衣', price: 35, unit: '件', desc: '干洗 + 立体熨烫' },
  { id: 'w6', cat: 'wash', name: '大衣（短款）', price: 45, unit: '件', desc: '干洗，去静电' },
  { id: 'w7', cat: 'wash', name: '大衣（长款）', price: 59, unit: '件', desc: '干洗，去静电' },
  { id: 'd1', cat: 'down', name: '羽绒马甲', price: 35, unit: '件', desc: '' },
  { id: 'd2', cat: 'down', name: '羽绒服（短款）', price: 45, unit: '件', desc: '' },
  { id: 'd3', cat: 'down', name: '羽绒服（长款）', price: 65, unit: '件', desc: '过膝款' },
  { id: 's1', cat: 'shoe', name: '运动鞋 / 板鞋', price: 29, unit: '双', desc: '含鞋带清洗' },
  { id: 's2', cat: 'shoe', name: '皮鞋', price: 35, unit: '双', desc: '清洁 + 上油护理' },
  { id: 's3', cat: 'shoe', name: '雪地靴', price: 49, unit: '双', desc: '内里除菌' },
  { id: 's4', cat: 'shoe', name: '短靴', price: 45, unit: '双', desc: '' },
  { id: 's5', cat: 'shoe', name: '长靴', price: 69, unit: '双', desc: '靴筒定型' },
  { id: 'h1', cat: 'home', name: '床品四件套', price: 39, unit: '套', desc: '高温除螨' },
  { id: 'h2', cat: 'home', name: '被芯（单人）', price: 49, unit: '条', desc: '' },
  { id: 'h3', cat: 'home', name: '毛毯', price: 45, unit: '条', desc: '' },
  { id: 'h4', cat: 'home', name: '枕芯', price: 19, unit: '个', desc: '' },
  { id: 'h5', cat: 'home', name: '窗帘', price: 12, unit: '㎡', desc: '按展开面积计' },
  { id: 'l1', cat: 'luxury', name: '真丝衬衫 / 连衣裙', price: 59, unit: '件', desc: '手工洗护' },
  { id: 'l2', cat: 'luxury', name: '羊绒大衣', price: 129, unit: '件', desc: '去球 + 柔顺护理' },
  { id: 'l3', cat: 'luxury', name: '皮衣 / 皮草', price: 199, unit: '件', desc: '清洁 + 补色 + 上光' },
  { id: 'l4', cat: 'luxury', name: '奢品包清洁护理', price: 299, unit: '只', desc: '五金抛光，影像留档' }
];

// 加购服务（checkbox）
const ADDONS = [
  { key: 'fast', name: '加急 24 小时送回', price: 15 },
  { key: 'mite', name: '除螨杀菌', price: 9 },
  { key: 'bag', name: '环保收纳袋', price: 0 }
];

// 上门取件时段
const SLOTS = ['09:00-11:00', '11:00-13:00', '13:00-15:00', '15:00-17:00', '17:00-19:00', '19:00-21:00'];

// 订单进度（细分节点）→ 列表里的大状态：0 待取件 / 1 清洗中 / 2 待送回 / 3 已完成
const STEPS = [
  { text: '订单已提交', desc: '等待取件员接单', main: 0 },
  { text: '取件员已接单', desc: '取件员王建国（工号 0732）将按预约时段上门', main: 0 },
  { text: '已上门取件', desc: '衣物已装袋封签，送往洗护工厂', main: 1 },
  { text: '入厂质检', desc: '逐件拍照登记，检查污渍与破损', main: 1 },
  { text: '清洗中', desc: '按面料分拣，水洗 / 干洗分线处理', main: 1 },
  { text: '烘干熨烫', desc: '低温烘干，蒸汽熨烫定型', main: 1 },
  { text: '出厂质检', desc: '复检通过，包装封袋', main: 2 },
  { text: '配送中', desc: '送件员正在送往你的地址', main: 2 },
  { text: '已签收', desc: '洗护完成，欢迎评价', main: 3 }
];

const STATUS = [
  { key: 0, name: '待取件' },
  { key: 1, name: '清洗中' },
  { key: 2, name: '待送回' },
  { key: 3, name: '已完成' }
];

// 质检照片：入厂质检节点之后才出现
const QC = [
  { src: '/images/qc/qc1.jpg', note: '领口轻微发黄，已做专项去渍' },
  { src: '/images/qc/qc2.jpg', note: '袖口有淡色旧渍，处理后明显减淡（陈年渍无法保证完全去除）' },
  { src: '/images/qc/qc3.jpg', note: '鞋面开胶处已标记，清洗时避开' },
  { src: '/images/qc/qc4.jpg', note: '水洗标识：可干洗、不可漂白、低温熨烫' }
];

const BANNERS = [
  { img: '/images/banner/b1.jpg', title: '新客首单立减 10 元', sub: '2 小时内上门取件', cat: 'wash' },
  { img: '/images/banner/b2.jpg', title: '洗鞋 29 元起', sub: '刷洗除臭，72 小时送回', cat: 'shoe' },
  { img: '/images/banner/b3.jpg', title: '窗帘上门拆装', sub: '12 元/㎡，换季一次洗净', cat: 'home' }
];

const COURIER = { name: '王建国', phone: '400-820-6688', no: '0732' };
const SERVICE_PHONE = '400-820-6688';

function item(id) {
  return ITEMS.find(x => x.id === id) || null;
}

module.exports = { CATS, ITEMS, ADDONS, SLOTS, STEPS, STATUS, QC, BANNERS, COURIER, SERVICE_PHONE, item };
