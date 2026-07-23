# 构建 Windows SDK：生成 mini_render.dll 与导入库 mini_render.dll.lib
# 在 Windows 主机执行（需安装 Visual Studio 生成工具 / MSVC）
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

$target = "x86_64-pc-windows-msvc"
Write-Host "==> 目标: $target"
rustup target add $target
cargo build --release --lib --target $target

$out = "target\windows\$target"
New-Item -ItemType Directory -Force -Path $out | Out-Null
Copy-Item "target\$target\release\mini_render.dll"     $out -ErrorAction SilentlyContinue
Copy-Item "target\$target\release\mini_render.dll.lib" $out -ErrorAction SilentlyContinue
Copy-Item "target\$target\release\mini_render.lib"     $out -ErrorAction SilentlyContinue
Copy-Item "include\mini_render.h" $out

Write-Host "OK Windows SDK 产物位于 $out"
Write-Host "   ARM64 目标: aarch64-pc-windows-msvc"
