# The Bazaar Helper

一个为 **The Bazaar** 游戏设计的强力悬浮窗辅助工具。它通过图像识别和日志分析技术，为您提供实时的备战建议、怪物技能详情以及卡牌出现概率参考。

## ✨ 核心功能

### 1️⃣ 智能怪物识别 (Monster Recognition)
- **全自动检测**：进入战斗前自动识别当前遇到的三个怪物。
- **动态天数适配**：自动分析游戏日志，匹配当前天数（Day 1 - Day 10+）的数值强度。
- **技能预览**：一键查看怪物的全部技能、血量、等级及其独有的掉落物品。

### 2️⃣ 实时概率显示 (Drop Probabilities)
- **精准掉落率**：根据当前游戏天数，实时显示商店中四种品质卡牌的出现概率：
  - <span style="color: #CD7F32; font-weight: bold;">铜牌 (Bronze)</span>
  - <span style="color: #C0C0C0; font-weight: bold;">银牌 (Silver)</span>
  - <span style="color: #FFD700; font-weight: bold;">金牌 (Gold)</span>
  - <span style="color: #B9F2FF; font-weight: bold;">钻石 (Diamond)</span>
- **一键切换预览**：支持 2x5 布局的天数选择器，点击即可预览未来天数的概率走向。

### 3️⃣ 升级路径横向对比 (Upgrade Progression)
- **数值变化一目了然**：点击卡片描述，即可在当前等级与各阶级（铜/银/金/钻）之间切换数值对比。
- **精准颜色标识**：所有卡牌边框和数值颜色均遵循官方阶级标准：
  - 铜: `#CD7F32` | 银: `#C0C0C0` | 金: `#FFD700` | 钻: `#B9F2FF`

### 4️⃣ 多维度数据同步
- **手牌/仓库实时同步**：基于图像与日志双重校验，实时更新你的当前作战配置。
- **词条语义高亮**：毒、燃烧、冰冻、急速等关键词条自动彩色高亮，描述更易读。

---

## 📦 下载与安装

1. 前往 [Releases](https://github.com/Duangi/BazaarHelper/releases) 页面。
2. **Windows 用户**：下载 `BazaarHelper_x.x.x_x64-setup.exe` 运行即可。
3. **Mac 用户**：下载 `BazaarHelper_x.x.x_x64.dmg`。
   > **注意**：Mac 版日志文件位于 `~/Library/Logs/Tempo Storm/The Bazaar/`，程序已自动适配。

---

## 🎮 使用说明

1. **窗口操作**：
   - 拖动顶部工具栏可移动窗口。
   - 拖动右下角边缘可调整大小。
   - 点击右上角收起/展开，保持简洁。
2. **识别怪物**：
   - 遇到怪物画面时，稍等一秒或点击 **🎯 识别画面中的怪物** 按钮。
3. **查看升级**：
   - 在卡片描述区域点击，即可看到该卡牌在不同颜色阶级下的数值差异。

---

## 🎯 常见问题 (FAQ)

- **为什么识别不到？** 
  - 请确保游戏窗口处于显示状态，分辨率建议使用 1080P 或更高（兼容 2K/4K）。
- **概率准确吗？**
  - 概率数据来源于游戏机制反编译与社区统计，仅供选牌参考。
- **支持自动更新吗？**
  - 为保证游戏运行期间的极致稳定性，**已移除自动更新功能**，请定期访问项目页面确认最新版本。

---

## 💻 项目开发

### 开发模式
```powershell
npm run tauri dev
```

### 构建生产版本
```powershell
# Windows
npm run tauri build -- --target nsis

# macOS (需在 Mac 设备上运行)
npm run tauri build -- --target dmg
```

---

## 📞 联系与支持

如果在使用过程中遇到 Bug 或有改进建议：
- **GitHub Issues**：提交反馈。
- **开发者 QQ**：309349894

---
*声明：本工具为第三方助手，不收集任何用户隐私。*

## 注意事项

- 首次运行时，Tauri 会下载和编译依赖，可能需要较长时间
- 窗口会始终显示在最上层，包括游戏之上
- 如需关闭始终置顶，可将 `alwaysOnTop` 设置为 `false`

## 技术栈

- **前端框架**: React 18 + TypeScript
- **构建工具**: Vite
- **桌面框架**: Tauri 2.x
- **后端语言**: Rust
