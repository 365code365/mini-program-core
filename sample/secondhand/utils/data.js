// 闲转的静态数据：分类、卖家、在售宝贝、热搜词。价格与成色按二手平台常见行情写
const G = '/images/goods/';
const CATS = [
  { key: 'digital', name: '手机数码', icon: '/images/cat/phone.png' },
  { key: 'office', name: '电脑办公', icon: '/images/cat/laptop.png' },
  { key: 'appliance', name: '家用电器', icon: '/images/cat/appliance.png' },
  { key: 'book', name: '图书教材', icon: '/images/cat/book.png' },
  { key: 'sport', name: '文体户外', icon: '/images/cat/sport.png' },
  { key: 'baby', name: '母婴玩具', icon: '/images/cat/baby.png' },
  { key: 'fashion', name: '服饰鞋包', icon: '/images/cat/fashion.png' },
  { key: 'home', name: '家具家居', icon: '/images/cat/home.png' }
];
const CONDS = ['全新', '99新', '95新', '9成新', '8成新'];
const SELLERS = {
  s1: { id: 's1', name: '林小满', avatar: '/images/avatar/a1.jpg', city: '杭州·西湖区', credit: '信用极好', reply: '98%', joined: '来闲转 4 年' },
  s2: { id: 's2', name: '阿杰的数码铺', avatar: '/images/avatar/a2.jpg', city: '杭州·滨江区', credit: '信用极好', reply: '95%', joined: '来闲转 6 年' },
  s3: { id: 's3', name: '苏苏', avatar: '/images/avatar/a3.jpg', city: '杭州·拱墅区', credit: '信用优秀', reply: '90%', joined: '来闲转 2 年' },
  s4: { id: 's4', name: '老陈不折腾', avatar: '/images/avatar/a4.jpg', city: '杭州·余杭区', credit: '信用优秀', reply: '86%', joined: '来闲转 5 年' },
  s5: { id: 's5', name: 'Momo', avatar: '/images/avatar/a5.jpg', city: '杭州·上城区', credit: '信用极好', reply: '99%', joined: '来闲转 3 年' },
  s6: { id: 's6', name: '浙大王同学', avatar: '/images/avatar/a6.jpg', city: '杭州·西湖区', credit: '信用良好', reply: '92%', joined: '来闲转 1 年' }
};
const ME = { id: 'me', name: '闲转用户_8826', avatar: '/images/avatar/a7.jpg', city: '杭州·西湖区' };
// ratio：首图高宽比，瀑布流按它估算列高；days：几天前发布
const ITEMS = [
  { id: 'i01', title: 'iPhone 13 128G 星光色 国行 电池健康 88%', price: 2650, orig: 5999, cond: '95新', cat: 'digital', imgs: ['g_iphone', 'g_iphone'], ratio: 1.25, seller: 's1', wants: 36, views: 812, days: 1, free: true, desc: '2022 年 3 月官网购入，一直带壳贴膜使用，屏幕无划痕，边框有一处细微磕碰（见第二张图）。面容正常，无拆无修，配原装盒子和数据线。可当面验机，支持同城自提。' },
  { id: 'i02', title: '任天堂 Switch OLED 日版 + 塞尔达王国之泪卡带', price: 1680, orig: 2599, cond: '99新', cat: 'digital', imgs: ['g_switch'], ratio: 0.8, seller: 's2', wants: 58, views: 1290, days: 0, free: true, desc: '去年年底入手，玩通了王国之泪就吃灰了。屏幕贴了钢化膜，Joy-Con 无漂移，底座、电源、HDMI 线齐全。卡带一起出，不单卖。' },
  { id: 'i03', title: '宜家 阿德尔书桌 120×60 白色 仅限自提', price: 120, orig: 349, cond: '9成新', cat: 'home', imgs: ['g_desk'], ratio: 1.0, seller: 's3', wants: 9, views: 203, days: 3, free: false, desc: '搬家出，桌面有轻微使用痕迹，结构稳固不晃。已拆成桌面 + 四条腿，小轿车后备箱能放下。文三路附近自提。' },
  { id: 'i04', title: '小米手环 8 NFC 版 陶瓷白 盒说全', price: 129, orig: 299, cond: '99新', cat: 'digital', imgs: ['g_band'], ratio: 1.3, seller: 's5', wants: 14, views: 356, days: 2, free: true, desc: '公司年会抽的，戴了不到一周，表带换回原装白色。NFC 可以刷杭州公交和门禁。' },
  { id: 'i05', title: 'Kindle Paperwhite 5 8G 带原装保护套', price: 520, orig: 1068, cond: '95新', cat: 'digital', imgs: ['g_kindle'], ratio: 1.33, seller: 's6', wants: 21, views: 467, days: 4, free: true, desc: '6.8 英寸屏，可调色温，屏幕无坏点无划痕。已退出账号恢复出厂，送一个磁吸保护套。' },
  { id: 'i06', title: '张宇考研数学 18 讲 + 1000 题 2025 版 几乎没写', price: 45, orig: 159, cond: '9成新', cat: 'book', imgs: ['g_books'], ratio: 1.0, seller: 's6', wants: 12, views: 288, days: 1, free: false, desc: '买来只写了前两章，后面全新。一共四本，不拆卖，邮费到付或者紫金港校区自取。' },
  { id: 'i07', title: '迪卡侬 RC100 公路车 M 码 适合 170~180', price: 680, orig: 1599, cond: '9成新', cat: 'sport', imgs: ['g_bike'], ratio: 0.75, seller: 's4', wants: 17, views: 395, days: 5, free: false, desc: '骑了大概 800 公里，链条刚换过，刹车皮八成新。送码表架和水壶架。只限自提，可以试骑。' },
  { id: 'i08', title: '戴森 HD08 吹风机 紫红色 国行带发票', price: 1580, orig: 2999, cond: '95新', cat: 'appliance', imgs: ['g_dryer'], ratio: 1.25, seller: 's5', wants: 44, views: 1021, days: 2, free: true, desc: '2023 年京东自营购买，发票和保修卡都在，还有一年多保修。五个风嘴齐全，滤网清洁过。' },
  { id: 'i09', title: '索尼 WH-1000XM4 降噪耳机 黑色', price: 980, orig: 2299, cond: '9成新', cat: 'digital', imgs: ['g_headphone'], ratio: 1.0, seller: 's2', wants: 27, views: 640, days: 6, free: true, desc: '降噪和续航都正常，左耳罩有一点点压痕不影响佩戴。原装收纳包、充电线、飞机转接头都在。' },
  { id: 'i10', title: '佳能 EOS M50 二代 + 15-45 套机 快门 3200', price: 2980, orig: 4999, cond: '95新', cat: 'digital', imgs: ['g_camera'], ratio: 0.8, seller: 's1', wants: 19, views: 530, days: 3, free: true, desc: '入门微单，翻转屏适合拍 vlog。快门数约 3200，镜头无霉无雾。送 64G 卡和两块电池。' },
  { id: 'i11', title: '乐高 42115 兰博基尼 已拼 无缺件 带说明书', price: 1350, orig: 2999, cond: '9成新', cat: 'baby', imgs: ['g_lego'], ratio: 1.0, seller: 's4', wants: 11, views: 274, days: 8, free: false, desc: '拼好后放在亚克力展示盒里，没怎么积灰。说明书和外盒都留着，可以拆散发货或者整车自提。' },
  { id: 'i12', title: '好孩子 婴儿推车 可坐可躺 一键收车', price: 260, orig: 899, cond: '8成新', cat: 'baby', imgs: ['g_stroller'], ratio: 1.25, seller: 's3', wants: 8, views: 190, days: 7, free: false, desc: '宝宝大了用不上，车篷有点褪色，刹车和避震都好。洗过晒过，送雨罩和挂钩。' },
  { id: 'i13', title: 'Keep 瑜伽垫 TPE 6mm 紫色 送绑带', price: 39, orig: 129, cond: '95新', cat: 'sport', imgs: ['g_yoga'], ratio: 0.8, seller: 's5', wants: 6, views: 98, days: 2, free: false, desc: '只在家用过几次，没什么味道，双面防滑。卷好绑带一起送。' },
  { id: 'i14', title: '美的 空气炸锅 5L 可视窗 无油烟', price: 99, orig: 299, cond: '99新', cat: 'appliance', imgs: ['g_airfryer'], ratio: 1.25, seller: 's3', wants: 15, views: 322, days: 1, free: false, desc: '618 买重了，只用过一次烤鸡翅，内胆不粘涂层完好。原箱在，同城可以送货上门。' },
  { id: 'i15', title: '雅马哈 F310 民谣吉他 41 寸 送琴包变调夹', price: 450, orig: 899, cond: '9成新', cat: 'sport', imgs: ['g_guitar'], ratio: 1.33, seller: 's6', wants: 10, views: 211, days: 9, free: false, desc: '大学社团用过一年，琴颈直，弦距正常，刚换了一套新弦。送琴包、变调夹、拨片和调音器。' },
  { id: 'i16', title: 'Nike Air Force 1 白色 42 码 只穿过两次', price: 360, orig: 799, cond: '99新', cat: 'fashion', imgs: ['g_sneaker'], ratio: 0.8, seller: 's2', wants: 23, views: 566, days: 0, free: true, desc: '得物购入有鉴定证书，尺码偏大换了一双，鞋底几乎没磨损。原盒原配件。' },
  { id: 'i17', title: 'MacBook Air M1 8+256 深空灰 电池循环 126 次', price: 3999, orig: 7999, cond: '95新', cat: 'office', imgs: ['g_laptop'], ratio: 0.8, seller: 's1', wants: 52, views: 1530, days: 1, free: true, desc: '教育优惠购入，外观无磕碰，键盘无油光，电池健康 93%。已抹掉激活锁，配原装充电器。' },
  { id: 'i18', title: 'iPhone 12 64G 蓝色 换过官方电池', price: 1599, orig: 4599, cond: '9成新', cat: 'digital', imgs: ['g_iphone'], ratio: 1.25, seller: 's4', wants: 18, views: 420, days: 4, free: true, desc: '去年在 Apple Store 换的电池，健康 100%。背面玻璃完好，边框有使用划痕。只出手机和充电线。' },
  { id: 'i19', title: 'ThinkPad X1 Carbon 2021 i5 16G 512G', price: 3200, orig: 9999, cond: '9成新', cat: 'office', imgs: ['g_laptop'], ratio: 0.8, seller: 's2', wants: 13, views: 388, days: 5, free: true, desc: '公司配发后自购的，14 寸 2K 屏，重 1.1kg 方便出差。C 面有轻微磨损，已重装正版 Windows 11。' },
  { id: 'i20', title: '《三体》全集 典藏版 三册 只看过一遍', price: 58, orig: 168, cond: '95新', cat: 'book', imgs: ['g_books'], ratio: 1.0, seller: 's6', wants: 7, views: 145, days: 6, free: false, desc: '重庆出版社典藏版，书脊无折痕，无笔记无折页。看完了给更需要的人。' },
  { id: 'i21', title: '飞利浦 空气炸锅 HD9252 4.1L', price: 159, orig: 499, cond: '9成新', cat: 'appliance', imgs: ['g_airfryer'], ratio: 1.25, seller: 's4', wants: 5, views: 120, days: 10, free: false, desc: '用了一年多，炸篮有轻微划痕，功能正常。换了更大的才出，只限自提。' },
  { id: 'i22', title: '贝恩施 大颗粒积木 120 粒 带收纳桶', price: 45, orig: 129, cond: '9成新', cat: 'baby', imgs: ['g_lego'], ratio: 1.0, seller: 's3', wants: 4, views: 86, days: 3, free: false, desc: '已用消毒湿巾擦过一遍，颗粒齐全，适合 2~6 岁。收纳桶有一道小划痕。' },
  { id: 'i23', title: '李宁 赤兔 6 跑鞋 41 码 白绿配色', price: 189, orig: 399, cond: '95新', cat: 'fashion', imgs: ['g_sneaker'], ratio: 0.8, seller: 's5', wants: 9, views: 175, days: 2, free: true, desc: '跑了大概 50 公里，鞋底颗粒清晰，鞋面干净。鞋码不合适才出。' },
  { id: 'i24', title: '原木学习桌 1 米 带两个抽屉', price: 260, orig: 799, cond: '8成新', cat: 'home', imgs: ['g_desk'], ratio: 1.0, seller: 's4', wants: 6, views: 133, days: 12, free: false, desc: '实木的很沉，桌面有几处小坑，抽屉推拉顺滑。需要自己找货拉拉，楼下有电梯。' },
  { id: 'i25', title: 'Bose QC45 降噪耳机 白色 盒说全', price: 1150, orig: 2299, cond: '95新', cat: 'digital', imgs: ['g_headphone'], ratio: 1.0, seller: 's1', wants: 16, views: 302, days: 7, free: true, desc: '通勤用了半年，耳罩无掉皮，降噪模式切换正常。有京东购买记录可查。' },
  { id: 'i26', title: '捷安特 ATX 山地车 27 速 26 寸', price: 980, orig: 1998, cond: '8成新', cat: 'sport', imgs: ['g_bike'], ratio: 0.75, seller: 's4', wants: 12, views: 247, days: 9, free: false, desc: '前叉避震正常，变速调过了，轮胎去年换的。车架有几处掉漆，不影响骑行。' }
];
const HOT = ['iPhone 13', 'Switch', '考研资料', '戴森吹风机', 'MacBook', '公路车', '乐高', 'Kindle', '降噪耳机', '空气炸锅'];
// 联想词：除了宝贝标题，再补几个常搜的短词
const WORDS = ['iPhone', 'iPhone 13', 'iPhone 12', 'iPad', 'Switch OLED', 'Switch 卡带', '考研数学', '考研英语', '戴森', '戴森吹风机',
  'MacBook Air', 'MacBook Pro', 'ThinkPad', '公路车', '山地车', '乐高', '乐高机械组', 'Kindle', '降噪耳机', '索尼耳机', 'Bose',
  '空气炸锅', '瑜伽垫', '吉他', '跑鞋', 'Nike', '书桌', '婴儿推车', '积木', '三体', '小米手环', '相机', '佳能'];
const SEED_COMMENTS = {
  i01: [{ user: '浙大王同学', avatar: '/images/avatar/a6.jpg', text: '电池健康多少？能不能当面验机', time: '昨天 21:14', reply: '88%，可以来西溪印象城验机' }],
  i02: [{ user: '老陈不折腾', avatar: '/images/avatar/a4.jpg', text: '卡带是实体卡吗？', time: '2 小时前', reply: '是的，实体卡' }, { user: 'Momo', avatar: '/images/avatar/a5.jpg', text: '1500 出吗', time: '1 小时前' }],
  i08: [{ user: '苏苏', avatar: '/images/avatar/a3.jpg', text: '发票能开公司抬头吗', time: '3 天前', reply: '个人抬头的哦' }],
  i17: [{ user: '阿杰的数码铺', avatar: '/images/avatar/a2.jpg', text: '有没有 AppleCare', time: '昨天 10:02', reply: '没有买' }]
};
function imgsOf(it) { return it.imgs.map(n => n.indexOf('/') >= 0 ? n : G + n + '.jpg'); }
module.exports = { CATS, CONDS, SELLERS, ME, ITEMS, HOT, WORDS, SEED_COMMENTS, imgsOf };
