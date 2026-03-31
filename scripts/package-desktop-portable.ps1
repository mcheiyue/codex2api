param(
  [string]$ExecutableName = 'Codex2API Desktop.exe'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$releaseDir = Join-Path $repoRoot 'src-tauri/target/release'
$distDir = Join-Path $repoRoot 'dist'
$portableRoot = Join-Path $distDir 'portable/Codex2API Desktop'
$zipPath = Join-Path $distDir 'codex2api-desktop-portable.zip'

if (!(Test-Path $releaseDir)) {
  throw "未找到 Tauri release 目录：$releaseDir"
}

if (Test-Path $portableRoot) {
  Remove-Item -Recurse -Force $portableRoot
}
if (Test-Path $zipPath) {
  Remove-Item -Force $zipPath
}

New-Item -ItemType Directory -Force -Path $portableRoot | Out-Null

$appExe = Get-ChildItem $releaseDir -Filter '*.exe' |
  Where-Object { $_.Name -notlike '*uninstall*' } |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1

if (-not $appExe) {
  throw "未找到桌面程序 exe。"
}

Copy-Item $appExe.FullName (Join-Path $portableRoot $ExecutableName)

$copyCandidates = @('resources', 'WebView2Loader.dll')
foreach ($candidate in $copyCandidates) {
  $candidatePath = Join-Path $releaseDir $candidate
  if (Test-Path $candidatePath) {
    Copy-Item $candidatePath $portableRoot -Recurse -Force
  }
}

Compress-Archive -Path (Join-Path $portableRoot '*') -DestinationPath $zipPath -Force
Write-Host "已生成便携包：$zipPath"
