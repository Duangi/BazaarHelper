# App.tsx 重构计划

## 目标
将 3700+ 行的 App.tsx 拆分成多个模块化文件

## 已完成
✅ src/types/index.ts - 类型定义
✅ src/constants/colors.ts - 颜色常量
✅ src/utils/helpers.ts - 工具函数
✅ src/utils/renderText.tsx - 文本渲染函数
✅ src/hooks/useWindowManager.ts - 窗口管理 Hook

## 待完成

### 1. 窗口事件监听 Hook
- src/hooks/useWindowListeners.ts
  - 监听窗口移动
  - 监听窗口调整大小
  - 处理 ref 更新

### 2. 视图组件
- src/views/MonsterView.tsx - 野怪一览
- src/views/CardRecognitionView.tsx - 卡牌识别
- src/views/ItemsView.tsx - 手头物品（手牌+仓库）
- src/views/SearchView.tsx - 百科搜索

### 3. 其他组件
- src/components/TopBar.tsx - 顶部工具栏
- src/components/TabBar.tsx - 标签栏
- src/components/Toast.tsx - 提示消息
- src/components/VersionScreen.tsx - 启动版本屏幕

### 4. 自定义 Hooks
- src/hooks/useMonsterData.ts - 野怪数据管理
- src/hooks/useSyncData.ts - 物品同步
- src/hooks/useYoloMonitor.ts - YOLO 监控

## 建议分步执行
1. 先测试当前已拆分的部分是否工作
2. 逐个创建视图组件
3. 最后精简 App.tsx

## 当前问题
收起/展开宽度不稳定 - 需要在重构中重点关注窗口尺寸管理逻辑
