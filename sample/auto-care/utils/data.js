// 静态业务数据：门店、服务项目、品牌车型、省份简称。价格参照杭州连锁养护门店 2026 年公开报价。

// 定位失败时的默认坐标：杭州武林广场
const DEFAULT_LOC = { latitude: 30.2741, longitude: 120.1551, name: '杭州市 · 武林广场' };

const STORES = [
  {
    id: 's1', name: '车管家 · 滨江旗舰店', short: '滨江旗舰店', img: '/images/store/s1.jpg',
    address: '杭州市滨江区江南大道 3850 号创新大厦 1 楼', latitude: 30.2085, longitude: 120.2108,
    phone: '0571-86651234', hours: '08:30-20:00', rating: 4.9, orders: 12860, bays: 12,
    tags: ['原厂配件', '钣喷中心', '新能源'], notice: '店内提供免费 Wi-Fi 与休息区，单次保养平均 60 分钟。'
  },
  {
    id: 's2', name: '车管家 · 西溪店', short: '西溪店', img: '/images/store/s2.jpg',
    address: '杭州市西湖区文二西路 808 号西溪印象城 B2', latitude: 30.2797, longitude: 120.0937,
    phone: '0571-88072345', hours: '09:00-21:00', rating: 4.8, orders: 9632, bays: 8,
    tags: ['商场停车', '精洗美容'], notice: '商场 B2 停车场 C 区进入，凭工单免停车费 3 小时。'
  },
  {
    id: 's3', name: '车管家 · 城西银泰店', short: '城西银泰店', img: '/images/store/s3.jpg',
    address: '杭州市拱墅区丰潭路 380 号城西银泰城北门', latitude: 30.2967, longitude: 120.1129,
    phone: '0571-85123456', hours: '08:30-19:30', rating: 4.7, orders: 7415, bays: 6,
    tags: ['轮胎专营', '四轮定位'], notice: '轮胎库存 300+ 规格，换季高峰建议提前一天预约。'
  },
  {
    id: 's4', name: '车管家 · 萧山机场路店', short: '萧山机场路店', img: '/images/store/s4.jpg',
    address: '杭州市萧山区机场路 1288 号', latitude: 30.2361, longitude: 120.3127,
    phone: '0571-82689012', hours: '08:00-20:00', rating: 4.8, orders: 5288, bays: 10,
    tags: ['年检代办', '24h 救援'], notice: '可代办车辆年检，周一至周五当天出结果。'
  }
];

// 服务分类 + 项目（mins 是工位占用时长）
const CATS = [
  { id: 'care', name: '保养', icon: '/images/svc/oil.png' },
  { id: 'wash', name: '洗车', icon: '/images/svc/wash.png' },
  { id: 'tire', name: '轮胎', icon: '/images/svc/tire.png' },
  { id: 'beauty', name: '美容', icon: '/images/svc/beauty.png' },
  { id: 'paint', name: '钣喷', icon: '/images/svc/paint.png' },
  { id: 'check', name: '年检', icon: '/images/svc/check.png' }
];

const SERVICES = [
  { id: 'v1', cat: 'care', name: '小保养', desc: '全合成机油 4L + 机油滤芯 + 工时', price: 299, mins: 60, hot: true },
  { id: 'v2', cat: 'care', name: '大保养', desc: '全合成机油 + 机滤 + 空滤 + 空调滤 + 工时', price: 699, mins: 120 },
  { id: 'v3', cat: 'care', name: '刹车油更换', desc: 'DOT4 刹车油 1L，含排空工时', price: 258, mins: 40 },
  { id: 'v4', cat: 'care', name: '空调滤芯更换', desc: '活性炭空调滤芯，过滤 PM2.5', price: 88, mins: 15 },
  { id: 'v5', cat: 'wash', name: '标准洗车', desc: '外观泡沫清洗 + 轮毂 + 吸尘', price: 35, mins: 30, hot: true },
  { id: 'v6', cat: 'wash', name: '精洗（含内饰）', desc: '外观精洗 + 内饰清洁 + 玻璃除油膜', price: 88, mins: 60 },
  { id: 'v7', cat: 'tire', name: '四轮定位', desc: '3D 定位仪检测并调整前束、外倾角', price: 128, mins: 40 },
  { id: 'v8', cat: 'tire', name: '轮胎动平衡', desc: '四轮动平衡，消除高速方向盘抖动', price: 80, mins: 30 },
  { id: 'v9', cat: 'tire', name: '补胎', desc: '蘑菇钉内补，含拆装与充气', price: 30, mins: 30 },
  { id: 'v10', cat: 'beauty', name: '全车打蜡', desc: '巴西棕榈蜡，光泽保持约 2 个月', price: 168, mins: 60 },
  { id: 'v11', cat: 'beauty', name: '全车镀晶', desc: '9H 镀晶，抗氧化，保持约 1 年', price: 1280, mins: 240 },
  { id: 'v12', cat: 'paint', name: '单面喷漆', desc: '进口漆面，烤房烘烤，质保 3 年', price: 380, mins: 360 },
  { id: 'v13', cat: 'paint', name: '划痕修复', desc: '轻微划痕抛光修复（未见底漆）', price: 200, mins: 90 },
  { id: 'v14', cat: 'check', name: '年检代办', desc: '代送检测站，含上线检测服务费', price: 198, mins: 180 },
  { id: 'v15', cat: 'check', name: '36 项全车检测', desc: '底盘、灯光、电瓶、油液全面体检', price: 0, mins: 30, hot: true }
];

// 品牌按首字母；models 是常见在售车型
const BRANDS = [
  { id: 'b-audi', letter: 'A', name: '奥迪', models: ['A4L', 'A6L', 'Q3', 'Q5L'] },
  { id: 'b-bmw', letter: 'B', name: '宝马', models: ['3 系', '5 系', 'X1', 'X3'] },
  { id: 'b-benz', letter: 'B', name: '奔驰', models: ['C 级', 'E 级', 'GLC', 'GLB'] },
  { id: 'b-honda', letter: 'B', name: '本田', models: ['雅阁', '思域', 'CR-V', '飞度'] },
  { id: 'b-byd', letter: 'B', name: '比亚迪', models: ['秦 PLUS', '汉', '宋 PLUS', '海豹', '元 PLUS'] },
  { id: 'b-changan', letter: 'C', name: '长安', models: ['CS75 PLUS', 'UNI-V', '逸动'] },
  { id: 'b-vw', letter: 'D', name: '大众', models: ['帕萨特', '迈腾', '朗逸', '途观 L', 'ID.4'] },
  { id: 'b-toyota', letter: 'F', name: '丰田', models: ['凯美瑞', '卡罗拉', 'RAV4 荣放', '汉兰达'] },
  { id: 'b-ford', letter: 'F', name: '福特', models: ['蒙迪欧', '锐界', '探险者'] },
  { id: 'b-geely', letter: 'G', name: '吉利', models: ['星越 L', '博越 L', '帝豪'] },
  { id: 'b-li', letter: 'L', name: '理想', models: ['L6', 'L7', 'L8', 'L9'] },
  { id: 'b-nissan', letter: 'R', name: '日产', models: ['轩逸', '天籁', '奇骏'] },
  { id: 'b-tesla', letter: 'T', name: '特斯拉', models: ['Model 3', 'Model Y'] },
  { id: 'b-nio', letter: 'W', name: '蔚来', models: ['ET5', 'ES6', 'ES8'] },
  { id: 'b-xpeng', letter: 'X', name: '小鹏', models: ['P7+', 'G6', 'MONA M03'] },
  { id: 'b-xiaomi', letter: 'X', name: '小米', models: ['SU7', 'YU7'] }
];

const PROVINCES = ['浙', '沪', '苏', '皖', '京', '粤', '闽', '赣', '鲁', '豫', '湘', '鄂', '川', '渝'];
const CITY_LETTERS = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L'];

// 车身颜色 → 车卡图
const CAR_IMGS = { white: '/images/car/car-white.jpg', blue: '/images/car/car-blue.jpg', red: '/images/car/car-red.jpg' };
const COLORS = [
  { key: 'white', name: '珍珠白' },
  { key: 'blue', name: '星空蓝' },
  { key: 'red', name: '烈焰红' }
];

const STATUS_TEXT = { pending: '待服务', serving: '服务中', done: '待评价', reviewed: '已完成', cancelled: '已取消' };

const CANCEL_REASONS = ['行程有变，改天再来', '预约时间选错了', '已在其他门店完成服务', '价格不合适'];

const REVIEW_TAGS = ['技师专业', '准时完工', '价格透明', '环境整洁', '休息区舒适', '会推荐朋友'];

const HOTLINE = '400-826-8899';

module.exports = {
  DEFAULT_LOC, STORES, CATS, SERVICES, BRANDS, PROVINCES, CITY_LETTERS, CAR_IMGS, COLORS,
  STATUS_TEXT, CANCEL_REASONS, REVIEW_TAGS, HOTLINE
};
