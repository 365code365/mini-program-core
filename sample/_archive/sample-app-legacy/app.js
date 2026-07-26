// 商城小程序
App({
  globalData: {
    userInfo: null,
    cart: [],
    cartCount: 0
  },
  
  onLaunch: function() {
    console.log('🛒 Mini Shop 启动');
  },
  
  // 添加商品到购物车
  addToCart: function(product, quantity) {
    console.log('🛒 addToCart 收到参数:', typeof product, JSON.stringify(product));
    quantity = quantity || 1;
    var cart = this.globalData.cart;
    var found = false;
    
    // 确保 product 是对象且有 id
    if (!product || typeof product !== 'object') {
      console.log('❌ product 不是对象:', typeof product, product);
      return;
    }
    
    if (product.id === undefined) {
      console.log('❌ product.id 不存在, product keys:', Object.keys(product));
      return;
    }
    
    var productId = product.id;
    console.log('🔍 商品ID:', productId, '名称:', product.name, '价格:', product.price);
    
    for (var i = 0; i < cart.length; i++) {
      console.log('  对比购物车[' + i + '] id=' + cart[i].id + ' vs ' + productId);
      if (cart[i].id === productId) {
        cart[i].quantity += quantity;
        found = true;
        console.log('✅ 找到已有商品，数量+1');
        break;
      }
    }
    
    if (!found) {
      cart.push({
        id: productId,
        name: product.name,
        price: product.price,
        image: product.image,
        quantity: quantity
      });
      console.log('➕ 新增商品到购物车');
    }
    
    this.updateCartCount();
    console.log('🛒 购物车现有', cart.length, '种商品');
  },
  
  // 更新购物车数量
  updateCartCount: function() {
    var count = 0;
    this.globalData.cart.forEach(function(item) {
      count += item.quantity;
    });
    this.globalData.cartCount = count;
  },
  
  // 获取购物车总价
  getCartTotal: function() {
    var total = 0;
    this.globalData.cart.forEach(function(item) {
      total += item.price * item.quantity;
    });
    return total.toFixed(2);
  }
});
