// 案例与设计师（杭州本地家装公司的口径：面积按建筑面积，预算为硬装 + 主材 + 定制 + 软装的全包价，单位万元）
const STYLES = ['全部', '现代简约', '北欧', '新中式', '日式原木', '轻奢', '法式'];

const DESIGNERS = [
  {
    id: 'd1', name: '周景行', title: '首席设计师', years: 12, rating: 4.9, fee: 260, styles: ['现代简约', '轻奢'], avatar: '/images/designers/d1.jpg',
    intro: '中国美术学院环境艺术专业，擅长大平层与改善型住宅，注重收纳动线和自然采光。2019 年 IAI 亚太设计奖住宅空间优秀奖。',
    reviews: [
      { user: '滨江 林女士', star: 5, text: '量房很细，连插座高度都按我们的身高调了，住进来一年没有一处想改的。' },
      { user: '未来科技城 王先生', star: 5, text: '预算控制得好，最后增项不到 3%。' }
    ]
  },
  {
    id: 'd2', name: '沈语桐', title: '资深设计师', years: 8, rating: 4.8, fee: 180, styles: ['北欧', '日式原木'], avatar: '/images/designers/d2.jpg',
    intro: '专注 90~140㎡ 三口之家，喜欢用原木与浅色系营造松弛感，儿童房成长型设计做过 60 多套。',
    reviews: [
      { user: '良渚 陈女士', star: 5, text: '儿童房可以随孩子长大调整，很贴心。' },
      { user: '拱墅 赵先生', star: 4, text: '方案改了三版都很耐心，施工期间也常来工地。' }
    ]
  },
  {
    id: 'd3', name: '陆承泽', title: '主任设计师', years: 15, rating: 4.9, fee: 320, styles: ['新中式'], avatar: '/images/designers/d3.jpg',
    intro: '研究江南园林十余年，把框景、借景用在现代住宅里。做过多套西湖周边别墅与合院。',
    reviews: [
      { user: '之江 顾先生', star: 5, text: '月洞门和茶室的比例拿捏得很好，来家里的朋友都要拍照。' }
    ]
  },
  {
    id: 'd4', name: '许清禾', title: '资深设计师', years: 9, rating: 4.7, fee: 200, styles: ['法式', '轻奢'], avatar: '/images/designers/d4.jpg',
    intro: '法国 ESAG 室内建筑硕士，擅长法式石膏线、护墙板与灯光层次，对软装搭配很有心得。',
    reviews: [
      { user: '钱江新城 吴女士', star: 5, text: '护墙板的颜色调了好几次小样，最后效果比效果图还好。' },
      { user: '城西 何女士', star: 4, text: '软装预算超了一点，但每件都很喜欢。' }
    ]
  },
  {
    id: 'd5', name: '韩亦舟', title: '设计师', years: 5, rating: 4.8, fee: 120, styles: ['现代简约', '北欧'], avatar: '/images/designers/d5.jpg',
    intro: '擅长小户型改造与旧房翻新，60㎡ 两房改三房、厨房改开放式都做过，性价比方案很受刚需业主欢迎。',
    reviews: [
      { user: '下沙 刘先生', star: 5, text: '老破小翻新，十二万做完，邻居都来参观。' }
    ]
  },
  {
    id: 'd6', name: '唐知夏', title: '资深设计师', years: 7, rating: 4.8, fee: 160, styles: ['日式原木', '新中式'], avatar: '/images/designers/d6.jpg',
    intro: '喜欢留白和天然材料，竹、藤、亚麻用得多，做过多套适合老人居住的适老化设计。',
    reviews: [
      { user: '萧山 徐女士', star: 5, text: '给父母做的适老化改造，卫生间扶手和夜灯都考虑到了。' }
    ]
  }
];

// before：装修前（毛坯）照片，用于前后对比
const CASES = [
  { id: 'c1', title: '滨江金茂府 128㎡ 现代简约三居', style: '现代简约', area: 128, layout: '3室2厅2卫', budget: 42, designer: 'd1', cover: '/images/rooms/r1.jpg', gallery: ['/images/rooms/r1.jpg', '/images/rooms/r7.jpg', '/images/rooms/r10.jpg'], before: '/images/rooms/r1-raw.jpg', likes: 2316, h: 300,
    desc: '业主是一家三口加一只猫。客厅取消电视墙，改成整面书柜 + 投影；入户做了 2.4 米高的收纳柜，鞋、婴儿车、快递都有地方放。', split: [32, 30, 22, 10, 6] },
  { id: 'c2', title: '良渚文化村 96㎡ 北欧两居', style: '北欧', area: 96, layout: '2室2厅1卫', budget: 26, designer: 'd2', cover: '/images/rooms/r2.jpg', gallery: ['/images/rooms/r2.jpg', '/images/rooms/r8.jpg', '/images/rooms/r4.jpg'], before: '/images/rooms/r1-raw.jpg', likes: 1688, h: 240,
    desc: '浅色橡木地板配薄荷绿沙发，阳台并入客厅做成猫爬架和绿植角。全屋无主灯，用筒灯和灯带做均匀照明。', split: [30, 28, 20, 14, 8] },
  { id: 'c3', title: '之江九里 210㎡ 新中式四居', style: '新中式', area: 210, layout: '4室2厅3卫', budget: 98, designer: 'd3', cover: '/images/rooms/r3.jpg', gallery: ['/images/rooms/r3.jpg', '/images/rooms/r9.jpg', '/images/rooms/r7.jpg'], before: '/images/rooms/r3-raw.jpg', likes: 3420, h: 320,
    desc: '餐厅与茶室之间开了一道月洞门，形成框景。胡桃木格栅、手工砖与宣纸灯，保留中式意境又不显沉闷。', split: [28, 32, 24, 10, 6] },
  { id: 'c4', title: '城西银泰旁 89㎡ 日式原木两居', style: '日式原木', area: 89, layout: '2室1厅1卫', budget: 22, designer: 'd6', cover: '/images/rooms/r4.jpg', gallery: ['/images/rooms/r4.jpg', '/images/rooms/r2.jpg', '/images/rooms/r8.jpg'], before: '/images/rooms/r1-raw.jpg', likes: 1205, h: 260,
    desc: '次卧做成榻榻米 + 升降桌，平时是书房，来客人就是客房。全屋白蜡木饰面，墙面用硅藻泥。', split: [31, 27, 26, 10, 6] },
  { id: 'c5', title: '钱江新城 165㎡ 轻奢大平层', style: '轻奢', area: 165, layout: '3室2厅2卫', budget: 76, designer: 'd4', cover: '/images/rooms/r5.jpg', gallery: ['/images/rooms/r5.jpg', '/images/rooms/r10.jpg', '/images/rooms/r7.jpg'], before: '/images/rooms/r6-raw.jpg', likes: 2890, h: 280,
    desc: '岩板电视墙配黄铜收口，深蓝丝绒沙发是全屋的视觉重心。主卧套房带独立衣帽间和双台盆。', split: [26, 34, 22, 12, 6] },
  { id: 'c6', title: '奥体印 138㎡ 法式三居', style: '法式', area: 138, layout: '3室2厅2卫', budget: 55, designer: 'd4', cover: '/images/rooms/r6.jpg', gallery: ['/images/rooms/r6.jpg', '/images/rooms/r7.jpg', '/images/rooms/r9.jpg'], before: '/images/rooms/r6-raw.jpg', likes: 2045, h: 310,
    desc: '客厅拱形窗套与护墙板呼应，奶咖色墙漆配人字拼地板。餐边柜内嵌咖啡机和小冰箱。', split: [30, 30, 22, 12, 6] },
  { id: 'c7', title: '下沙 62㎡ 老破小两房改三房', style: '现代简约', area: 62, layout: '3室1厅1卫', budget: 13, designer: 'd5', cover: '/images/rooms/r7.jpg', gallery: ['/images/rooms/r7.jpg', '/images/rooms/r1.jpg', '/images/rooms/r2.jpg'], before: '/images/rooms/r3-raw.jpg', likes: 4102, h: 250,
    desc: '把原来 18㎡ 的大卧室一分为二，做成儿童房 + 小书房；厨房外移做成开放式，餐厅借用过道。', split: [36, 28, 22, 8, 6] },
  { id: 'c8', title: '未来科技城 118㎡ 北欧三居', style: '北欧', area: 118, layout: '3室2厅1卫', budget: 33, designer: 'd2', cover: '/images/rooms/r8.jpg', gallery: ['/images/rooms/r8.jpg', '/images/rooms/r2.jpg', '/images/rooms/r4.jpg'], before: '/images/rooms/r1-raw.jpg', likes: 1532, h: 290,
    desc: '主卧墨绿色背景墙配原木床，客厅做了一整面洞洞板展示墙。卫生间三分离，干区放洗衣机烘干机叠放。', split: [31, 29, 21, 12, 7] },
  { id: 'c9', title: '西溪湿地 180㎡ 新中式合院', style: '新中式', area: 180, layout: '4室2厅2卫', budget: 82, designer: 'd6', cover: '/images/rooms/r9.jpg', gallery: ['/images/rooms/r9.jpg', '/images/rooms/r3.jpg', '/images/rooms/r4.jpg'], before: '/images/rooms/r3-raw.jpg', likes: 2610, h: 270,
    desc: '一层做成可以看到庭院的茶餐厅，圆窗对着一株红枫。卧室全部用木作吊顶，冬暖夏凉。', split: [27, 31, 26, 10, 6] },
  { id: 'c10', title: '湖滨 146㎡ 轻奢改善三居', style: '轻奢', area: 146, layout: '3室2厅2卫', budget: 61, designer: 'd1', cover: '/images/rooms/r10.jpg', gallery: ['/images/rooms/r10.jpg', '/images/rooms/r5.jpg', '/images/rooms/r1.jpg'], before: '/images/rooms/r6-raw.jpg', likes: 1977, h: 300,
    desc: '餐厅一侧做了酒柜和中岛，灰色木饰面配香槟金灯具，整体克制不张扬。', split: [28, 32, 24, 10, 6] }
];

const SPLIT_NAMES = ['人工辅材', '主材', '全屋定制', '软装家具', '家电'];

function designer(id) { return DESIGNERS.find((d) => d.id === id); }
function caseById(id) { return CASES.find((c) => c.id === id); }

module.exports = { STYLES, DESIGNERS, CASES, SPLIT_NAMES, designer, caseById };
