// 订单进度推送：先连 WebSocket（wx.connectSocket → SocketTask），服务端推 {type:'step', orderId, step}。
// 连不上（onError / onClose / 3 秒内没 onOpen / connectSocket 直接 fail）就降级成本地 setInterval 模拟推送，
// 每 INTERVAL 推进一个节点。所有回调都包在 try 里，不留未捕获错误。
const URL = 'wss://push.jingyidaojia.com/v1/orders/';
const INTERVAL = 6000;
const OPEN_TIMEOUT = 3000;

function connect(orderId, onPush, onMode) {
  let task = null, timer = null, guard = null, opened = false, closed = false;

  const startLocal = reason => {
    if (timer || closed) return;
    if (task && task.readyState <= 1) { try { task.close({ code: 1000, reason: 'fallback' }); } catch (e) {} }
    onMode('local', reason);
    timer = setInterval(() => onPush({ type: 'advance', orderId }), INTERVAL);
  };

  try {
    task = wx.connectSocket({
      url: URL + orderId,
      header: { 'X-Client': 'jingyi-miniprogram' },
      timeout: 5000,
      fail: err => startLocal(err && err.errMsg)
    });
    task.onOpen(() => {
      opened = true;
      clearTimeout(guard);
      onMode('socket');
      task.send({ data: JSON.stringify({ type: 'subscribe', orderId }) });
    });
    task.onMessage(res => {
      let msg = null;
      try { msg = JSON.parse(res.data); } catch (e) { return; }
      if (msg && msg.orderId === orderId) onPush(msg);
    });
    task.onError(err => startLocal((err && err.errMsg) || 'socket error'));
    task.onClose(() => { if (!closed) startLocal('socket closed'); });
  } catch (e) {
    startLocal(e && e.message);
  }
  guard = setTimeout(() => { if (!opened) startLocal('open timeout'); }, OPEN_TIMEOUT);

  return {
    close() {
      closed = true;
      clearTimeout(guard);
      clearInterval(timer);
      timer = null;
      if (task && task.readyState <= 1) { try { task.close({ code: 1000, reason: 'leave page' }); } catch (e) {} }
    }
  };
}

module.exports = { connect, INTERVAL };
