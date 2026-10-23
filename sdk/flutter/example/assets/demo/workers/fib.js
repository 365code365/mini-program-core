// Worker 脚本（app.json 的 "workers": "workers"）：收到 {n} 算 fib(n) 回去
function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }

worker.onMessage(function (msg) {
  const t0 = Date.now();
  const value = fib(msg.n);
  worker.postMessage({ n: msg.n, value: value, ms: Date.now() - t0 });
});
