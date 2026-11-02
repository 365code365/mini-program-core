;; play-tower 的战斗计算（WXWebAssembly）。combat.wasm 由这份源码逐条手编成字节码（没有引入 wat 编译器），
;; 两者一一对应；改了这里要同步重编 combat.wasm（js/combat.js 开局会拿同一组输入与 JS 版对一遍，不一致就退回 JS 版）。
;;
;; 内存布局：0 起是候选敌人，每个 16 字节 [x f32, y f32, 已走路程 f32, 保留]，最多 512 个；
;; 8192 起是 within 写出的下标（i32）。
(module
  (memory (export "mem") 1)

  ;; 索敌：射程（平方）内「走得最远」的那个（离城堡最近，塔防的经典优先级）；没有返回 -1
  (func (export "pick") (param $n i32) (param $tx f32) (param $ty f32) (param $r2 f32) (result i32)
    (local $k i32) (local $bi i32) (local $base i32) (local $best f32) (local $dx f32) (local $dy f32)
    i32.const -1 local.set $bi
    f32.const -1 local.set $best
    block $done loop $next
      local.get $k local.get $n i32.ge_s br_if $done
      local.get $k i32.const 4 i32.shl local.set $base
      local.get $base f32.load offset=0 local.get $tx f32.sub local.set $dx
      local.get $base f32.load offset=4 local.get $ty f32.sub local.set $dy
      local.get $dx local.get $dx f32.mul local.get $dy local.get $dy f32.mul f32.add local.get $r2 f32.le
      if
        local.get $base f32.load offset=8 local.get $best f32.gt
        if
          local.get $base f32.load offset=8 local.set $best
          local.get $k local.set $bi
        end
      end
      local.get $k i32.const 1 i32.add local.set $k
      br $next
    end end
    local.get $bi)

  ;; 范围：圆内的候选下标依次写到 8192 起，返回个数（炮塔溅射、冰塔减速、电塔找下一跳）
  (func (export "within") (param $n i32) (param $cx f32) (param $cy f32) (param $r2 f32) (result i32)
    (local $k i32) (local $cnt i32) (local $base i32) (local $dx f32) (local $dy f32)
    block $done loop $next
      local.get $k local.get $n i32.ge_s br_if $done
      local.get $k i32.const 4 i32.shl local.set $base
      local.get $base f32.load offset=0 local.get $cx f32.sub local.set $dx
      local.get $base f32.load offset=4 local.get $cy f32.sub local.set $dy
      local.get $dx local.get $dx f32.mul local.get $dy local.get $dy f32.mul f32.add local.get $r2 f32.le
      if
        i32.const 8192 local.get $cnt i32.const 2 i32.shl i32.add local.get $k i32.store offset=0
        local.get $cnt i32.const 1 i32.add local.set $cnt
      end
      local.get $k i32.const 1 i32.add local.set $k
      br $next
    end end
    local.get $cnt)

  ;; 伤害：护甲直接减，但至少留 25%；pierce（电塔）无视护甲
  (func (export "dmg") (param $base f32) (param $armor f32) (param $pierce i32) (result f32)
    local.get $pierce
    if (result f32)
      local.get $base
    else
      local.get $base f32.const 0.25 f32.mul
      local.get $base local.get $armor f32.sub
      f32.max
    end))
