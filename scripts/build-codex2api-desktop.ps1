$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$tauriCommand = Get-Command cargo-tauri -ErrorAction SilentlyContinue

if (-not $tauriCommand) {
  throw '未找到 cargo-tauri。请先执行 cargo install tauri-cli --version ^2.0 --locked，或直接使用 GitHub Actions 构建。'
}

& (Join-Path $PSScriptRoot 'build-codex2api-sidecar.ps1')

Push-Location (Join-Path $repoRoot 'src-tauri')
try {
  cargo tauri build
  if ($LASTEXITCODE -ne 0) {
    throw 'cargo tauri build 执行失败。'
  }
}
finally {
  Pop-Location
}

& (Join-Path $PSScriptRoot 'package-desktop-portable.ps1')

Write-Host 'Windows 桌面版便携包构建完成。'
