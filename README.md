# Enzyme Toolbox（T5M Unlock Tool）

面向 T5m 305 设备的桌面工具箱：ADB 环境检测、设备管理与 Root 提权。

基于 Tauri 2 + Vue 3 + TypeScript + Naive UI 构建，采用步骤引导式交互，全程实时日志输出。

## 功能特性

- **ADB 环境检查**：自动查找系统中已安装的 adb；未安装时自动下载官方 platform-tools（Windows / Linux）
- **设备选择**：列出并选择已连接的 ADB 设备
- **Root 提权**：向设备推送内置的 `preload.so`，通过 `LD_PRELOAD` 注入 `app_process` 进程，完成后自动执行 `su -c 'id'` 验证是否获得 root 权限
- **执行日志**：实时展示提权过程各步骤输出，支持一键导出为文本文件
- **跨平台**：Windows / Linux 均可构建运行

## 技术栈

- [Tauri 2](https://tauri.app/)（Rust 后端）
- [Vue 3](https://vuejs.org/) + TypeScript + [Vite](https://vite.dev/)
- [Naive UI](https://www.naiveui.com/) 组件库
- Vue Router / Vue I18n / Axios / MingCute Icons
- 包管理器：[pnpm](https://pnpm.io/)

## 开发与构建

### 环境要求

- Node.js 20+、pnpm 8+
- Rust stable（含各平台编译依赖）

### 安装依赖

```bash
pnpm install
```

### 本地开发

```bash
pnpm tauri dev
```

### 构建产物

```bash
pnpm tauri build
```

- Windows：生成 NSIS 安装包；另可用 `pnpm tauri build --no-bundle` 得到免安装的便携版 exe
- Linux：生成 deb / rpm / AppImage

Linux 构建前需安装系统依赖：

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

## 注意事项

- 提权库文件 `src/assets/preload/` 已通过 `.gitignore` 排除，不入库。构建前需将对应的 `enzymeym-t5m305-preload-dev.so` 放回该目录（编译期通过 `include_bytes!` 内嵌进二进制）。
- 提权仅针对 T5m 305 设备，请仅在自有设备上进行研究、学习使用，并遵守当地法律法规。

## 许可证

本项目基于 [GNU GPL v3](LICENSE) 开源协议发布。

Copyright (C) 2026 酶游明（Enzymeym）
