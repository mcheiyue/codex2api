# Windows 桌面版封装

本仓库采用**最小侵入式桌面封装**：保留上游 Go 服务与 `frontend/` 管理后台原结构，仅额外增加 `src-tauri/` 与少量构建脚本。

## 运行方式

- 桌面壳启动时自动拉起 `codex2api.exe`
- 默认在本地数据目录生成 SQLite 配置，并把数据库文件放在桌面版运行目录
- 健康检查通过后自动打开 `http://127.0.0.1:8080/admin/`
- 关闭窗口时只隐藏到系统托盘
- 只有点击托盘菜单中的“退出”，才会真正结束后端进程并关闭应用

## 目录说明

- `desktop/`：桌面壳的本地启动页，只负责显示启动状态
- `src-tauri/`：Tauri 2 桌面层、托盘逻辑、子进程托管与打包配置
- `scripts/build-codex2api-sidecar.ps1`：构建嵌入前端后的 `codex2api.exe`
- `scripts/build-codex2api-desktop.ps1`：构建 Windows 桌面版并整理便携包
- `.github/workflows/desktop-windows.yml`：Windows 自动构建流水线

## 本地构建

前提：Windows、Node.js、Go、Rust 已安装。

```powershell
./scripts/build-codex2api-desktop.ps1
```

构建完成后可重点查看：

- `dist/codex2api-desktop-portable.zip`

当前默认交付物为便携版 zip，解压后即可直接分发给 Windows 用户使用。

## 首次启动后的配置位置

桌面版首次运行会自动在系统的本地应用数据目录下创建运行目录，并写入 `.env` 与 SQLite 数据文件。后续如需改端口或管理密钥，直接编辑该目录下的 `.env` 即可。

## 与上游同步的建议

1. 保持上游业务代码结构不动
2. 通过 `upstream/main` 同步上游更新
3. 把桌面层改动尽量限制在 `src-tauri/`、`desktop/`、`scripts/`、`.github/workflows/`
4. 同步完成后 push 到自己的仓库，由 GitHub Actions 自动产出新的 Windows 桌面便携包
