param(
  [string]$OutputPath = "src-tauri/bin/codex2api.exe"
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$frontendDir = Join-Path $repoRoot 'frontend'
$output = Join-Path $repoRoot $OutputPath
$outputDir = Split-Path -Parent $output

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

Push-Location $frontendDir
try {
  $npmCiSucceeded = $false
  for ($attempt = 1; $attempt -le 3; $attempt++) {
    npm ci
    if ($LASTEXITCODE -eq 0) {
      $npmCiSucceeded = $true
      break
    }

    if ($attempt -eq 3) {
      throw "npm ci 连续 $attempt 次失败，无法继续构建前端。"
    }

    Write-Warning "npm ci 第 $attempt 次失败，2 秒后重试。"
    Start-Sleep -Seconds 2
  }

  if (-not $npmCiSucceeded) {
    throw 'npm ci 执行失败，无法继续构建前端。'
  }

  npm run build
  if ($LASTEXITCODE -ne 0) {
    throw 'npm run build 执行失败。'
  }

  $frontendDistIndex = Join-Path $frontendDir 'dist/index.html'
  if (!(Test-Path $frontendDistIndex)) {
    throw "前端构建产物缺失：$frontendDistIndex"
  }
}
finally {
  Pop-Location
}

Push-Location $repoRoot
try {
  go build -trimpath -ldflags='-s -w' -o $output .
  if ($LASTEXITCODE -ne 0) {
    throw 'go build 执行失败。'
  }
}
finally {
  Pop-Location
}

Write-Host "已生成后端 sidecar: $output"
