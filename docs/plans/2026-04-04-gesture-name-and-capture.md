# 手势名称与捕捉模式 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在编辑手势对话框中添加"手势名称"输入字段，并将手势序列输入方式从手动点击方向按钮改为全局热键捕捉。

**Architecture:** 新增 `GestureEntry` 结构体将 `name` 和 `Action` 合并存储，使用 `#[serde(flatten)]` 保持向后兼容的 JSON 格式。捕捉模式通过全局静态标志和结果存储实现 recognizer 回调与 UI 线程之间的通信。UI 通过 Slint Timer 轮询捕捉结果。

**Tech Stack:** Rust, Slint UI, serde (flatten + tagged enum), std::sync::atomic

---

## Task 1: 在 `GestureDir` 上添加 `dir_name()` 方法

**Files:**
- Modify: `src/core/gesture.rs:41-72`

**Step 1: 添加 `dir_name()` 方法**

在 `GestureDir` 的 impl 块中添加一个返回英文方向名的方法，用于生成配置中的手势键。

```rust
/// Get the direction name used as config key (e.g., "Up", "Down", "UpLeft")
pub fn dir_name(&self) -> &'static str {
    match self {
        GestureDir::Up => "Up",
        GestureDir::Down => "Down",
        GestureDir::Left => "Left",
        GestureDir::Right => "Right",
        GestureDir::UpLeft => "UpLeft",
        GestureDir::UpRight => "UpRight",
        GestureDir::DownLeft => "DownLeft",
        GestureDir::DownRight => "DownRight",
    }
}
```

**Step 2: 添加测试**

```rust
#[test]
fn test_dir_name() {
    assert_eq!(GestureDir::Up.dir_name(), "Up");
    assert_eq!(GestureDir::DownRight.dir_name(), "DownRight");
}
```

**Step 3: 运行测试**

Run: `cargo test test_dir_name`
Expected: PASS

**Step 4: Commit**

```bash
git add src/core/gesture.rs
git commit -m "feat: add GestureDir::dir_name() method for config key generation"
```

---

## Task 2: 添加 `GestureEntry` 结构体并更新 `GestureConfig`

**Files:**
- Modify: `src/config/config.rs:85-92` (Action 附近), `src/config/config.rs:231-283` (GestureConfig)

**Step 1: 添加 `GestureEntry` 结构体**

在 `Action` 定义之后添加：

```rust
/// A gesture entry combining a user-friendly name with an action.
/// Stored as the value in gesture HashMaps (key is the direction sequence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureEntry {
    #[serde(default)]
    pub name: String,
    #[serde(flatten)]
    pub action: Action,
}
```

**Step 2: 更新 `GestureConfig` 的类型定义**

```rust
pub struct GestureConfig {
    pub version: u32,
    pub global_gestures: HashMap<String, GestureEntry>,
    pub app_gestures: HashMap<String, HashMap<String, GestureEntry>>,
    pub disabled_apps: HashSet<String>,
    pub settings: Settings,
}
```

**Step 3: 更新 `GestureConfig::default()`**

```rust
impl Default for GestureConfig {
    fn default() -> Self {
        let mut global_gestures = HashMap::new();
        global_gestures.insert(
            "Right".to_string(),
            GestureEntry {
                name: "Ctrl+L".to_string(),
                action: Action::Keyboard(KeyboardAction {
                    keys: vec!["VK_CONTROL".to_string(), "VK_L".to_string()],
                }),
            },
        );
        global_gestures.insert(
            "Down".to_string(),
            GestureEntry {
                name: "最小化".to_string(),
                action: Action::Window(WindowAction {
                    command: WindowCommand::Minimize,
                }),
            },
        );
        global_gestures.insert(
            "Up".to_string(),
            GestureEntry {
                name: "最大化".to_string(),
                action: Action::Window(WindowAction {
                    command: WindowCommand::Maximize,
                }),
            },
        );
        Self {
            version: 1,
            global_gestures,
            app_gestures: HashMap::new(),
            disabled_apps: HashSet::new(),
            settings: Settings::default(),
        }
    }
}
```

**Step 4: 添加序列化测试**

```rust
#[test]
fn test_gesture_entry_serialization() {
    let entry = GestureEntry {
        name: "复制".to_string(),
        action: Action::Keyboard(KeyboardAction {
            keys: vec!["VK_CONTROL".to_string(), "VK_C".to_string()],
        }),
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("\"name\":\"复制\""));
    assert!(json.contains("\"type\":\"keyboard\""));
    assert!(json.contains("\"keys\""));

    // Round-trip
    let deserialized: GestureEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "复制");
}

#[test]
fn test_gesture_entry_backward_compatible() {
    // Old format without name field should deserialize with empty name
    let json = r#"{"type":"window","command":"Maximize"}"#;
    let entry: GestureEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.name, "");
    assert!(matches!(entry.action, Action::Window(_)));
}
```

**Step 5: 运行测试**

Run: `cargo test test_gesture_entry`
Expected: PASS

**Step 6: Commit**

```bash
git add src/config/config.rs
git commit -m "feat: add GestureEntry struct with name field, update GestureConfig types"
```

---

## Task 3: 更新 `intent.rs` 适配 `GestureEntry`

**Files:**
- Modify: `src/core/intent.rs`

**Step 1: 更新缓存类型和构建方法**

将缓存类型从 `HashMap<String, Action>` 改为只缓存 action（GestureEntry.action），保持查找接口不变：

```rust
fn build_global_cache(config: &GestureConfig) -> HashMap<String, Action> {
    let mut cache = HashMap::new();
    for (gesture_str, entry) in &config.global_gestures {
        cache.insert(gesture_str.clone(), entry.action.clone());
    }
    cache
}

fn build_app_caches(config: &GestureConfig) -> HashMap<String, HashMap<String, Action>> {
    let mut app_caches = HashMap::new();
    for (app_name, gestures) in &config.app_gestures {
        let mut cache = HashMap::new();
        for (gesture_str, entry) in gestures {
            cache.insert(gesture_str.clone(), entry.action.clone());
        }
        app_caches.insert(app_name.clone(), cache);
    }
    app_caches
}
```

**Step 2: 更新 `get_action` 方法**

```rust
pub fn get_action(&self, gesture_str: &str, app_name: Option<&str>) -> Option<&Action> {
    if let Some(app) = app_name {
        if let Some(app_cache) = self.app_caches.get(app) {
            if let Some(action) = app_cache.get(gesture_str) {
                return Some(action);
            }
        }
    }
    self.global_cache.get(gesture_str)
}
```

注意：`get_action` 返回 `&Action`，但缓存现在拥有 Action（从 entry.action.clone() 得来），所以返回引用仍然有效。

**Step 3: 运行测试**

Run: `cargo test --lib`
Expected: 编译通过，所有测试 PASS

**Step 4: Commit**

```bash
git add src/core/intent.rs
git commit -m "refactor: update intent finder to use GestureEntry.action"
```

---

## Task 4: 修复手势键分隔符不一致问题

**Files:**
- Modify: `src/ui/config_dialog.rs` (join separator)
- Modify: `src/core/intent.rs` (确认 separator)

**Step 1: 统一分隔符常量**

在 `src/core/gesture.rs` 中添加常量：

```rust
/// Separator used in gesture direction keys (e.g., "Right → Down")
pub const GESTURE_DIR_SEPARATOR: &str = " → ";
```

**Step 2: 更新 `config_dialog.rs` 使用该常量**

在 `config_dialog.rs` 中：
- `edit_dialog_confirmed` 回调中 `st.edit_directions.join(" -> ")` 改为使用 `" → "`
- `edit-gesture-clicked` 回调中 `name.split(" -> ")` 改为 `name.split(" → ")`
- `gesture_name_to_mnemonic` 中 `name.split(" -> ")` 改为 `name.split(" → ")`
- `direction_index_to_name` 保持不变（返回单个方向名）

**Step 3: 确认 `intent.rs` 已经使用 `" → "`**

`intent.rs` 的 `gesture_to_string` 已经使用 `.join(" → ")`，无需修改。

**Step 4: 运行测试**

Run: `cargo test --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add src/core/gesture.rs src/ui/config_dialog.rs
git commit -m "fix: unify gesture direction separator to ' → ' across config and intent"
```

---

## Task 5: 添加捕捉模式全局状态

**Files:**
- Create: `src/core/capture.rs`
- Modify: `src/core/mod.rs`

**Step 1: 创建 `capture.rs`**

```rust
//! Gesture capture mode for settings UI
//!
//! Provides global state for capturing gestures in the settings dialog.
//! When capture mode is active, the recognizer stores the captured direction
//! sequence instead of executing the matched action.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static CAPTURE_MODE: AtomicBool = AtomicBool::new(false);
static CAPTURE_RESULT: Mutex<Option<Vec<String>>> = Mutex::new(None);

/// Enable capture mode. The next completed gesture will be captured.
pub fn start_capture() {
    // Clear any previous result
    if let Ok(mut result) = CAPTURE_RESULT.lock() {
        *result = None;
    }
    CAPTURE_MODE.store(true, Ordering::SeqCst);
}

/// Cancel capture mode without waiting for a gesture.
pub fn cancel_capture() {
    CAPTURE_MODE.store(false, Ordering::SeqCst);
    if let Ok(mut result) = CAPTURE_RESULT.lock() {
        *result = None;
    }
}

/// Check if currently in capture mode.
pub fn is_capture_mode() -> bool {
    CAPTURE_MODE.load(Ordering::SeqCst)
}

/// Store a captured gesture direction sequence.
/// Called by the recognizer callback when a gesture is completed in capture mode.
pub fn set_capture_result(dirs: Vec<String>) {
    if let Ok(mut result) = CAPTURE_RESULT.lock() {
        *result = Some(dirs);
    }
    CAPTURE_MODE.store(false, Ordering::SeqCst);
}

/// Take the captured gesture result (returns Some if a gesture was captured, None otherwise).
/// This consumes the result.
pub fn take_capture_result() -> Option<Vec<String>> {
    if let Ok(mut result) = CAPTURE_RESULT.lock() {
        result.take()
    } else {
        None
    }
}
```

**Step 2: 在 `core/mod.rs` 中注册模块**

```rust
pub mod capture;
```

**Step 3: Commit**

```bash
git add src/core/capture.rs src/core/mod.rs
git commit -m "feat: add gesture capture mode global state"
```

---

## Task 6: 更新 `app.rs` recognizer 回调支持捕捉模式

**Files:**
- Modify: `src/core/app.rs:57-90`

**Step 1: 修改 recognizer callback**

在 `GestureRecognizerEvent::GestureCompleted` 分支中添加捕捉模式检查：

```rust
GestureRecognizerEvent::GestureCompleted(gesture) => {
    info!("✅ {}", gesture.short_display());

    // Check if in capture mode (for settings UI)
    if crate::core::capture::is_capture_mode() {
        let dirs: Vec<String> = gesture.directions.iter()
            .map(|d| d.dir_name().to_string())
            .collect();
        info!("🎯 Gesture captured: {:?}", dirs);
        crate::core::capture::set_capture_result(dirs);
        return;
    }

    // Normal mode - find and execute action
    let finder = intent_finder_clone.lock().unwrap();
    if let Some(intent) = finder.find(&gesture, None) {
        // ... existing code ...
    }
}
```

**Step 2: 运行编译测试**

Run: `cargo build`
Expected: 编译成功

**Step 3: Commit**

```bash
git add src/core/app.rs
git commit -m "feat: add capture mode check in recognizer callback"
```

---

## Task 7: 更新 Slint UI 编辑对话框

**Files:**
- Modify: `src/ui/gesture_app.slint`

**Step 1: 添加编辑对话框属性**

在 `GestureAppWindow` 的 "Edit Dialog Properties" 区域添加：

```slint
in-out property <string> edit-gesture-name: "";
in-out property <bool> edit-capturing: false;
```

添加回调：

```slint
callback edit-capture-clicked();
callback edit-clear-directions();
```

**Step 2: 替换编辑对话框内容**

将方向网格区域替换为捕捉按钮 + 手势名称输入。编辑对话框内容改为：

```slint
// Dialog content
VerticalLayout {
    vertical-stretch: 1;
    padding-left: 20px;
    padding-right: 20px;
    padding-top: 12px;
    padding-bottom: 16px;
    spacing: 10px;

    // 手势名称
    HorizontalLayout {
        height: 32px;
        spacing: 8px;

        Text {
            text: "手势名称:";
            font-size: 12px;
            color: #555555;
            vertical-alignment: center;
            width: 64px;
        }

        LineEdit {
            text: root.edit-gesture-name;
            edited => {
                root.edit-gesture-name = self.text;
            }
        }
    }

    // 手势序列 - 捕捉方式
    Text {
        text: "手势序列:";
        font-size: 12px;
        color: #555555;
    }

    // 捕捉状态显示区域
    HorizontalLayout {
        height: 40px;
        spacing: 8px;

        Rectangle {
            vertical-stretch: 1;
            background: #f8f8f8;
            border-radius: 4px;
            border-width: 1px;
            border-color: #e0e0e0;

            Text {
                text: root.edit-has-directions
                    ? root.edit-direction-display
                    : (root.edit-capturing ? "请用手势触发键画出方向..." : "点击右侧按钮捕捉手势");
                font-size: root.edit-has-directions ? 16px : 12px;
                color: root.edit-has-directions ? #0078d4 : (root.edit-capturing ? #e65100 : #bbbbbb);
                x: 10px;
                width: parent.width - 10px;
                height: 100%;
                vertical-alignment: center;
            }
        }

        // 捕捉按钮
        Rectangle {
            width: 80px;
            height: 36px;
            y: (parent.height - 36px) / 2;
            background: root.edit-capturing
                ? (capture-ta.has-hover ? #c62828 : #d32f2f)
                : (capture-ta.has-hover ? #1565c0 : #1976d2);
            border-radius: 4px;

            capture-ta := TouchArea {
                width: 100%;
                height: 100%;
                clicked => {
                    root.edit-capture-clicked();
                }
            }

            Text {
                text: root.edit-capturing ? "捕捉中..." : "开始捕捉";
                font-size: 12px;
                color: #ffffff;
                horizontal-alignment: center;
                vertical-alignment: center;
                width: 100%;
                height: 100%;
            }
        }

        // 清除按钮
        Rectangle {
            width: 50px;
            height: 36px;
            y: (parent.height - 36px) / 2;
            background: clear-seq-ta.has-hover ? #e0e0e0 : #f0f0f0;
            border-radius: 4px;
            border-width: 1px;
            border-color: #d0d0d0;

            clear-seq-ta := TouchArea {
                width: 100%;
                height: 100%;
                clicked => {
                    root.edit-clear-directions();
                }
            }

            Text {
                text: "清除";
                font-size: 11px;
                color: #555555;
                horizontal-alignment: center;
                vertical-alignment: center;
                width: 100%;
                height: 100%;
            }
        }
    }

    // ... Action type selector and params remain unchanged ...
}
```

注意：移除旧的 `edit-direction-clicked` 回调和 3x3 DirButton 网格。保留 `edit-clear-directions` 回调。

**Step 3: 移除不再需要的回调**

移除 `callback edit-direction-clicked(int);`，保留 `edit-clear-directions`。

**Step 4: Commit**

```bash
git add src/ui/gesture_app.slint
git commit -m "feat: replace direction grid with capture button and gesture name input"
```

---

## Task 8: 更新 `config_dialog.rs` 使用 GestureEntry 和捕捉逻辑

**Files:**
- Modify: `src/ui/config_dialog.rs`

这是最大的改动，涉及多个部分。

### Part A: 更新数据访问以使用 GestureEntry

**Step 1: 更新 `action_to_type_index`、`action_to_detail` 等函数**

这些函数接收 `&Action`，不需要改签名。但调用处需要从 `GestureEntry` 中提取 `.action`。

**Step 2: 更新 `build_gesture_model`**

```rust
fn build_gesture_model(state: &DialogState) -> slint::ModelRc<GestureItem> {
    let pairs = state.gesture_pairs();
    let items: Vec<GestureItem> = pairs
        .iter()
        .map(|(name, entry)| {
            GestureItem {
                name: SharedString::from(
                    if entry.name.is_empty() { name.as_str() } else { entry.name.as_str() }
                ),
                mnemonic: SharedString::from(gesture_name_to_mnemonic(name).as_str()),
                action_type: SharedString::from(action_type_display(&entry.action).as_str()),
                action_params: SharedString::from(action_params_display(&entry.action).as_str()),
            }
        })
        .collect();
    vec_to_model(items)
}
```

**Step 3: 更新 `gesture_pairs` 返回类型**

```rust
fn gesture_pairs(&self) -> Vec<(String, GestureEntry)> {
    let map = self.gesture_map_for_app(&self.current_app);
    let mut pairs: Vec<(String, GestureEntry)> = map.into_iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}
```

**Step 4: 更新 `gesture_map_for_app` 返回类型**

```rust
fn gesture_map_for_app(&self, app_name: &str) -> HashMap<String, GestureEntry> {
    // same logic, returns HashMap<String, GestureEntry>
}
```

### Part B: 更新编辑对话框回调

**Step 5: 更新 `add-gesture-clicked` 回调**

```rust
window.on_add_gesture_clicked(move || {
    let mut st = state_cb.borrow_mut();
    st.edit_directions.clear();
    st.edit_original_name = None;
    if let Some(win) = window_weak.upgrade() {
        win.set_edit_dialog_title(SharedString::from("添加手势"));
        win.set_edit_gesture_name(SharedString::from(""));
        win.set_edit_direction_display(SharedString::from(""));
        win.set_edit_has_directions(false);
        win.set_edit_capturing(false);
        win.set_edit_action_type_index(0);
        win.set_edit_action_detail(SharedString::from(""));
        win.set_edit_window_command_index(0);
        win.set_edit_dialog_visible(true);
    }
});
```

**Step 6: 更新 `edit-gesture-clicked` 回调**

编辑时加载手势名称和方向序列：

```rust
window.on_edit_gesture_clicked(move |idx: i32| {
    let (directions, original_name, entry_display_name, display, type_idx, detail, window_cmd_idx) = {
        let st = state_cb.borrow();
        let pairs = st.gesture_pairs();
        if idx < 0 || (idx as usize) >= pairs.len() { return; }
        let (name, entry) = &pairs[idx as usize];
        let dirs: Vec<String> = name.split(" → ").map(|s| s.trim().to_string()).collect();
        let disp = gesture_name_to_mnemonic(name);
        let t_idx = action_to_type_index(&entry.action);
        let det = action_to_detail(&entry.action);
        let wc_idx = match &entry.action {
            Action::Window(w) => window_command_to_index(&w.command),
            _ => 0,
        };
        (dirs, name.clone(), entry.name.clone(), disp, t_idx, det, wc_idx)
    };

    {
        let mut st = state_cb.borrow_mut();
        st.edit_directions = directions;
        st.edit_original_name = Some(original_name);
    }

    if let Some(win) = window_weak.upgrade() {
        win.set_edit_dialog_title(SharedString::from("编辑手势"));
        win.set_edit_gesture_name(SharedString::from(entry_display_name.as_str()));
        win.set_edit_direction_display(SharedString::from(display.as_str()));
        win.set_edit_has_directions(true);
        win.set_edit_capturing(false);
        win.set_edit_action_type_index(type_idx);
        win.set_edit_action_detail(SharedString::from(detail.as_str()));
        win.set_edit_window_command_index(window_cmd_idx);
        win.set_edit_dialog_visible(true);
    }
});
```

**Step 7: 添加 `edit-capture-clicked` 回调**

```rust
let window_weak_capture = window.as_weak();
window.on_edit_capture_clicked(move || {
    if let Some(win) = window_weak_capture.upgrade() {
        let currently_capturing = win.get_edit_capturing();
        if currently_capturing {
            // Cancel capture
            crate::core::capture::cancel_capture();
            win.set_edit_capturing(false);
        } else {
            // Start capture
            crate::core::capture::start_capture();
            win.set_edit_capturing(true);
        }
    }
});
```

**Step 8: 添加 Slint Timer 轮询捕捉结果**

在 `run_settings_window` 中，在 `setup_callbacks` 之后添加：

```rust
// Timer to poll for gesture capture results
let capture_window_weak = window.as_weak();
let capture_timer = slint::Timer::default();
capture_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(100), move || {
    if let Some(win) = capture_window_weak.upgrade() {
        if !win.get_edit_dialog_visible() || !win.get_edit_capturing() {
            return;
        }

        if let Some(dirs) = crate::core::capture::take_capture_result() {
            // Convert direction names to display format
            let display = gesture_name_to_mnemonic(&dirs.join(" → "));
            win.set_edit_direction_display(SharedString::from(display.as_str()));
            win.set_edit_has_directions(true);
            win.set_edit_capturing(false);
        }
    }
});
```

注意：`capture_timer` 需要保持存活直到窗口关闭。将其保存在 setup_callbacks 返回值中或作为局部变量保持。

**Step 9: 更新 `edit-dialog-confirmed` 回调**

从窗口读取 gesture name：

```rust
window.on_edit_dialog_confirmed(move || {
    let (action_type_idx, action_detail, window_cmd_idx, gesture_name_input) = {
        if let Some(win) = window_weak.upgrade() {
            (
                win.get_edit_action_type_index(),
                win.get_edit_action_detail().to_string(),
                win.get_edit_window_command_index(),
                win.get_edit_gesture_name().to_string(),
            )
        } else { return; }
    };

    let mut st = state_cb.borrow_mut();
    if st.edit_directions.is_empty() { return; }

    let gesture_key = st.edit_directions.join(" → ");

    // Remove old gesture if name changed
    let old_name_to_remove = st.edit_original_name.as_ref().and_then(|original_name| {
        if original_name != &gesture_key { Some(original_name.clone()) } else { None }
    });
    if let Some(old_name) = old_name_to_remove {
        st.remove_gesture(&old_name);
    }

    // Build action (same as before)
    let new_action = /* ... existing action building code ... */;

    let entry = GestureEntry {
        name: gesture_name_input,
        action: new_action,
    };
    st.set_gesture_entry(gesture_key, entry);

    // ... refresh UI ...
});
```

需要在 `DialogState` 中添加 `set_gesture_entry` 方法：

```rust
fn set_gesture_entry(&mut self, gesture_name: String, entry: GestureEntry) {
    let app = &self.current_app;
    if app == "global" {
        self.config.global_gestures.insert(gesture_name, entry);
    } else {
        self.config.app_gestures.entry(app.clone()).or_default().insert(gesture_name, entry);
    }
    if let Err(e) = self.save_config() {
        error!("Auto-save failed: {}", e);
    }
}
```

**Step 10: 更新 `edit-dialog-cancelled` 回调**

添加 `cancel_capture()` 调用和清理 `edit_capturing` 状态：

```rust
window.on_edit_dialog_cancelled(move || {
    crate::core::capture::cancel_capture();
    let mut st = state_cb.borrow_mut();
    st.edit_directions.clear();
    st.edit_original_name = None;
    drop(st);
    if let Some(win) = window_weak.upgrade() {
        win.set_edit_has_directions(false);
        win.set_edit_capturing(false);
        win.set_edit_dialog_visible(false);
    }
});
```

**Step 11: 更新 `edit-clear-directions` 回调**

```rust
window.on_edit_clear_directions(move || {
    crate::core::capture::cancel_capture();
    let mut st = state_cb.borrow_mut();
    st.edit_directions.clear();
    drop(st);
    if let Some(win) = window_weak.upgrade() {
        win.set_edit_direction_display(SharedString::from(""));
        win.set_edit_has_directions(false);
        win.set_edit_capturing(false);
    }
});
```

### Part C: 更新其他回调

**Step 12: 更新 `action-type-changed` 和 `action-detail-changed`**

这两个回调从 `pairs[idx].1`（原来是 `&Action`）获取数据，现在变为 `&GestureEntry`，需要通过 `.action` 访问：

```rust
let (_, entry) = &pairs[selected_idx as usize];
let detail = action_to_detail(&entry.action);
let type_idx = action_to_type_index(&entry.action);
```

并更新 `set_gesture` 调用为使用新的 `set_gesture_entry`：

```rust
// 构建 GestureEntry，保留原有的 name
let new_entry = GestureEntry {
    name: entry.name.clone(),
    action: new_action,
};
st.set_gesture_entry(gesture_name.clone(), new_entry);
```

**Step 13: 更新 `remove-gesture-clicked`**

无需修改，`remove_gesture` 方法只接收 key 字符串。

**Step 14: 运行编译和测试**

Run: `cargo build`
Expected: 编译成功

**Step 15: Commit**

```bash
git add src/ui/config_dialog.rs
git commit -m "feat: wire up gesture name field and capture mode in config dialog"
```

---

## Task 9: 移除不再使用的代码

**Files:**
- Modify: `src/ui/gesture_app.slint` (移除 `DirButton` 组件如果不再使用)
- Modify: `src/ui/config_dialog.rs` (移除 `edit-direction-clicked` 回调注册)

**Step 1: 清理**

- 移除 `DirButton` 组件定义（如果确认不再使用）
- 移除 `on_edit_direction_clicked` 回调注册
- 移除 `direction_index_to_name` 函数
- 移除旧的 `edit-direction-clicked` callback 属性

**Step 2: 编译测试**

Run: `cargo build`
Expected: 编译成功，无 warning

**Step 3: Commit**

```bash
git add -u
git commit -m "chore: remove unused direction button code and callbacks"
```

---

## Task 10: 端到端测试

**Step 1: 编译运行**

Run: `cargo build && cargo run`

**Step 2: 手动测试流程**

1. 打开设置窗口
2. 点击"添加"按钮
3. 验证编辑对话框显示"手势名称"输入框
4. 输入手势名称（如"复制"）
5. 点击"开始捕捉"按钮，验证按钮变为红色"捕捉中..."
6. 用鼠标中键（或配置的触发键）画一个方向手势
7. 验证序列自动填入显示区
8. 选择操作类型和参数
9. 点击确定，验证手势出现在列表中，名称显示正确
10. 点击"编辑"按钮，验证名称和序列正确加载
11. 重新捕捉一个不同的手势，验证更新

**Step 3: 验证配置文件**

检查 `%APPDATA%\RustGesture\config.json`：
- 新添加的手势应包含 `name` 字段
- 旧的没有 `name` 的手势应正常加载（name 默认为空）

**Step 4: Commit (if any fixes)**

```bash
git add -u
git commit -m "fix: any issues found during end-to-end testing"
```
