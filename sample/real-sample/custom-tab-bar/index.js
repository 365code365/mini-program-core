
Component({
  data: {
    selected: 0,
    cartCount: 0
  },

  lifetimes: {
    attached: function() {
      this.updateSelected();
      this.updateCartCount();
    }
  },

  methods: {
    updateSelected: function() {
      const pages = getCurrentPages();
      const currentPage = pages[pages.length - 1];
      const route = currentPage.route;

      let selected = 0;
      if (route.includes('index')) selected = 0;
      else if (route.includes('category')) selected = 1;
      else if (route.includes('cart')) selected = 2;
      else if (route.includes('profile')) selected = 3;

      this.setData({ selected });
    },

    updateCartCount: function() {
      // getApp() 用时再取：组件脚本可能在 App() 之前就被加载
      const app = getApp();
      this.setData({ cartCount: app ? app.globalData.cartCount : 0 });
    },

    onSwitchTab: function(e) {
      const index = e.currentTarget.dataset.index;
      const path = e.currentTarget.dataset.path;

      if (this.data.selected !== index) {
        wx.switchTab({ url: path });
      }
    }
  }
});
