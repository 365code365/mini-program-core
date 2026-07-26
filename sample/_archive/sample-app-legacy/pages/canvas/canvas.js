Page({
  data: {},

  onLoad: function() {
    console.log('Canvas page onLoad');
    this.drawBasicShapes();
    this.drawGradients();
    this.drawChart();
  },

  // 绘制基础图形
  drawBasicShapes: function() {
    console.log('Drawing basic shapes...');
    var ctx = wx.createCanvasContext('canvas1');
    
    // 填充矩形 - 橙色
    ctx.setFillStyle('#FF6B35');
    ctx.fillRect(20, 20, 80, 60);
    
    // 描边矩形 - 绿色
    ctx.setStrokeStyle('#07c160');
    ctx.setLineWidth(3);
    ctx.strokeRect(120, 20, 80, 60);
    
    // 填充圆形 - 蓝色
    ctx.beginPath();
    ctx.setFillStyle('#1890ff');
    ctx.arc(260, 50, 30, 0, 2 * Math.PI);
    ctx.fill();
    
    // 绘制线条 - 紫色
    ctx.beginPath();
    ctx.setStrokeStyle('#722ed1');
    ctx.setLineWidth(2);
    ctx.moveTo(20, 110);
    ctx.lineTo(100, 130);
    ctx.lineTo(180, 100);
    ctx.lineTo(280, 130);
    ctx.stroke();
    
    ctx.draw();
    console.log('Basic shapes drawn');
  },

  // 绘制渐变（简化版 - 用纯色代替）
  drawGradients: function() {
    console.log('Drawing gradients...');
    var ctx = wx.createCanvasContext('canvas2');
    
    // 用红色矩形代替线性渐变
    ctx.setFillStyle('#ff6666');
    ctx.fillRect(20, 20, 120, 80);
    
    // 用蓝色矩形代替径向渐变
    ctx.setFillStyle('#6699ff');
    ctx.fillRect(160, 20, 120, 80);
    
    ctx.draw();
    console.log('Gradients drawn');
  },

  // 绘制柱状图
  drawChart: function() {
    console.log('Drawing chart...');
    var ctx = wx.createCanvasContext('canvas3');
    var data = [65, 85, 45, 95, 70, 55, 80];
    var colors = ['#FF6B35', '#07c160', '#1890ff', '#722ed1', '#eb2f96', '#faad14', '#13c2c2'];
    
    var chartHeight = 140;
    var barWidth = 30;
    var gap = 10;
    var startX = 20;
    var startY = 150;
    
    // 绘制坐标轴
    ctx.setStrokeStyle('#dddddd');
    ctx.setLineWidth(1);
    ctx.beginPath();
    ctx.moveTo(startX, startY - chartHeight);
    ctx.lineTo(startX, startY);
    ctx.lineTo(startX + 280, startY);
    ctx.stroke();
    
    // 绘制柱状图
    for (var i = 0; i < data.length; i++) {
      var barHeight = (data[i] / 100) * chartHeight;
      var x = startX + i * (barWidth + gap) + gap;
      var y = startY - barHeight;
      
      ctx.setFillStyle(colors[i]);
      ctx.fillRect(x, y, barWidth, barHeight);
    }
    
    ctx.draw();
    console.log('Chart drawn');
  },

  // 重新绘制
  redraw: function() {
    this.drawBasicShapes();
    this.drawGradients();
    this.drawChart();
  },

  // 清除全部
  clearAll: function() {
    var canvasIds = ['canvas1', 'canvas2', 'canvas3'];
    for (var i = 0; i < canvasIds.length; i++) {
      var ctx = wx.createCanvasContext(canvasIds[i]);
      ctx.clearRect(0, 0, 320, 200);
      ctx.draw();
    }
  }
});
