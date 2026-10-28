;; g-wasm 的基准：递归 fib 与内存里的整数排序（插入排序，n²）。源文件留着，bench.wasm 由 examples 现场编（见 game.js 头）
(module
  (memory (export "mem") 1)
  (func $fib (export "fib") (param i32) (result i32)
    local.get 0 i32.const 2 i32.lt_s
    if (result i32) local.get 0
    else local.get 0 i32.const 1 i32.sub call $fib local.get 0 i32.const 2 i32.sub call $fib i32.add end)
  (func (export "sort") (param $n i32) (local $i i32) (local $j i32) (local $v i32)
    i32.const 1 local.set $i
    block $done loop $outer
      local.get $i local.get $n i32.ge_s br_if $done
      local.get $i i32.const 2 i32.shl i32.load local.set $v
      local.get $i i32.const 1 i32.sub local.set $j
      block $b loop $inner
        local.get $j i32.const 0 i32.lt_s br_if $b
        local.get $j i32.const 2 i32.shl i32.load local.get $v i32.le_s br_if $b
        local.get $j i32.const 1 i32.add i32.const 2 i32.shl local.get $j i32.const 2 i32.shl i32.load i32.store
        local.get $j i32.const 1 i32.sub local.set $j
        br $inner end end
      local.get $j i32.const 1 i32.add i32.const 2 i32.shl local.get $v i32.store
      local.get $i i32.const 1 i32.add local.set $i
      br $outer end end))
