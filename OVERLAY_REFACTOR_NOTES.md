# Overlay 架构重构 - 集成指南

## ✅ 已完成的修改

### 1. 配置文件
- ✅ `tauri.conf.json`: 添加了两个新窗口配置
  - `yolo-monitor`: YOLO监控窗口（300x200，左上角）
  - `detail-popup`: 详情弹出窗口（400x600）

### 2. 前端组件
- ✅ `src/YoloMonitor.tsx`: YOLO监控组件
- ✅ `src/DetailPopup.tsx`: 详情弹出组件（包含缩放动画）
- ✅ `src/main.tsx`: 路由逻辑更新
- ✅ `yolo-monitor.html` 和 `detail-popup.html`: HTML入口文件

### 3. Rust后端
- ✅ 窗口初始化逻辑（始终保持点击穿透）
- ✅ 新命令：
  - `show_yolo_monitor_window(show: bool)`
  - `show_detail_popup_at(x, y, data_type, data)`
  - `hide_detail_popup()`

## 📝 需要在 App.tsx 中集成的代码

### 1. YOLO Monitor 显示控制

在设置中切换 YOLO Monitor 时：

```typescript
// 替换原来的 localStorage 设置
const toggleYoloMonitor = async (show: bool) => {
    await invoke('show_yolo_monitor_window', { show });
    localStorage.setItem('show-yolo-monitor', show.toString());
};
```

### 2. 右键点击处理

在处理 YOLO 扫描结果的右键点击时：

```typescript
// 监听右键点击事件
useEffect(() => {
    const unlisten = listen('global-right-click', async (event: any) => {
        const { x, y } = event.payload;
        
        // 调用后端检测
        const result = await invoke('handle_overlay_right_click', { x, y });
        
        if (result) {
            // 显示详情弹出窗口
            await invoke('show_detail_popup_at', {
                x: x - 200, // 偏移让窗口居中在点击位置
                y: y - 300,
                dataType: result.type,
                data: result.data
            });
        } else {
            // 点击空白处，隐藏详情
            await invoke('hide_detail_popup');
        }
    });
    
    return () => { unlisten.then(fn => fn()); };
}, []);
```

### 3. 关闭详情窗口

用户可以通过以下方式关闭详情：
- 点击空白处（自动检测）
- 按 ESC 键（需要在 DetailPopup 中添加键盘监听）

```typescript
// 在 DetailPopup.tsx 中添加
useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
            invoke('hide_detail_popup');
        }
    };
    
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
}, []);
```

## 🎨 新架构的优势

1. **不再有标题栏问题**: 两个小窗口始终点击穿透，Windows不会显示标题栏
2. **更好的动画**: 详情框从点击位置放大，视觉效果更好
3. **独立管理**: 每个窗口职责清晰，易于维护
4. **更流畅**: 不需要频繁切换点击穿透状态

## 🔄 迁移步骤

1. 删除 `OverlayApp.tsx` 中的旧代码（可选，作为备份保留）
2. 在 App.tsx 中集成上述代码
3. 测试 YOLO Monitor 的显示/隐藏
4. 测试右键点击详情的显示和缩放动画
5. 测试点击空白处关闭详情

## 🐛 调试技巧

- 查看浏览器控制台：打开 Dev Tools 查看每个窗口
- Rust日志：查看终端输出的调试信息
- 窗口位置：可以在 `show_detail_popup_at` 中调整偏移量，使窗口更好地居中

## 📌 注意事项

- 两个窗口都设置了 `focusable: false`，不会抢夺游戏焦点
- macOS 上使用 NSPanel 技术，可以覆盖全屏游戏
- Windows 上使用透明窗口 + 点击穿透，避免标题栏
