# g-hello

测什么：小游戏运行模式骨架——`game.js` 以模块作用域执行、`require` 包内模块、`console` 输出到宿主、
`wx.onError` 收到定时器里抛出的异常、`wx.onUnhandledRejection` 只收到真正没人处理的拒绝、
`GameGlobal` 就是全局对象、小程序专属的 `Page` / `App` 不存在。

期望（无头 60Hz 跑 10 帧）：日志里依次出现 `hello 1 true …`、`warn-line`、`boom-for-test`（带栈）、
`T:READY g-hello`、`T:EV unhandled {"reason":"nope"}`（只有这一条，没有 handled-late）、
`T:EV onError {"msg":"in-timer"}`、`T:DONE {}`；`T:FAIL` 为 0。用例：`tests/game_e2e.rs` 的 `r1_*`。
