// 静态业务数据（上海地区家政服务 2026 年市场价）。
// type: hourly 按小时计（stepper 选时长），spec 按规格计（选规格 + 数量）
const SERVICES = [
  {
    id: 'daily', name: '日常保洁', icon: '/images/svc/daily.png', cover: '/images/svc/daily-cover.jpg', type: 'hourly',
    price: 45, unit: '小时', minQty: 3, maxQty: 8, sold: 182364, rate: '99.2%',
    slogan: '3 小时起约，持证阿姨自带工具上门',
    specs: [{ id: 'h', name: '按小时计费', price: 45 }],
    standard: ['客厅：地面吸尘拖洗、桌面茶几擦拭、沙发表面除尘', '卧室：床头柜衣柜表面擦拭、床底吸尘、整理床铺', '厨房：灶台台面去油、水槽清洁、柜门擦拭', '卫生间：马桶、淋浴区、镜面、地漏清洁消毒'],
    notice: ['服务时长从阿姨到达开始计算', '请提前准备好拖把、抹布以外的专用清洁剂（如有特殊材质）', '贵重物品请自行收好', '超出预约时长按 ¥45/小时 现场补差']
  },
  {
    id: 'deep', name: '深度保洁', icon: '/images/svc/deep.png', cover: '/images/svc/deep-cover.jpg', type: 'spec',
    price: 398, unit: '套', minQty: 1, maxQty: 1, sold: 46210, rate: '98.7%',
    slogan: '2 位保洁师协作，厨卫深度去污除垢',
    specs: [
      { id: 'd60', name: '60㎡ 以下', price: 398 }, { id: 'd90', name: '60~90㎡', price: 528 },
      { id: 'd120', name: '90~120㎡', price: 658 }, { id: 'd150', name: '120~150㎡', price: 798 }
    ],
    standard: ['全屋玻璃（内侧）与窗框窗槽清洁', '厨房油烟机外表、灶具、瓷砖重油污处理', '卫生间水垢、玻璃隔断、五金件抛光', '踢脚线、门框、开关面板逐一擦拭'],
    notice: ['约 5~6 小时完成，2 位保洁师同时上门', '高空外窗、吊顶不在服务范围', '面积以房产证套内面积为准']
  },
  {
    id: 'appliance', name: '家电清洗', icon: '/images/svc/appliance.png', cover: '/images/svc/appliance-cover.jpg', type: 'spec',
    price: 89, unit: '台', minQty: 1, maxQty: 6, sold: 93571, rate: '99.0%',
    slogan: '拆洗消毒，高温蒸汽去除霉菌异味',
    specs: [
      { id: 'ac1', name: '挂式空调', price: 89 }, { id: 'ac2', name: '柜式空调', price: 139 },
      { id: 'hood', name: '油烟机（侧吸 / 顶吸）', price: 199 }, { id: 'wm', name: '波轮洗衣机', price: 129 }, { id: 'fr', name: '冰箱', price: 119 }
    ],
    standard: ['断电拆卸面板、滤网，专用药剂浸泡', '高温蒸汽冲洗内机 / 内筒，杀菌除螨', '装回后通电试机，确认运行正常'],
    notice: ['中央空调、嵌入式家电请联系客服报价', '服务前请清空洗衣机 / 冰箱内物品']
  },
  {
    id: 'newhome', name: '开荒保洁', icon: '/images/svc/newhome.png', cover: '/images/svc/newhome-cover.jpg', type: 'spec',
    price: 720, unit: '套', minQty: 1, maxQty: 1, sold: 12086, rate: '98.1%',
    slogan: '装修后首次入住，清除水泥胶渍粉尘',
    specs: [{ id: 'n90', name: '90㎡ 以下', price: 720 }, { id: 'n120', name: '90~120㎡', price: 960 }, { id: 'n150', name: '120~150㎡', price: 1200 }],
    standard: ['铲除地面、玻璃上的水泥点、胶渍、油漆点', '全屋粉尘清理，柜内柜外擦拭', '门窗轨道、卫浴五金除胶'],
    notice: ['请确认水电已通', '按 ¥8/㎡ 计价，现场面积超出按实际补差']
  },
  {
    id: 'pipe', name: '管道疏通', icon: '/images/svc/pipe.png', cover: '/images/svc/pipe-cover.jpg', type: 'spec',
    price: 80, unit: '处', minQty: 1, maxQty: 4, sold: 35422, rate: '98.9%',
    slogan: '30 分钟响应，不通不收费',
    specs: [{ id: 'p1', name: '地漏 / 洗手盆', price: 80 }, { id: 'p2', name: '马桶 / 下水道', price: 120 }, { id: 'p3', name: '主管道（电动疏通）', price: 260 }],
    standard: ['先检测堵塞位置再报价，确认后施工', '疏通后放水测试 3 分钟', '清理施工产生的污物'],
    notice: ['因管道老化破损需更换的，材料费另计', '不通不收费，7 天内复堵免费返工']
  },
  {
    id: 'electric', name: '水电维修', icon: '/images/svc/electric.png', cover: '/images/svc/electric-cover.jpg', type: 'spec',
    price: 50, unit: '项', minQty: 1, maxQty: 10, sold: 28790, rate: '99.1%',
    slogan: '持电工证师傅，明码标价先报价后施工',
    specs: [
      { id: 'e0', name: '上门检测（不维修）', price: 50 }, { id: 'e1', name: '开关 / 插座更换', price: 30 },
      { id: 'e2', name: '灯具安装（吸顶灯）', price: 60 }, { id: 'e3', name: '水龙头更换', price: 60 }
    ],
    standard: ['施工前断电断水并做好防护', '使用国标线材与配件，可用自购材料', '完工后通电通水测试'],
    notice: ['材料费另计，师傅现场出示价格单', '强电改造需物业审批的请提前办理']
  },
  {
    id: 'install', name: '家具安装', icon: '/images/svc/install.png', cover: '/images/svc/install-cover.jpg', type: 'spec',
    price: 60, unit: '件', minQty: 1, maxQty: 10, sold: 19654, rate: '98.6%',
    slogan: '网购家具上门组装，墙面打孔一次到位',
    specs: [{ id: 'i1', name: '书桌 / 床头柜', price: 60 }, { id: 'i2', name: '床（1.5~1.8m）', price: 120 }, { id: 'i3', name: '衣柜（2 门以上）', price: 180 }, { id: 'i4', name: '窗帘杆 / 置物架', price: 50 }],
    standard: ['按说明书组装，检查配件是否齐全', '需打孔的先探测墙内管线', '清理包装垃圾并带走'],
    notice: ['缺件或板材破损请先联系商家补发', '大理石、瓷砖墙面打孔加收 ¥20/孔']
  },
  {
    id: 'move', name: '同城搬家', icon: '/images/svc/move.png', cover: '/images/svc/move-cover.jpg', type: 'spec',
    price: 299, unit: '车', minQty: 1, maxQty: 3, sold: 8432, rate: '97.9%',
    slogan: '一口价含 2 位搬运师傅，10 公里内不加价',
    specs: [{ id: 'm1', name: '小面包车（约 2.5 方）', price: 299 }, { id: 'm2', name: '金杯车（约 5 方）', price: 399 }, { id: 'm3', name: '4.2 米厢货（约 12 方）', price: 599 }],
    standard: ['2 位师傅负责搬上搬下', '家具包角、电器缠膜保护', '超过 10 公里按 ¥6/公里 计费'],
    notice: ['无电梯楼层每层加收 ¥20', '钢琴、保险柜等大件请联系客服']
  }
];

// 推荐的阿姨 / 师傅（按服务类型派单）
const STAFF = [
  { id: 'a1', name: '王秀英', avatar: '/images/staff/a1.jpg', title: '金牌保洁师', years: 8, orders: 2316, good: '99.6%', home: '安徽六安', skills: ['daily', 'deep', 'newhome'], phone: '13917264508', intro: '做事细致，擅长厨房重油污和收纳整理。' },
  { id: 'a2', name: '李春梅', avatar: '/images/staff/a2.jpg', title: '家电清洗技师', years: 6, orders: 1782, good: '99.3%', home: '江西九江', skills: ['appliance', 'daily'], phone: '13817520936', intro: '持家电清洗上岗证，空调、油烟机拆洗经验丰富。' },
  { id: 'a3', name: '张建国', avatar: '/images/staff/a3.jpg', title: '水电维修师傅', years: 12, orders: 3105, good: '99.1%', home: '江苏盐城', skills: ['electric', 'pipe'], phone: '13601837742', intro: '持低压电工证，先报价后施工，从不乱加价。' },
  { id: 'a4', name: '陈卫东', avatar: '/images/staff/a4.jpg', title: '安装搬运师傅', years: 9, orders: 1468, good: '98.8%', home: '河南信阳', skills: ['install', 'move'], phone: '13761492058', intro: '宜家、全友等品牌家具装过上千件。' }
];

// 优惠券模板：cat 为空表示全品类
const COUPONS = [
  { id: 'cp1', name: '新客专享券', amount: 30, min: 99, cat: '', expire: '2026-12-31' },
  { id: 'cp2', name: '家电清洗券', amount: 20, min: 150, cat: 'appliance', expire: '2026-12-31' },
  { id: 'cp3', name: '深度保洁券', amount: 50, min: 400, cat: 'deep', expire: '2026-11-30' },
  { id: 'cp4', name: '国庆焕新券', amount: 15, min: 80, cat: '', expire: '2026-10-08' }
];
// 兑换码 → 券
const REDEEM = { HAOGUANJIA: { id: 'cpx', name: '兑换码专享券', amount: 25, min: 120, cat: '', expire: '2027-03-31' } };

const CANCEL_REASONS = ['时间安排有变', '地址填错了', '已找到其他师傅', '价格比预期高', '其他原因'];
const REVIEW_TAGS = ['准时上门', '干活细致', '工具齐全', '态度很好', '收费透明', '会再预约'];

const HOTLINE = '400-820-5566';

module.exports = { SERVICES, STAFF, COUPONS, REDEEM, CANCEL_REASONS, REVIEW_TAGS, HOTLINE };
