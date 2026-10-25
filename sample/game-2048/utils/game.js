// 2048 的纯逻辑：棋盘是 size×size 的二维数组，0 表示空格。不碰 wx、不碰页面，方便推演与测试。

function empty(size) {
  const b = []
  for (let r = 0; r < size; r++) {
    const row = []
    for (let c = 0; c < size; c++) row.push(0)
    b.push(row)
  }
  return b
}

function clone(board) {
  return board.map(row => row.slice())
}

// 在空格里随机放一个方块：90% 是 2，10% 是 4。空格按行优先排序后取下标，保证同一种子结果确定。
// 返回放下的位置 { r, c, v }，没有空格时返回 null
function spawn(board, rng) {
  const cells = []
  for (let r = 0; r < board.length; r++) {
    for (let c = 0; c < board.length; c++) {
      if (!board[r][c]) cells.push([r, c])
    }
  }
  if (!cells.length) return null
  const [r, c] = cells[rng.int(cells.length)]
  const v = rng.next() < 0.9 ? 2 : 4
  board[r][c] = v
  return { r, c, v }
}

// 把一行往左挤并合并：每个方块一次移动里最多合并一次（[2,2,2,2] → [4,4,0,0]，不是 [8,0,0,0]）
function slideLine(line) {
  const nums = line.filter(v => v)
  const out = []
  let gained = 0
  for (let i = 0; i < nums.length; i++) {
    if (i + 1 < nums.length && nums[i] === nums[i + 1]) {
      out.push(nums[i] * 2)
      gained += nums[i] * 2
      i++
    } else {
      out.push(nums[i])
    }
  }
  while (out.length < line.length) out.push(0)
  return { line: out, gained }
}

// 按方向取出「往左挤」视角下的各条线，挤完再写回去
function readLine(board, dir, i) {
  const n = board.length
  const line = []
  for (let k = 0; k < n; k++) {
    if (dir === 'left') line.push(board[i][k])
    else if (dir === 'right') line.push(board[i][n - 1 - k])
    else if (dir === 'up') line.push(board[k][i])
    else line.push(board[n - 1 - k][i])
  }
  return line
}

function writeLine(board, dir, i, line) {
  const n = board.length
  for (let k = 0; k < n; k++) {
    if (dir === 'left') board[i][k] = line[k]
    else if (dir === 'right') board[i][n - 1 - k] = line[k]
    else if (dir === 'up') board[k][i] = line[k]
    else board[n - 1 - k][i] = line[k]
  }
}

// 返回 { board: 新棋盘, moved: 是否有方块动了, gained: 本次得分 }，不改传入的棋盘
function move(board, dir) {
  const next = clone(board)
  let gained = 0
  let moved = false
  for (let i = 0; i < board.length; i++) {
    const before = readLine(board, dir, i)
    const res = slideLine(before)
    gained += res.gained
    if (res.line.some((v, k) => v !== before[k])) moved = true
    writeLine(next, dir, i, res.line)
  }
  return { board: next, moved, gained }
}

// 还能不能动：有空格，或者横竖相邻有相同的数
function canMove(board) {
  const n = board.length
  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      const v = board[r][c]
      if (!v) return true
      if (c + 1 < n && board[r][c + 1] === v) return true
      if (r + 1 < n && board[r + 1][c] === v) return true
    }
  }
  return false
}

function maxTile(board) {
  let m = 0
  board.forEach(row => row.forEach(v => { if (v > m) m = v }))
  return m
}

// 手指位移 → 方向；位移太小（<24px）当成点击，返回空串
function direction(dx, dy) {
  if (Math.max(Math.abs(dx), Math.abs(dy)) < 24) return ''
  if (Math.abs(dx) > Math.abs(dy)) return dx > 0 ? 'right' : 'left'
  return dy > 0 ? 'down' : 'up'
}

module.exports = { empty, clone, spawn, slideLine, move, canMove, maxTile, direction }
