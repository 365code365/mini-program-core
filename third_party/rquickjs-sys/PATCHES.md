# 仓库里这份 rquickjs-sys 改了什么

基于 crates.io 上的 `rquickjs-sys 0.12.1`（内置 quickjs-ng 0.15.1）。`Cargo.toml` 的 `[patch.crates-io]` 指到这里，
所以桌面、Android、iOS、macOS、鸿蒙编出来的都是这一份。与原版的完整差异在 `mini-render.patch`
（`diff -ruN 原版 本目录`），升级 rquickjs 时先把它在新版上重放一遍，再跑下面「怎么验」。

改动都只为一件事：小游戏（Cocos 等引擎）在纯解释器上跑得更快。没有 JIT，不生成机器码。
语义与原版逐条相同。新加的缓存只在「能证明结果不变」时才用；证明不了，就走原来那条路。

## 1. 属性读写的内联缓存（`OP_get_field_ic` / `OP_get_field2_ic` / `OP_put_field_ic`）

- 函数第一次被调用时（`JS_CallInternal` 序言里的 `js_bytecode_init_ic`），把字节码里的 `get_field` / `get_field2` / `put_field`
  原地改写成带缓存的三条新指令。指令长度不变，操作数从原子换成缓存槽号，原子挪进槽里。
  生成器、async 函数的函数体不经这里，保持原指令。
- **只在内存里**：写字节码（`JS_WriteFunctionBytecode`，字节码缓存用）时换回原指令和原子，序列化格式与原版逐字节相同；
  读字节码时如果遇到这三条指令，按坏数据拒收（否则槽号会越界）。
- 自有属性：槽里记「下标 + 接收者形状的身份号」。身份号对得上就直接读 `p->prop[idx]`；
  对不上，再校验这个下标上是不是同一个原子（形状不同但布局一样的对象照样命中），还不行才查哈希表。
- 原型链上的属性（方法、getter）：槽里记「接收者形状身份号 + 纪元 → 持有者 + 下标」，有两路，同一处常有两三种接收者。
  - **形状身份号** `JSShape.uid` 放在原来的填充位里。新建、克隆、搬家、压缩、原地加属性、原地换原型、
    `js_shape_prepare_update`（原地改标志 / 删属性之前）都会换一个新号。所以内存被别的形状重用、原地多了属性，号都对不上。
  - **纪元** `rt->ic_epoch`：原型对象的结构一变就加一，包括加属性、删属性、改类型、换原型、换 getter / setter 函数。
    纪元一变，所有原型链缓存项一起失效。凡是当过形状原型的对象，在 `js_new_shape2` 里都会打上 `is_prototype`。
    原版只在 `setPrototypeOf` 时才打这个标。
- 数组与类型化数组按具名属性查时，当普通对象看：数组只在整数下标上特殊，类型化数组只在规范数字串上特殊。
  所以 `arr.push`、`f32.subarray` 也能命中缓存。
- **只读 this 上属性的 getter 不调用**：Cocos、TypeScript 编出来的引擎里有成千上万个
  `get x() { return this._x; }` / `return this.a.b`（farm001 里，原型链缓存命中的 getter 调用有 2/3 是这种）。
  认出这种字节码后，缓存里顺带记下那个属性在接收者形状里的下标，命中时直接读。
  只有读到的是自有的普通数据属性时才这样做，否则照常调用 getter，所以异常栈、副作用都和原来完全一样。
- `put_field` 加新属性（构造函数里的 `this.x = …`）：缓存「这个接收者形状加上当前纪元下，整条原型链上都没有这个属性」。
  命中时直接 `add_property`，不再逐层查原型链上有没有 setter 或只读属性。
- `JS_SetInlineCacheEnabled(rt, false)` 关掉缓存，只影响之后第一次被调用的函数。引擎里对应的对照开关是 `MINI_JS_NO_IC=1`。

## 2. 小块池（`js_pool_alloc`）

≤ 512 字节的分配按 16 字节分 32 档，每档一条侵入式空闲链表，挂在 `JSRuntime` 上。每档最多留 4096 块，多的还给宿主；
`JS_FreeRuntime` 时全部还掉。池开着时，每一块前面都有 16 字节的头，记着可用大小，记账不必再调 `malloc_usable_size`。
它替掉了原来 Rust 侧的分档池（rquickjs 的 `Allocator`）：那一层每次分配还要穿过 rquickjs 的包装与 dyn 分发。
`JS_SetSmallBlockPoolDefault(false)` 设进程默认值，引擎里的开关是 `MINI_JS_SYSTEM_ALLOC=1`。
这个值只能在建第一个运行时之前设，因为头是每一块都带的。

## 3. 数组 / 类型化数组的快路径

- `get_array_el` / `get_array_el2`：数组和 `Float32Array` 就地取，其余类型化数组走 `js_get_fast_array_element`；
  普通对象不再白试一次。值为整数的浮点下标（`x * 4 + 1` 这类）也当整数下标用。
- `put_array_el`：`Float32Array` / `Float64Array` 写数字、整数类型化数组写整数时就地写，按位截断即 ToInt8 / ToUint16 等。
  `Uint8ClampedArray`、BigInt 数组照旧走原路径。进程里出现过不可变 ArrayBuffer（`js_ab_immutable_seen`）后，
  写入一律退回原路径，因为原路径要多追三层指针才能确认 buffer 可写。
- `OP_get_length` 读类型化数组的 `length`：按类缓存「验证过的接收者形状」。条件是：对象上没有自有 `length`，
  原型链上找到的是内置的 `js_typed_array_get_length`。纪元一变就重验。原来每次都要先判一遍数字下标，再调 C getter。

## 4. 构建

`build.rs` 加了 `rerun-if-changed=quickjs`。原版只盯环境变量，改了 C 源码 cargo 不会重编。

## 怎么验（改这里的任何东西都要全过）

- **test262 全量 + quickjs 自带 tests/**：用这份源码编 `run-test262`，跑 `-c test262.conf -a`。
  失败清单必须与原版逐条相同。2026-10-29 实测：81122 条用例，两边都是同样的 153 条失败；tests/ 两边都是 0 失败。
- **内联缓存的对抗用例**：遮蔽、删属性、换原型、改 getter、冻结、Proxy、类型化数组的 length 与数字串、整数截断、
  浮点下标、多态、只读 getter 内联、加新属性后原型上出现 setter 等。原版与这份逐行输出相同。
- 引擎回归：`bash tools/regress-async.sh`、`cargo test --release --test game_e2e`。
