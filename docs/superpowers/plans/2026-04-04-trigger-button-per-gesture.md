# 触发按键配置实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将手势触发按键从全局设置改为每个手势条目的属性，并在配置界面中增加触发按键单选按钮。

**Architecture:** 保持现有 HashMap key 编码方式（`M_Right → Down`），仅移除全局 `Settings.trigger_button` 限制，在 UI 编辑对话框中增加触发按键选择。tracker 已经支持多按键触发，无需改动。

**Tech Stack:** Rust, Slint UI, serde

---

### Task 1: 移除 config.rs 中的全局 trigger_button 限制

**Files:**
- Modify: `src/config/config.rs` (Settings 结构体)

- [ ] **Step 1: 从 Settings 中移除 trigger_button 字段**

在 `src/config/config.rs` 中，从 `Settings` 结构体中移除 `trigger_button` 字段，同时移除 `TriggerButton` 枚举（如果不再被其他地方使用）。在 `Settings::default()` 中也移除对应行。

`Settings` 结构体改为：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Minimum distance in pixels before gesture starts
    pub min_distance: u32,

    /// Minimum distance in pixels for a gesture direction
    pub effective_move: u32,

    /// Timeout in milliseconds before gesture is cancelled
    pub stay_timeout: u32,

    /// Enable 8-direction gestures (only first stroke)
    pub enable_8_direction: bool,

    /// Disable gestures in fullscreen applications
    pub disable_in_fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            min_distance: 5,
            effective_move: 20,
            stay_timeout: 500,
            enable_8_direction: false,
            disable_in_fullscreen: true,
        }
    }
}
```

添加 `#[serde(default)]` 属性保持旧配置向后兼容：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // ... 字段同上
}

impl Default for Settings {
    // ... 同上
}
```

删除 `TriggerButton` 枚举定义。

- [ ] **Step 2: 编译检查**

Run: `cargo check 2>&1 | head -50`

预期会有编译错误，因为多处引用了 `TriggerButton` 和 `settings.trigger_button`。记录这些错误，后续 task 修复。

- [ ] **Step 3: Commit**

```bash
git add src/config/config.rs
git commit -m "refactor(config): remove global trigger_button from Settings"
```

---

### Task 2: 更新 recognizer.rs 移除 trigger_button 参数

**Files:**
- Modify: `src/core/recognizer.rs`

- [ ] **Step 1: 移除 GestureRecognizer 中的 trigger_button 字段和相关逻辑**

`GestureRecognizer` 结构体移除 `trigger_button` 字段。构造函数移除 `trigger_button` 参数，移除 `convert_trigger_button` 方法。

```rust
pub struct GestureRecognizer {
    tracker: PathTracker,
    max_gesture_steps: usize,
}

impl GestureRecognizer {
    pub fn new(settings: Settings) -> Self {
        let tracker = PathTracker::new(settings);
        Self {
            tracker,
            max_gesture_steps: 12,
        }
    }

    // ... 其他方法不变，移除 convert_trigger_button

    /// Handle a mouse event
    pub fn handle_mouse_event(&mut self, event: &MouseEvent) {
        // 不再传 trigger_button，tracker 内部根据事件类型决定
        self.tracker.handle_mouse_event(event);
    }
}
```

`handle_mouse_event` 方法签名改为不再需要 `_trigger_button` 参数（tracker 侧也一并修改）。

更新 `create_shared_recognizer` 函数：

```rust
pub fn create_shared_recognizer(settings: Settings) -> SharedRecognizer {
    Arc::new(Mutex::new(GestureRecognizer::new(settings)))
}
```

- [ ] **Step 2: 更新 tracker.rs 的 handle_mouse_event 签名**

在 `src/core/tracker.rs` 中，`handle_mouse_event` 方法移除 `_trigger_button` 参数：

```rust
pub fn handle_mouse_event(&mut self, event: &MouseEvent) {
    match event {
        // ... 现有匹配逻辑不变
    }
}
```

- [ ] **Step 3: 更新测试**

在 `src/core/recognizer.rs` 和 `src/core/tracker.rs` 的测试中，移除所有 `TriggerButton` 引用，调整构造函数调用。

recognizer.rs 测试：
```rust
#[test]
fn test_recognizer_creation() {
    let settings = Settings::default();
    let recognizer = GestureRecognizer::new(settings);
    assert_eq!(recognizer.state(), &TrackerState::Idle);
}

#[test]
fn test_recognizer_tracking() {
    let settings = Settings::default();
    let mut recognizer = GestureRecognizer::new(settings);
    recognizer.handle_mouse_event(&MouseEvent::MiddleButtonDown(100, 100));
    assert!(recognizer.is_capturing());
    recognizer.handle_mouse_event(&MouseEvent::MouseMove(120, 100));
    assert!(recognizer.is_tracking());
}
```

- [ ] **Step 4: 编译检查**

Run: `cargo check 2>&1 | head -50`

预期还有 `hook_callback.rs` 和 `app.rs` 的编译错误。

- [ ] **Step 5: Commit**

```bash
git add src/core/recognizer.rs src/core/tracker.rs
git commit -m "refactor(recognizer): remove trigger_button parameter, all buttons can trigger"
```

---

### Task 3: 更新 hook_callback.rs 和 app.rs

**Files:**
- Modify: `src/core/hook_callback.rs`
- Modify: `src/core/app.rs`

- [ ] **Step 1: 更新 GestureHookCallback**

在 `src/core/hook_callback.rs` 中，移除 `TriggerButton` 导入和构造函数中的 `_trigger_button` 参数：

```rust
use crate::core::recognizer::SharedRecognizer;
use crate::winapi::hook::{MouseEvent, MouseHookCallback, set_processing_mouse_moves};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

pub struct GestureHookCallback {
    recognizer: SharedRecognizer,
    enabled: Arc<AtomicBool>,
    _event_sender: mpsc::Sender<MouseEvent>,
}

impl GestureHookCallback {
    pub fn new(
        recognizer: SharedRecognizer,
        enabled: Arc<AtomicBool>
    ) -> Self {
        info!("GestureHookCallback created with multi-button support (Right/Middle/X1/X2)");
        // ... 其余不变
    }
}
```

- [ ] **Step 2: 更新 GestureApp**

在 `src/core/app.rs` 中，移除 `TriggerButton` 引用，更新 `create_shared_recognizer` 调用：

```rust
let recognizer = create_shared_recognizer(config.settings.clone());
```

移除 `TriggerButton` 的 `use` 语句。

- [ ] **Step 3: 编译检查**

Run: `cargo check 2>&1 | head -50`

预期编译通过（可能还有 config_dialog.rs 的警告，因为 `TriggerButton` 被移除）。如果有其他编译错误，修复。

- [ ] **Step 4: 运行测试**

Run: `cargo test 2>&1`

预期所有测试通过。

- [ ] **Step 5: Commit**

```bash
git add src/core/hook_callback.rs src/core/app.rs
git commit -m "refactor(app): update hook callback and app to remove trigger_button"
```

---

### Task 4: 在 gesture_app.slint 中添加触发按键单选按钮

**Files:**
- Modify: `src/ui/gesture_app.slint`

- [ ] **Step 1: 添加 RadioButton 组件**

在 `gesture_app.slint` 中添加自定义 RadioButton 组件（在 `GestureRow` 组件之后）：

```slint
// Custom radio button component
component RadioButton inherits Rectangle {
    height: 24px;
    in property <bool> checked: false;
    in property <string> label: "";
    callback toggled();

    HorizontalLayout {
        width: 100%;
        height: 100%;
        spacing: 4px;

        // Radio circle
        Rectangle {
            width: 16px;
            height: 16px;
            y: (parent.height - 16px) / 2;
            border-radius: 8px;
            border-width: 1px;
            border-color: root.checked ? #0078d4 : #999999;
            background: transparent;

            // Inner dot when checked
            if root.checked: Rectangle {
                width: 8px;
                height: 8px;
                x: (parent.width - 8px) / 2;
                y: (parent.height - 8px) / 2;
                border-radius: 4px;
                background: #0078d4;
            }
        }

        // Label text
        Text {
            text: root.label;
            font-size: 12px;
            color: root.checked ? #0078d4 : #555555;
            vertical-alignment: center;
        }
    }

    ta := TouchArea {
        width: 100%;
        height: 100%;
        clicked => { root.toggled(); }
    }
}
```

- [ ] **Step 2: 在 GestureAppWindow 中添加属性和单选按钮**

添加新属性 `edit-trigger-button-index`：

```slint
in-out property <int> edit-trigger-button-index: 0;
```

在编辑对话框中，「手势名称」和「手势序列」之间插入触发按键行：

```slint
// 触发按键
Text {
    text: "触发按键:";
    font-size: 12px;
    color: #555555;
}

HorizontalLayout {
    height: 28px;
    spacing: 16px;

    RadioButton {
        label: "中键";
        checked: root.edit-trigger-button-index == 0;
        toggled => { root.edit-trigger-button-index = 0; }
    }
    RadioButton {
        label: "右键";
        checked: root.edit-trigger-button-index == 1;
        toggled => { root.edit-trigger-button-index = 1; }
    }
    RadioButton {
        label: "X1";
        checked: root.edit-trigger-button-index == 2;
        toggled => { root.edit-trigger-button-index = 2; }
    }
    RadioButton {
        label: "X2";
        checked: root.edit-trigger-button-index == 3;
        toggled => { root.edit-trigger-button-index = 3; }
    }
}
```

- [ ] **Step 3: 编译检查**

Run: `cargo check 2>&1 | head -50`

预期 UI 模块编译通过。如果 Slint 编译错误，修复布局问题。

- [ ] **Step 4: Commit**

```bash
git add src/ui/gesture_app.slint
git commit -m "feat(ui): add trigger button radio buttons to edit dialog"
```

---

### Task 5: 更新 config_dialog.rs 处理触发按键

**Files:**
- Modify: `src/ui/config_dialog.rs`

- [ ] **Step 1: 添加 trigger_button 索引转换函数**

在 `config_dialog.rs` 中添加辅助函数（在现有的 `index_to_window_command` 之后）：

```rust
/// Convert GestureTriggerButton to radio button index (0=Middle, 1=Right, 2=X1, 3=X2)
fn trigger_button_to_index(button: &crate::core::gesture::GestureTriggerButton) -> i32 {
    match button {
        crate::core::gesture::GestureTriggerButton::Middle => 0,
        crate::core::gesture::GestureTriggerButton::Right => 1,
        crate::core::gesture::GestureTriggerButton::X1 => 2,
        crate::core::gesture::GestureTriggerButton::X2 => 3,
    }
}

/// Convert radio button index to GestureTriggerButton
fn index_to_trigger_button(index: i32) -> crate::core::gesture::GestureTriggerButton {
    match index {
        0 => crate::core::gesture::GestureTriggerButton::Middle,
        1 => crate::core::gesture::GestureTriggerButton::Right,
        2 => crate::core::gesture::GestureTriggerButton::X1,
        3 => crate::core::gesture::GestureTriggerButton::X2,
        _ => crate::core::gesture::GestureTriggerButton::Middle,
    }
}
```

- [ ] **Step 2: 更新 add-gesture-clicked 回调**

在 `on_add_gesture_clicked` 中，初始化 `edit-trigger-button-index` 为 0（默认中键）：

在已有的 `win.set_edit_window_command_index(0);` 之后添加：
```rust
win.set_edit_trigger_button_index(0);
```

- [ ] **Step 3: 更新 edit-gesture-clicked 回调**

在 `on_edit_gesture_clicked` 中，解析 gesture key 后，将 trigger_button 设置到 UI：

在已有的变量解构中（`let (trigger_button, directions, ...)`），已经有了 `trigger_button`。

在 `win.set_edit_window_command_index(window_cmd_idx);` 之后添加：
```rust
win.set_edit_trigger_button_index(trigger_button_to_index(&trigger_button));
```

- [ ] **Step 4: 更新 capture timer**

在 capture timer 的 `if let Some(captured)` 块中，在设置 `win.set_edit_capturing(false);` 之后添加：

```rust
win.set_edit_trigger_button_index(trigger_button_to_index(&captured.trigger_button));
```

- [ ] **Step 5: 更新 edit-dialog-confirmed 回调**

在 `on_edit_dialog_confirmed` 中，将 `edit_trigger_button_index` 映射为 trigger_button：

在读取窗口值的部分添加 `edit_trigger_button_index`：
```rust
let (action_type_idx, action_detail, window_cmd_idx, gesture_name_input, trigger_button_idx) = {
    if let Some(win) = window_weak.upgrade() {
        (
            win.get_edit_action_type_index(),
            win.get_edit_action_detail().to_string(),
            win.get_edit_window_command_index(),
            win.get_edit_gesture_name().to_string(),
            win.get_edit_trigger_button_index(),
        )
    } else {
        return;
    }
};
```

在生成 `button_prefix` 时，使用从 UI 读取的索引（仅在非捕捉模式下，即 `edit_directions` 非空但用户可能手动选择了不同按键）：

将现有的 `button_prefix` 生成逻辑替换为：
```rust
let trigger_button = index_to_trigger_button(trigger_button_idx);
let button_prefix = match trigger_button {
    crate::core::gesture::GestureTriggerButton::Right => "R_",
    crate::core::gesture::GestureTriggerButton::Middle => "M_",
    crate::core::gesture::GestureTriggerButton::X1 => "X1_",
    crate::core::gesture::GestureTriggerButton::X2 => "X2_",
};
```

同时在确认回调中更新 `st.edit_trigger_button`：
```rust
st.edit_trigger_button = trigger_button;
```

- [ ] **Step 6: 编译检查**

Run: `cargo check 2>&1 | head -50`

预期编译通过。

- [ ] **Step 7: 运行测试**

Run: `cargo test 2>&1`

预期所有测试通过。

- [ ] **Step 8: Commit**

```bash
git add src/ui/config_dialog.rs
git commit -m "feat(ui): wire up trigger button selection in edit dialog"
```

---

### Task 6: 最终验证

- [ ] **Step 1: 完整编译**

Run: `cargo build 2>&1`

预期编译成功。

- [ ] **Step 2: 运行所有测试**

Run: `cargo test 2>&1`

预期全部通过。

- [ ] **Step 3: 最终 commit**

```bash
git add -A
git commit -m "feat: per-gesture trigger button selection in config UI"
```
