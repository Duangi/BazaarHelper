# The Bazaar Helper

这是一个为The Bazaar游戏设计的悬浮窗助手工具，用于显示附魔信息。

## 功能特性

- ✅ 悬浮窗口，始终置顶于游戏之上
- ✅ 可拖动窗口（拖动标题栏）
- ✅ 可调整窗口大小
- ✅ 半透明背景，不遮挡游戏视野
- ✅ 显示12种附魔的详细信息
- ✅ 支持隐藏/显示内容
- ✅ 无边框设计，更美观

## 安装依赖

### 1. 安装 Rust

访问 [https://www.rust-lang.org/learn/get-started](https://www.rust-lang.org/learn/get-started)

或者直接下载安装：
```powershell
# 下载并运行 rustup-init.exe
# 按照提示完成安装
# 安装完成后重启 PowerShell
```

安装完成后验证：
```powershell
rustc --version
cargo --version
```

### 2. 安装 Node.js 依赖（已完成）

```powershell
npm install
```

## 运行项目

### 开发模式
```powershell
npm run tauri dev
```

### 构建生产版本
```powershell
npm run tauri build
```

构建完成后，可执行文件位于：`src-tauri/target/release/`

## 使用说明

1. 启动应用后，会显示一个悬浮窗口
2. 拖动标题栏可以移动窗口位置
3. 拖动窗口边缘可以调整大小
4. 点击"隐藏"按钮可以隐藏表格内容（但保留标题栏）
5. 点击"×"按钮关闭应用

## 项目结构

```
BazaarHelper/
├── src/                    # React 前端代码
│   ├── App.tsx            # 主应用组件
│   ├── App.css            # 样式文件
│   └── main.tsx           # 入口文件
├── src-tauri/             # Tauri 后端代码
│   ├── src/               # Rust 源代码
│   ├── icons/             # 应用图标
│   └── tauri.conf.json    # Tauri 配置文件
└── package.json           # Node.js 依赖配置
```

## 自定义配置

### 修改窗口属性

编辑 `src-tauri/tauri.conf.json`：

```json
{
  "app": {
    "windows": [{
      "width": 600,           // 窗口宽度
      "height": 400,          // 窗口高度
      "resizable": true,      // 是否可调整大小
      "alwaysOnTop": true,    // 是否始终置顶
      "decorations": false,   // 是否显示系统边框
      "transparent": true     // 是否透明
    }]
  }
}
```

### 修改附魔数据

编辑 `src/App.tsx` 中的 `enchantments` 数组。

## 注意事项

- 首次运行时，Tauri 会下载和编译依赖，可能需要较长时间
- 窗口会始终显示在最上层，包括游戏之上
- 如需关闭始终置顶，可将 `alwaysOnTop` 设置为 `false`

## 技术栈

- **前端框架**: React 18 + TypeScript
- **构建工具**: Vite
- **桌面框架**: Tauri 2.x
- **后端语言**: Rust
