// 精选商城 mock 数据：所有页面共用一份商品表，图片都在 /assets/products/ 下
var IMG = '/assets/products/';

var products = [
  { id: 101, name: '降噪蓝牙耳机 Pro', desc: '主动降噪 · 40小时续航', price: '199', originalPrice: '399', sold: '2.3万', image: IMG + 'earbuds.jpg', hero: IMG + 'earbuds_l.jpg', tag: '限时特惠', cat: 1 },
  { id: 102, name: '智能运动手表', desc: '心率血氧 · 独立GPS', price: '599', originalPrice: '899', sold: '1.8万', image: IMG + 'watch.jpg', hero: IMG + 'watch_l.jpg', tag: '热卖', cat: 1 },
  { id: 103, name: '旗舰影像手机', desc: '一英寸大底 · 120Hz高刷', price: '3999', originalPrice: '4599', sold: '1.2万', image: IMG + 'phone.jpg', hero: IMG + 'phone_l.jpg', tag: '新品', cat: 1 },
  { id: 104, name: '轻薄5G手机', desc: '6.7英寸 · 超长续航', price: '1899', originalPrice: '2299', sold: '8900', image: IMG + 'phone2.jpg', hero: IMG + 'phone2_l.jpg', cat: 1 },
  { id: 105, name: '轻跑缓震运动鞋', desc: '透气网面 · 回弹中底', price: '329', originalPrice: '459', sold: '3.1万', image: IMG + 'sneakers.jpg', hero: IMG + 'sneakers_l.jpg', tag: '爆款', cat: 2 },
  { id: 106, name: '通勤防泼水双肩包', desc: '15.6寸电脑仓 · 大容量', price: '259', originalPrice: '359', sold: '1.6万', image: IMG + 'backpack.jpg', hero: IMG + 'backpack_l.jpg', cat: 8 },
  { id: 107, name: '丝绒雾面口红', desc: '显白正红 · 持久不拔干', price: '129', originalPrice: '199', sold: '4.5万', image: IMG + 'lipstick.jpg', hero: IMG + 'lipstick_l.jpg', tag: '人气', cat: 3 },
  { id: 108, name: '精品挂耳咖啡礼盒', desc: '中深烘焙 · 12包装', price: '89', originalPrice: '129', sold: '5.6万', image: IMG + 'coffee.jpg', hero: IMG + 'coffee_l.jpg', cat: 4 },
  { id: 109, name: '草莓奶油蛋糕', desc: '动物奶油 · 当日现做', price: '168', originalPrice: '218', sold: '9200', image: IMG + 'cake.jpg', hero: IMG + 'cake_l.jpg', tag: '新品', cat: 4 },
  { id: 110, name: '316不锈钢保温杯', desc: '12小时保温 · 500ml', price: '99', originalPrice: '159', sold: '2.7万', image: IMG + 'thermos.jpg', hero: IMG + 'thermos_l.jpg', tag: '特价', cat: 5 }
];

var byId = {};
products.forEach(function(p) { byId[p.id] = p; });

function get(id) {
  return byId[id];
}

function pick(ids) {
  return ids.map(function(id) { return byId[id]; });
}

// 分类页：每个分类下的商品（一个商品可以出现在多个分类）
var categoryProducts = {
  1: [103, 104, 101, 102],
  2: [105, 106],
  3: [107],
  4: [108, 109],
  5: [110, 108],
  6: [110, 109],
  7: [105, 102, 110],
  8: [106, 105]
};

// 购物车 / 订单的默认演示数据（本地存储为空时使用）
function demoCart() {
  return [
    { id: 101, quantity: 1, selected: true },
    { id: 105, quantity: 2, selected: true },
    { id: 108, quantity: 1, selected: false }
  ].map(function(c) {
    var p = byId[c.id];
    return { id: p.id, name: p.name, desc: p.desc, price: p.price, image: p.image, quantity: c.quantity, selected: c.selected };
  });
}

function orderItem(id, qty) {
  var p = byId[id];
  return { id: p.id, name: p.name, desc: p.desc, price: p.price, image: p.image, quantity: qty };
}

function demoOrders() {
  var address = { id: 1, name: '张三', phone: '138****8888', address: '北京市朝阳区建国路88号' };
  return [
    { id: 3, orderNo: 'JX20241022180312', products: [orderItem(102, 1)], total: '599.00', address: address, status: 'pending', statusText: '待付款', createTime: '2024-10-22 18:03' },
    { id: 2, orderNo: 'JX20241018093045', products: [orderItem(107, 2), orderItem(110, 1)], total: '357.00', address: address, status: 'shipped', statusText: '待收货', createTime: '2024-10-18 09:30' },
    { id: 1, orderNo: 'JX20241009201538', products: [orderItem(109, 1)], total: '168.00', address: address, status: 'completed', statusText: '已完成', createTime: '2024-10-09 20:15' }
  ];
}

module.exports = {
  products: products,
  get: get,
  pick: pick,
  categoryProducts: categoryProducts,
  demoCart: demoCart,
  demoOrders: demoOrders
};
