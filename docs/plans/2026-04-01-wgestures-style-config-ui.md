# WGestures 风格配置界面实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 用 Slint UI 框架替换现有 Win32 配置对话框，实现 WGestures 风格的手势管理界面（左侧应用列表 + 右侧手势表格 + 底部参数区）。

**Architecture:** Slint UI 在独立线程中运行，通过 channel 与 Win32 钩子/托盘线程通信。托盘菜单"设置"发送信号，主线程启动 Slint 窗口。Slint 通过 Rust 回调读写 config.json。

**Tech Stack:** Rust + Slint UI (slint crate v1.x + slint-build) + 现有 serde_json 配置

---

### Task 1: 添加 Slint 依赖和构建配置

**Files:**
- Modify: `Cargo.toml`
- Create: `build.rs`

**Step 1: 添加 Slint 依赖到 Cargo.toml**

在 `[dependencies]` 中添加 `slint`，在 `[build-dependencies]` 中添加 `slint-build`：

```toml
[dependencies]
slint = "1.9"
# ... 现有依赖保留

[build-dependencies]
slint-build = "1.9"
```

**Step 2: 创建 build.rs**

```rust
fn main() {
    slint_build::compile_with_config(
        "src/ui/gesture_app.slint",
        slint_build::CompilerConfiguration::new()
            .with_style("fluent-dark".into()),
    )
    .expect("Slint build failed");
}
```

**Step 3: 验证构建配置**

Run: `cargo check`
Expected: 编译成功（此时 .slint 文件还不存在，会有错误，这是预期的）

**Step 4: Commit**

```bash
git add Cargo.toml build.rs
git commit -m "build: add Slint UI framework dependencies"
```

---

### Task 2: 创建 Slint UI 文件

**Files:**
- Create: `src/ui/gesture_app.slint`

**Step 1: 创建手势管理界面的 Slint 定义**

```slint
import { Button, VerticalBox, HorizontalBox, LineEdit, ComboBox, GroupBox, StandardListView, TabWidget, ListView, GridBox } from "std-widgets.slint";

// 数据结构
export struct AppItem in {
    name: string,
    icon: string,
}

export struct GestureItem in {
    name: string,
    mnemonic: string,
    action: string,
}

export struct ActionParameter in {
    action-type: string,
    action-value: string,
    execute-immediately: bool,
}

export component GestureAppWindow inherits Window {
    title: "RustGesture 设置";
    preferred-width: 750px;
    preferred-height: 520px;
    min-width: 600px;
    min-height: 400px;

    // 数据属性
    in property <[AppItem]> app-list: [];
    in-out property <int> selected-app-index: 0;
    in property <[GestureItem]> gesture-list: [];
    in-out property <int> selected-gesture-index: -1;

    // 参数区域
    in property <string> param-title: "选择一个手势查看参数";
    in property <[string]> action-type-options: ["键盘快捷键", "鼠标动作", "窗口命令", "运行程序"];
    in-out property <int> selected-action-type: 0;
    in property <string> action-value: "";
    in-out property <bool> execute-immediately: false;
    in property <bool> param-enabled: false;

    // 提示
    in property <string> status-text: "* 改动将自动保存并立即生效";

    // 回调
    callback app-selected(int);
    callback gesture-selected(int);
    callback add-gesture();
    callback remove-gesture(int);
    callback edit-gesture(int);
    callback action-type-changed(int);
    callback action-value-changed(string);
    callback add-app(string);

    // === 布局 ===
    HorizontalLayout {
        padding: 0px;
        spacing: 0px;

        // 左侧边栏 - 应用列表
        VerticalLayout {
            width: 160px;
            min-width: 140px;
            background: #ffffff;
            padding: 4px;
            spacing: 2px;

            // 应用列表标题
            HorizontalLayout {
                height: 28px;
                padding-left: 8px;
                alignment: center;
                Text {
                    text: "应用程序";
                    font-size: 12px;
                    font-weight: 700;
                    color: #333333;
                    vertical-alignment: center;
                }
            }

            // 应用列表
            ListView {
                selected: selected-app-index;
                selection-mode: single;
                model: app-list;

                current-value-changed => {
                    selected-app-index = self.current-value.idx;
                    root.app-selected(self.current-value.idx);
                }

                delegate := HorizontalLayout {
                    padding: 6px;
                    padding-left: 10px;
                    spacing: 8px;
                    height: 30px;

                    background: touched ? #e8e8e8 : (root.selected-app-index == idx ? #f0f0f0 : #ffffff);

                    Text {
                        text: model.name;
                        font-size: 12px;
                        color: #333333;
                        vertical-alignment: center;
                    }

                    touch := TouchArea {
                        clicked => {
                            root.selected-app-index = idx;
                            root.app-selected(idx);
                        }
                    }
                };
            }

            // 底部添加按钮
            HorizontalLayout {
                height: 32px;
                padding: 4px;
                alignment: center;

                Button {
                    text: "+ 添加应用";
                    font-size: 11px;
                    clicked => {
                        root.add-app("");
                    }
                }
            }

            Rectangle { vertical-stretch: 0; height: 1px; background: #d0d0d0; }
        }

        // 分隔线
        Rectangle {
            width: 1px;
            background: #d0d0d0;
        }

        // 右侧内容区
        VerticalLayout {
            padding: 12px;
            spacing: 8px;
            background: #fafafa;

            // 手势列表标题栏
            HorizontalLayout {
                height: 30px;
                alignment: space-between;

                Text {
                    text: "手势列表";
                    font-size: 13px;
                    font-weight: 700;
                    color: #333333;
                    vertical-alignment: center;
                }
            }

            // 手势表格（使用 ListView 模拟表格）
            Rectangle {
                vertical-stretch: 1;
                border-width: 1px;
                border-color: #d0d0d0;
                background: #ffffff;
                clip: true;

                VerticalLayout {
                    padding: 0px;
                    spacing: 0px;

                    // 表头
                    HorizontalLayout {
                        height: 28px;
                        spacing: 0px;
                        background: #f0f0f0;

                        Rectangle { width: 30px; Text { text: "#"; font-size: 11px; font-weight: 700; color: #666; horizontal-alignment: center; vertical-alignment: center; } }
                        Rectangle { width: 1px; background: #d0d0d0; }
                        Rectangle { horizontal-stretch: 2; Text { text: "  名称"; font-size: 11px; font-weight: 700; color: #666; vertical-alignment: center; } }
                        Rectangle { width: 1px; background: #d0d0d0; }
                        Rectangle { horizontal-stretch: 1; Text { text: "  助记符"; font-size: 11px; font-weight: 700; color: #666; vertical-alignment: center; } }
                        Rectangle { width: 1px; background: #d0d0d0; }
                        Rectangle { horizontal-stretch: 2; Text { text: "  操作"; font-size: 11px; font-weight: 700; color: #666; vertical-alignment: center; } }
                    }

                    Rectangle { height: 1px; background: #d0d0d0; }

                    // 手势列表
                    ListView {
                        vertical-stretch: 1;
                        selected: selected-gesture-index;
                        model: gesture-list;

                        current-value-changed => {
                            selected-gesture-index = self.current-value.idx;
                            root.gesture-selected(self.current-value.idx);
                        }

                        delegate := HorizontalLayout {
                            height: 28px;
                            spacing: 0px;
                            padding: 0px;

                            background: touch-area.has-hover ? #f0f0f0 : (root.selected-gesture-index == idx ? #e8f0fe : #ffffff);

                            Rectangle {
                                width: 30px;
                                Text {
                                    text: idx + 1;
                                    font-size: 11px;
                                    color: #999;
                                    horizontal-alignment: center;
                                    vertical-alignment: center;
                                }
                            }
                            Rectangle { width: 1px; background: #e8e8e8; }
                            Rectangle {
                                horizontal-stretch: 2;
                                Text {
                                    text: "  " + model.name;
                                    font-size: 12px;
                                    color: #333;
                                    vertical-alignment: center;
                                }
                            }
                            Rectangle { width: 1px; background: #e8e8e8; }
                            Rectangle {
                                horizontal-stretch: 1;
                                Text {
                                    text: "  " + model.mnemonic;
                                    font-size: 13px;
                                    color: #555;
                                    vertical-alignment: center;
                                }
                            }
                            Rectangle { width: 1px; background: #e8e8e8; }
                            Rectangle {
                                horizontal-stretch: 2;
                                Text {
                                    text: "  " + model.action;
                                    font-size: 12px;
                                    color: #555;
                                    vertical-alignment: center;
                                }
                            }

                            touch-area := TouchArea {
                                clicked => {
                                    root.selected-gesture-index = idx;
                                    root.gesture-selected(idx);
                                }
                            }
                        };
                    }
                }
            }

            // 操作按钮栏
            HorizontalLayout {
                height: 32px;
                spacing: 8px;
                alignment: left;

                Button {
                    text: "+ 添加";
                    font-size: 11px;
                    width: 70px;
                    clicked => { root.add-gesture(); }
                }
                Button {
                    text: "- 删除";
                    font-size: 11px;
                    width: 70px;
                    enabled: root.selected-gesture-index >= 0;
                    clicked => { root.remove-gesture(root.selected-gesture-index); }
                }
                Button {
                    text: "编辑";
                    font-size: 11px;
                    width: 70px;
                    enabled: root.selected-gesture-index >= 0;
                    clicked => { root.edit-gesture(root.selected-gesture-index); }
                }
            }

            // 参数设置区
            Rectangle {
                height: 120px;
                border-width: 1px;
                border-color: #d0d0d0;
                background: #ffffff;
                visible: root.param-enabled;

                VerticalLayout {
                    padding: 10px;
                    spacing: 6px;

                    Text {
                        text: root.param-title;
                        font-size: 12px;
                        font-weight: 700;
                        color: #333;
                    }

                    HorizontalLayout {
                        height: 28px;
                        spacing: 8px;
                        alignment: left;

                        Text {
                            text: "执行操作:";
                            font-size: 12px;
                            color: #555;
                            vertical-alignment: center;
                            width: 70px;
                        }

                        ComboBox {
                            model: root.action-type-options;
                            current-index: root.selected-action-type;
                            current-index-changed(new-index) => {
                                root.selected-action-type = new-index;
                                root.action-type-changed(new-index);
                            }
                            enabled: root.param-enabled;
                            width: 160px;
                        }
                    }

                    LineEdit {
                        text: root.action-value;
                        edited(new-text) => {
                            root.action-value = new-text;
                            root.action-value-changed(new-text);
                        }
                        enabled: root.param-enabled;
                        height: 28px;
                    }
                }
            }

            // 底部状态文字
            Text {
                text: root.status-text;
                font-size: 11px;
                color: #999;
            }
        }
    }
}
```

**Step 2: 验证 Slint 文件语法**

Run: `cargo build 2>&1 | head -30`
Expected: Slint 编译通过（可能有 Rust 侧的类型错误，因为还没写 Rust 代码）

**Step 3: Commit**

```bash
git add src/ui/gesture_app.slint
git commit -m "ui: add WGestures-style Slint UI definition"
```

---

### Task 3: 重写 config_dialog.rs 使用 Slint

**Files:**
- Modify: `src/ui/config_dialog.rs` (完全重写)

**Step 1: 重写 config_dialog.rs**

新的 `config_dialog.rs` 需要：
1. 通过 `slint::include_modules!()` 引入编译后的 Slint 模块
2. 将 `GestureConfig` 数据转换为 Slint 模型
3. 设置回调处理用户操作（增删改手势、切换应用、保存配置）
4. 在独立线程中运行 Slint 事件循环

```rust
//! Configuration dialog - Slint UI based settings window

use slint::SharedString;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{info, warn, error};

use crate::config::config::{GestureConfig, Action};

// Import Slint compiled module
slint::include_modules!();

/// Configuration dialog using Slint UI
pub struct ConfigDialog {
    config_path: PathBuf,
}

impl ConfigDialog {
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// Show the configuration window (blocks until closed)
    pub fn show(&self) {
        let config_path = self.config_path.clone();

        // Load config
        let config = match Self::load_config(&config_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to load config: {}", e);
                return;
            }
        };

        // Create Slint window
        let window = GestureAppWindow::new().expect("Failed to create Slint window");

        // Populate app list
        let app_items = Self::build_app_list(&config);
        window.set_app_list(app_items.into());

        // Populate gesture list for "Global"
        let gesture_items = Self::build_gesture_list(&config.global_gestures);
        window.set_gesture_list(gesture_items.into());

        // Clone for callbacks
        let window_weak = window.as_weak();
        let config_arc = Arc::new(Mutex::new(config));
        let config_path_cb = config_path.clone();

        // App selection callback
        let config_for_app = config_arc.clone();
        let window_for_app = window.as_weak();
        window.on_app_selected(move |idx| {
            let idx = idx as usize;
            let config = config_for_app.lock().unwrap();
            let gestures = if idx == 0 {
                &config.global_gestures
            } else {
                let apps: Vec<&String> = config.app_gestures.keys().collect();
                if idx > apps.len() { return; }
                match apps.get(idx - 1) {
                    Some(app_name) => config.app_gestures.get(*app_name).unwrap_or(&config.global_gestures),
                    None => &config.global_gestures,
                }
            };
            let items = Self::build_gesture_list(gestures);
            if let Some(w) = window_for_app.upgrade() {
                w.set_gesture_list(items.into());
                w.set_selected_gesture_index(-1);
                w.set_param_enabled(false);
            }
        });

        // Gesture selection callback - show parameters
        let config_for_gesture = config_arc.clone();
        let window_for_gesture = window.as_weak();
        window.on_gesture_selected(move |idx| {
            let idx = idx as usize;
            if idx < 0 { return; }
            let config = config_for_gesture.lock().unwrap();
            let gesture_list = Self::build_gesture_list(&config.global_gestures);
            if idx >= gesture_list.len() { return; }
            let item = &gesture_list[idx];

            if let Some(w) = window_for_gesture.upgrade() {
                w.set_param_title(format!("手势 '{}' 的参数", item.name).into());
                w.set_param_enabled(true);
                w.set_action_value(item.action.clone().into());
                // Set action type based on action
                // ... (will be implemented in detail)
            }
        });

        // Add gesture callback
        let config_for_add = config_arc.clone();
        let window_for_add = window.as_weak();
        window.on_add_gesture(move || {
            // TODO: open gesture edit dialog
            info!("Add gesture requested");
        });

        // Remove gesture callback
        let config_for_remove = config_arc.clone();
        let window_for_remove = window.as_weak();
        window.on_remove_gesture(move |idx| {
            let idx = idx as usize;
            let mut config = config_for_remove.lock().unwrap();
            // Remove from current gesture list
            // ... save and refresh
            info!("Remove gesture at index {}", idx);
        });

        // Edit gesture callback
        window.on_edit_gesture(move |idx| {
            info!("Edit gesture at index {}", idx);
        });

        // Action type changed callback
        let config_for_action = config_arc.clone();
        window.on_action_type_changed(move |type_idx| {
            info!("Action type changed to {}", type_idx);
        });

        // Action value changed callback - auto save
        let config_for_value = config_arc.clone();
        let path_for_value = config_path_cb.clone();
        window.on_action_value_changed(move |value| {
            info!("Action value changed: {}", value);
            // Auto-save on change
            let mut config = config_for_value.lock().unwrap();
            // Update the action value...
            if let Err(e) = Self::save_config(&config, &path_for_value) {
                error!("Failed to auto-save: {}", e);
            }
        });

        // Add app callback
        window.on_add_app(move |_name| {
            info!("Add app requested");
        });

        // Run the Slint event loop (blocking)
        window.run().expect("Slint event loop failed");
    }

    fn load_config(path: &PathBuf) -> anyhow::Result<GestureConfig> {
        let content = std::fs::read_to_string(path)?;
        let config: GestureConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    fn save_config(config: &GestureConfig, path: &PathBuf) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(config)?;
        std::fs::write(path, json)?;
        info!("Configuration saved");
        Ok(())
    }

    fn build_app_list(config: &GestureConfig) -> Vec<AppItem> {
        let mut items = vec![AppItem {
            name: SharedString::from("全局"),
            icon: SharedString::from("🌐"),
        }];
        for app_name in config.app_gestures.keys() {
            items.push(AppItem {
                name: SharedString::from(app_name.as_str()),
                icon: SharedString::from("📱"),
            });
        }
        items
    }

    fn build_gesture_list(gestures: &HashMap<String, Action>) -> Vec<GestureItem> {
        gestures.iter().map(|(name, action)| {
            let mnemonic = Self::gesture_name_to_mnemonic(name);
            GestureItem {
                name: SharedString::from(name.as_str()),
                mnemonic: SharedString::from(mnemonic),
                action: SharedString::from(action.display_info()),
            }
        }).collect()
    }

    /// Convert gesture name like "Right → Down" to mnemonic "→↓"
    fn gesture_name_to_mnemonic(name: &str) -> String {
        name.split(" → ")
            .map(|dir| {
                match dir.trim() {
                    "Up" => "↑",
                    "Down" => "↓",
                    "Left" => "←",
                    "Right" => "→",
                    "UpLeft" => "↖",
                    "UpRight" => "↗",
                    "DownLeft" => "↙",
                    "DownRight" => "↘",
                    other => other,
                }
            })
            .collect()
    }
}
```

**Step 2: 验证编译**

Run: `cargo build 2>&1`
Expected: 编译通过（可能有少量类型不匹配需要修复）

**Step 3: Commit**

```bash
git add src/ui/config_dialog.rs
git commit -m "feat(ui): rewrite config dialog using Slint UI framework"
```

---

### Task 4: 更新 tray.rs 集成

**Files:**
- Modify: `src/ui/tray.rs`
- Modify: `src/ui/mod.rs`

**Step 1: 更新 tray.rs 中的设置对话框调用**

关键变化：
- `ConfigDialog::show()` 不再需要 `parent_hwnd` 参数
- 在 `std::thread::spawn` 中启动 Slint UI，避免阻塞消息循环线程

在 `tray.rs` 中修改两处调用 `ConfigDialog` 的地方：
1. `show_context_menu` 中的菜单项 3（Settings）
2. `window_proc` 中的双击事件

改为在新线程中启动：
```rust
let config_path = self.config_path.clone();
std::thread::spawn(move || {
    let dialog = ConfigDialog::new(config_path);
    dialog.show();
});
```

**Step 2: 更新 mod.rs（如果需要）**

确认 `mod.rs` 正确导出模块，可能需要添加 Slint 相关声明。

**Step 3: 验证编译**

Run: `cargo build 2>&1`
Expected: 编译通过

**Step 4: Commit**

```bash
git add src/ui/tray.rs src/ui/mod.rs
git commit -m "refactor(tray): launch Slint config dialog in separate thread"
```

---

### Task 5: 构建验证和修复

**Step 1: 完整构建**

Run: `cargo build 2>&1`
Expected: 编译成功

**Step 2: 修复所有编译错误**

根据编译输出修复：
- Slint 类型不匹配
- 回调签名问题
- 缺少的 import

**Step 3: 最终 Commit**

```bash
git add -A
git commit -m "fix(ui): resolve Slint integration compilation issues"
```

---

## 注意事项

1. **Slint 线程安全**: Slint 组件不是 Send/Sync，必须在创建它的线程中使用 `Weak::upgrade_in_thread` 从其他线程更新
2. **自动保存**: WGestures 风格是"改动自动保存"，在 `action_value_changed` 回调中触发保存
3. **助记符转换**: 手势名称 "Right → Down" 需要转换为 "→↓" 符号
4. **Slint 样式**: 使用 `fluent-dark` 或默认样式，可在 build.rs 中切换
