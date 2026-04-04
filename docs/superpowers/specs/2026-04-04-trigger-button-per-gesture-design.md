# 触发按键配置设计：从全局设置改为每个手势条目属性

**日期**: 2026-04-04

## 背景

当前系统中，手势触发按键（`Settings.trigger_button`）是全局设置，所有手势共用同一个触发按键。用户希望每个手势条目可以指定自己的触发按键（右键/中键/X1/X2），匹配时按键和方向序列都一致才执行动作。

## 目标

1. 移除全局 `Settings.trigger_button` 的限制，允许所有4个按键触发手势跟踪
2. 在配置界面的编辑对话框中增加触发按键单选按钮（中键/右键/X1/X2）
3. 保持现有 key 编码方式（`M_Right → Down`）不变，匹配逻辑不变

## 方案

选择保持现有 HashMap key 编码方式（选项 B），trigger_button 继续编码在 key 前缀中。改动最小且向后兼容。

## 改动范围

### 1. `src/core/tracker.rs` — 移除全局按键限制

- tracker 不再只监听 `Settings.trigger_button` 指定的按键
- 右键、中键、X1、X2 任意按下都启动手势跟踪
- 按下的按键记录到 `Gesture.trigger_button`

### 2. `src/ui/gesture_app.slint` — 编辑对话框增加单选按钮

- 在「手势名称」和「手势序列」之间增加一行：`触发按键: ○中键 ○右键 ○X1 ○X2`
- 使用自定义 RadioButton 组件（Slint 无内置 RadioButton）
- 新增属性：`edit-trigger-button-index: int`（0=中键, 1=右键, 2=X1, 3=X2）

### 3. `src/ui/config_dialog.rs` — 处理触发按键读写

- 编辑时：从 gesture key 前缀解析 trigger_button，设置到单选按钮
- 确认时：读取选中的按键，生成带前缀的 gesture key
- 捕捉时：捕捉结果自带 trigger_button，同步到单选按钮

### 4. `src/config/config.rs` — 移除 Settings.trigger_button

- 从 `Settings` 中移除 `trigger_button` 字段
- 添加 `#[serde(default)]` 保持旧配置文件向后兼容

### 不改动的部分

- `src/core/gesture.rs` — 结构体不变
- `src/core/intent.rs` — 匹配逻辑不变
- `src/core/capture.rs` — 捕捉逻辑不变

## 向后兼容

- 旧配置文件 `Settings.trigger_button` 字段通过 serde 默认值静默忽略
- 已有的 `M_Right` 等 key 格式完全兼容
