// QuickJS 改动（third_party/rquickjs-sys/PATCHES.md）的对抗用例：每一条先在热循环里把缓存填上，再改结构，看读到的是不是新的。
// 期望输出 qjs_ic_cases.out 是原版 quickjs-ng 0.15.1 跑出来的（逐行）；js/qjs_runtime.rs 的单测拿它比
var out = [];
function log() { out.push(Array.prototype.join.call(arguments, ' ')); }
function get(o) { return o.x; }
function call(o) { return o.m(); }
function put(o, v) { o.x = v; }
function rep(n, f) { var r; for (var i = 0; i < n; i++) r = f(i); return r; }

// 1 原型方法被自有属性遮住
function A() {} A.prototype.m = function() { return 'proto'; };
var a = new A();
rep(100, function() { return call(a); });
a.m = function() { return 'own'; };
log('1', call(a), call(new A()));
// 2 原型链中间一层后来加了同名属性
function B() {} B.prototype = Object.create(A.prototype);
var b = new B();
rep(100, function() { return call(b); });
B.prototype.m = function() { return 'mid'; };
log('2', call(b));
delete B.prototype.m;
log('2b', call(b));
// 3 删掉持有者上的属性
delete A.prototype.m;
try { call(new A()); log('3', 'no throw'); } catch (e) { log('3', e instanceof TypeError); }
A.prototype.m = function() { return 'again'; };
log('3b', call(new A()), call(b));
// 4 setPrototypeOf 换原型
var c = {}; Object.setPrototypeOf(c, { m: function() { return 'p1'; } });
rep(100, function() { return call(c); });
Object.setPrototypeOf(c, { m: function() { return 'p2'; } });
log('4', call(c));
// 5 数据属性改成 getter、getter 换掉、改回数据属性
var P = { x: 1 }; var d = Object.create(P);
rep(100, function() { return get(d); });
Object.defineProperty(P, 'x', { get: function() { return 'g1:' + (this === d); }, configurable: true });
log('5', get(d));
Object.defineProperty(P, 'x', { get: function() { return 'g2'; }, configurable: true });
log('5b', get(d));
Object.defineProperty(P, 'x', { value: 'v', writable: true, configurable: true });
log('5c', get(d));
// 6 自有属性的下标提示跨形状（布局不同的对象混着来）
var objs = [{ x: 1 }, { y: 2, x: 3 }, { z: 0, y: 0, x: 5 }, Object.create({ x: 7 }), { x: undefined }];
var s6 = []; for (var k = 0; k < 50; k++) s6.push(get(objs[k % 5])); log('6', s6.slice(0, 10).join(','));
// 7 删掉自有属性再读（应走原型）
var e7 = Object.create({ x: 'p' }); e7.x = 'o';
rep(100, function() { return get(e7); });
delete e7.x; log('7', get(e7));
e7.x = 'o2'; log('7b', get(e7));
// 8 冻结 / 不可写之后写
var f8 = { x: 1 }; rep(100, function(i) { put(f8, i); }); Object.freeze(f8);
(function() { 'use strict'; try { f8.x = 5; log('8', 'no throw'); } catch (e) { log('8', e instanceof TypeError); } })();
put(f8, 9); log('8b', f8.x);
var w8 = { x: 1 }; rep(100, function(i) { put(w8, i); }); Object.defineProperty(w8, 'x', { writable: false });
put(w8, 123); log('8c', w8.x);
// 9 setter 在原型上，写入走 setter
var S = { set x(v) { this._x = 'set' + v; } }; var s9 = Object.create(S);
rep(10, function(i) { put(s9, i); }); log('9', s9._x, Object.prototype.hasOwnProperty.call(s9, 'x'));
// 10 大量临时对象（形状反复建、放，内存被重用）
function V(x, y) { this.x = x; this.y = y; } V.prototype.len = function() { return this.x + this.y; };
var tot = 0; for (var i = 0; i < 20000; i++) { var v = new V(i, 1); tot += v.len(); if (i % 1000 == 0) V.prototype.len2 = i; }
log('10', tot);
// 11 Proxy 与数组在原型链上
var px = new Proxy({}, { get: function(t, k) { return k === 'x' ? 'proxy' : undefined; } });
var o11 = Object.create(px); rep(50, function() { return get(o11); }); log('11', get(o11));
var arrp = Object.create([1, 2, 3]); log('11b', arrp.length, arrp[1]);
// 12 数组与类型化数组的下标快路径：越界、空洞、负数、浮点下标、分离的 buffer
var arr = [1, 2, 3]; var s12 = 0; for (var i = -1; i < 5; i++) s12 += (arr[i] === undefined ? 100 : arr[i]); log('12', s12);
var hole = [1, , 3]; Array.prototype[1] = 'protoHole'; log('12b', hole[1]); delete Array.prototype[1];
var fa = new Float32Array([1.5, -0, NaN, 3]); log('12c', fa[0], 1 / fa[1], fa[2], fa[3], fa[4], fa[1.5], fa[-1]);
fa[0] = 2.75; fa[1] = 1e40; fa[2] = -0; fa[3] = '7'; fa[9] = 1; log('12d', fa[0], fa[1], 1 / fa[2], fa[3], fa.length, fa[9]);
var fd = new Float64Array(2); fd[0] = 0.1; fd[1] = {valueOf: function() { return 42; }}; log('12e', fd[0], fd[1]);
var buf = new ArrayBuffer(16); var f12 = new Float32Array(buf); for (var i = 0; i < 4; i++) f12[i] = i;
var r12 = 0; for (var k = 0; k < 100; k++) r12 += f12[k & 3];
if (buf.transfer) { buf.transfer(); log('12f', r12, f12[0], f12.length); f12[0] = 5; log('12g', f12[0]); } else log('12f', r12);
var i32 = new Int32Array([5, 6]); var u8 = new Uint8Array([255]); log('12h', i32[1], u8[0], i32[5]);
var args = (function() { return arguments; })(1, 2); log('12i', args[0], args[1], args[2]);
// 13 对象当原型、自己又在加属性（字典式用法）
var base13 = {}; var kids = []; for (var i = 0; i < 10; i++) kids.push(Object.create(base13));
for (var i = 0; i < 200; i++) { base13['k' + (i % 20)] = i; }
var s13 = 0; for (var i = 0; i < 10; i++) s13 += kids[i].k3 + kids[i].k19; log('13', s13);
// 14 类与 super、静态、私有
class C1 { m() { return 'c1'; } get g() { return 'g1'; } }
class C2 extends C1 { m() { return 'c2>' + super.m(); } }
var c2 = new C2(); rep(100, function() { return c2.m() + c2.g; }); log('14', c2.m(), c2.g);
C1.prototype.m = function() { return 'c1new'; }; log('14b', c2.m());
// 15 全局变量 / let 遮住全局
globalThis.gv = 1; function readG() { return gv; } rep(100, readG); globalThis.gv = 2; log('15', readG());
// 16 字符串与原始值上的属性
var str = 'abc'; log('16', str.length, str.charAt(1), (5).toFixed(1), true.toString());
String.prototype.zz = 'sz'; log('16b', str.zz); delete String.prototype.zz; log('16c', str.zz);
// 17 Symbol.toPrimitive 与 getter 抛异常
var thrower = Object.create({ get x() { throw new Error('boom'); } });
try { rep(3, function() { return get(thrower); }); } catch (e) { log('17', e.message); }
// 18 原型属性被改成不可配置的访问器、再变回去
var P18 = { m: function() { return 1; } }; var o18 = Object.create(P18); rep(100, function() { return call(o18); });
P18.m = function() { return 2; }; log('18', call(o18));
// 19 with / eval 改变量
var o19 = { x: 'with' }; with (o19) { log('19', x); }
// 20 很多不同形状经同一处读（超态）
var many = []; for (var i = 0; i < 64; i++) { var o = {}; o['p' + i] = i; o.x = i; many.push(o); }
var s20 = 0; for (var r = 0; r < 10; r++) for (var i = 0; i < 64; i++) s20 += get(many[i]); log('20', s20);

// 21 数组上调方法（数组接收者走缓存），之后改 Array.prototype / 实例上自有同名
var ar21 = [1, 2, 3]; function pushIt(a, v) { return a.push(v); }
for (var i = 0; i < 100; i++) pushIt(ar21, i);
var oldPush = Array.prototype.push; Array.prototype.push = function(v) { return 'patched'; };
log('21', pushIt(ar21, 1), pushIt([], 2));
Array.prototype.push = oldPush; ar21.push = function() { return 'own'; }; log('21b', pushIt(ar21, 1), pushIt([5], 1));
function idx0(a) { return a.indexOf(3); } log('21c', idx0([1, 3]), idx0(new Float32Array([0, 0, 3])), idx0('x3'));
// 22 类型化数组 length：自有 length、换掉原型上的 getter
var t22 = new Float32Array(5); function len(a) { return a.length; } var s22 = 0; for (var i = 0; i < 100; i++) s22 += len(t22);
log('22', s22, len(new Uint16Array(3)), len([1, 2]), len('abcd'), len({ length: 'o' }));
Object.defineProperty(t22, 'length', { value: 99 }); log('22b', len(t22), len(new Float32Array(2)));
var TA = Object.getPrototypeOf(Float32Array.prototype); var d22 = Object.getOwnPropertyDescriptor(TA, 'length');
Object.defineProperty(TA, 'length', { get: function() { return 'hijack'; }, configurable: true });
log('22c', len(new Float32Array(2)), len(new Int8Array(1)));
Object.defineProperty(TA, 'length', d22); log('22d', len(new Float32Array(2)));
Object.defineProperty(Float32Array.prototype, 'length', { value: 'proto', configurable: true }); log('22e', len(new Float32Array(2)), len(new Int8Array(1)));
delete Float32Array.prototype.length; log('22f', len(new Float32Array(2)));
var buf22 = new ArrayBuffer(8); var f22 = new Float32Array(buf22); log('22g', len(f22)); if (buf22.transfer) { buf22.transfer(); log('22h', len(f22)); }
// 23 类型化数组上的具名属性与数字串
var f23 = new Float32Array(2); Float32Array.prototype.Infinity = 'protoInf'; Float32Array.prototype.foo = 'protoFoo';
function rdInf(o) { return o.Infinity; } function rdFoo(o) { return o.foo; }
for (var i = 0; i < 50; i++) { rdInf(f23); rdFoo(f23); }
log('23', rdInf(f23), rdFoo(f23), rdInf(Object.create(Float32Array.prototype)));
f23.foo = 'own'; log('23b', rdFoo(f23), rdFoo(new Float32Array(1)));
// 24 整数类型化数组写入的截断
var u8 = new Uint8Array(2), i8 = new Int8Array(2), u16 = new Uint16Array(2), i16 = new Int16Array(1), i32a = new Int32Array(1), u32 = new Uint32Array(1), u8c = new Uint8ClampedArray(2);
for (var i = 0; i < 3; i++) { u8[0] = 300; u8[1] = -1; i8[0] = 200; i8[1] = -129; u16[0] = 70000; u16[1] = -2; i16[0] = 40000; i32a[0] = -5; u32[0] = -1; u8c[0] = 300; u8c[1] = -5; }
log('24', u8[0], u8[1], i8[0], i8[1], u16[0], u16[1], i16[0], i32a[0], u32[0], u8c[0], u8c[1]);
u8[5] = 1; i8[0] = 1.7; u16[0] = '12'; log('24b', u8[5], i8[0], u16[0]);
// 25 浮点下标
var a25 = [10, 20, 30]; var f25 = new Float32Array([1, 2, 3]); var h = 0.5;
log('25', a25[1.0], a25[h * 4], a25[2.5], a25[-0], a25[NaN], a25[1e10], f25[h * 2], f25[2.5], f25[-0]);
var o25 = { 1: 'one', '1.5': 'onehalf' }; log('25b', o25[1.0], o25[1.5], o25[h * 2]);
var s25 = 'xyz'; log('25c', s25[h * 2], s25[1.5]);
// 26 多态：同一处读三种接收者
function P1() { this.a = 1; } P1.prototype.who = function() { return 'p1'; };
function P2() { this.b = 1; } P2.prototype.who = function() { return 'p2'; };
function P3() { this.c = 1; } P3.prototype.who = function() { return 'p3'; };
var rs = [new P1(), new P2(), new P3()], w26 = [];
for (var i = 0; i < 30; i++) w26.push(rs[i % 3].who()); log('26', w26.slice(0, 6).join(','));
P2.prototype.who = function() { return 'p2x'; }; log('26b', rs[0].who(), rs[1].who(), rs[2].who());

// 27 类型化数组子类 / 换原型之后的 length
class MyF extends Float32Array { get length() { return 'sub'; } }
var mf = new MyF(3), pf = new Float32Array(4); for (var i = 0; i < 50; i++) { len(mf); len(pf); }
log('27', len(mf), len(pf)); Object.setPrototypeOf(pf, { get length() { return 'swapped'; } }); log('27b', len(pf), len(new Float32Array(1)));
class MyG extends Float32Array {} var mg = new MyG(6); for (var i = 0; i < 50; i++) len(mg); log('27c', len(mg));
MyG.prototype.extra = 1; Object.defineProperty(MyG.prototype, 'length', { get: function() { return 'late'; }, configurable: true }); log('27d', len(mg), len(new Float32Array(2)));

// 28 只读 this 属性的 getter（内联掉调用）：箭头函数 getter、链式 getter、缺属性、非严格、this 是原始值
var self28 = { y: 'lexical' };
var G = (function() { var that = this; return { get a() { return this._a; }, get b() { return this.a; }, get c() { return this.none; } }; })();
var o28 = Object.create(G); o28._a = 'A';
Object.defineProperty(G, 'arrow', { get: (function() { return () => this === undefined ? 'undef' : (this && this.y); }).call(self28) });
function rd(o, k) { return o[k]; } function ra(o) { return o.a; } function rb(o) { return o.b; } function rc(o) { return o.c; } function rarrow(o) { return o.arrow; }
for (var i = 0; i < 50; i++) { ra(o28); rb(o28); rc(o28); rarrow(o28); }
log('28', ra(o28), rb(o28), rc(o28), rarrow(o28));
o28._a = 'A2'; delete o28._a; log('28b', ra(o28)); G._a = 'fromG'; log('28c', ra(o28));
Object.defineProperty(o28, '_a', { get: function() { return 'own getter'; }, configurable: true }); log('28d', ra(o28));
Object.defineProperty(Number.prototype, 'nv', { get: function() { return typeof this; }, configurable: true });
function rnv(x) { return x.nv; } for (var i = 0; i < 20; i++) rnv(5); log('28e', rnv(5), rnv(Object(5)));
class K { constructor() { this._v = 7; } get v() { return this._v; } } class K2 extends K { get v() { return this._v + 1; } }
function rv(o) { return o.v; } var ks = [new K(), new K2()]; var s28 = []; for (var i = 0; i < 20; i++) s28.push(rv(ks[i & 1])); log('28f', s28.slice(0, 4).join(','));
var px28 = new Proxy({}, { get: function(t, k) { return 'px:' + String(k); } }); var g28 = Object.create(px28);
var G2 = { get w() { return this.q; } }; Object.setPrototypeOf(G2, px28); var o28b = Object.create(G2); for (var i = 0; i < 20; i++) rd(o28b, 'w'); log('28g', rd(o28b, 'w'));
function strictG() { 'use strict'; return this; } var G3 = {}; Object.defineProperty(G3, 'me', { get: function() { return this.tag; } }); var o28c = Object.create(G3); o28c.tag = 't'; log('28h', o28c.me);

// 29 加新属性的缓存：之后原型上出现 setter / 只读属性 / 接收者不可扩展 / 原型链上有 Proxy
function Mk() {} function mk(v) { var o = new Mk(); o.np = v; return o; }
for (var i = 0; i < 50; i++) mk(i);
log('29', mk(1).np, Object.keys(mk(2)).join());
Object.defineProperty(Mk.prototype, 'np', { set: function(v) { this._np = 'set' + v; }, configurable: true });
var m29 = mk(3); log('29b', m29.np, m29._np, Object.prototype.hasOwnProperty.call(m29, 'np'));
delete Mk.prototype.np; for (var i = 0; i < 20; i++) mk(i);
Object.defineProperty(Mk.prototype, 'np', { value: 'ro', writable: false, configurable: true });
var m29c = mk(4); log('29c', m29c.np, Object.prototype.hasOwnProperty.call(m29c, 'np'));
(function() { 'use strict'; try { var o = new Mk(); o.np = 1; log('29d', 'no throw'); } catch (e) { log('29d', e instanceof TypeError); } })();
delete Mk.prototype.np; for (var i = 0; i < 20; i++) mk(i);
var ne = new Mk(); Object.preventExtensions(ne); ne.np = 5; log('29e', ne.np, Object.keys(ne).length);
(function() { 'use strict'; var ne2 = new Mk(); Object.preventExtensions(ne2); try { ne2.np = 1; log('29f', 'no throw'); } catch (e) { log('29f', e instanceof TypeError); } })();
var trap = []; var pxp = new Proxy({}, { set: function(t, k, v, r) { trap.push(k); return Reflect.set(t, k, v, r); } });
function Pq() {} Pq.prototype = Object.create(pxp); function mq(v) { var o = new Pq(); o.np = v; return o; }
for (var i = 0; i < 10; i++) mq(i); log('29g', trap.length, mq(7).np);
Object.prototype.np2 = 'op'; function mk2() { var o = {}; o.np2 = 'own'; return o; } for (var i = 0; i < 20; i++) mk2();
Object.defineProperty(Object.prototype, 'np2', { set: function(v) { this.__s = v; }, configurable: true }); var r29 = mk2(); log('29h', r29.__s, Object.prototype.hasOwnProperty.call(r29, 'np2'));
delete Object.prototype.np2;
class Fld { a = 1; b = 2; } for (var i = 0; i < 20; i++) new Fld(); log('29i', JSON.stringify(new Fld()));
out.join('\n');
