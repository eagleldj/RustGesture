# 手势轨迹覆盖层 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现 WGestures 风格的鼠标手势轨迹显示和手势名称提示。

**Architecture:** 新增透明覆盖层窗口模块，通过 mpsc 通道接收绘制命令。在 PathTracker 中添加位置更新事件，recognizer 回调中发送 overlay 命令。使用 GDI + UpdateLayeredWindow 绘制。

**Tech Stack:** Windows GDI, WS_EX_LAYERED 窗口, mpsc 通道, serde 配置序列化

---

### Task 1: Settings 新增配置字段

**Files:**
- Modify: `src/config/config.rs:194-224` (Settings struct + Default impl)

**Step 1: 在 Settings 中新增字段**

在 `Settings` struct 中添加：
```rust
pub show_trail: bool,
pub show_gesture_name: bool,
pub trail_width: u32,
pub trail_color_right: String,
pub trail_color_middle: String,
pub trail_color_x: String,
pub trail_color_unknown: String,
```

Default impl 中添加：
```rust
show_trail: true,
show_gesture_name: true,
trail_width: 3,
trail_color_right: "#0096FF".to_string(),
trail_color_middle: "#00CC66".to_string(),
trail_color_x: "#FF8800".to_string(),
trail_color_unknown: "#6633CC".to_string(),
```

**Step 2: 编译验证**

Run: `cargo build --lib`
Expected: 编译通过

**Step 3: Commit**

```bash
git add src/config/config.rs
git commit -m "feat(config): 添加手势轨迹和名称显示的配置字段"
```

---

### Task 2: 创建 Overlay 模块

**Files:**
- Create: `src/winapi/overlay.rs`
- Modify: `src/winapi/mod.rs`

overlay.rs 完整实现包含以下部分：

**2a: 数据类型**

```rust
use std::sync::mpsc::{self, Receiver, Sender};
use tracing::{error, info};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;

/// 绘制命令
pub enum OverlayCommand {
    StartTrail { x: i32, y: i32, color: u32, width: u32 },
    TrailPoint { x: i32, y: i32 },
    ShowName { name: String, x: i32, y: i32 },
    Clear,
    Shutdown,
}

struct OverlayState {
    points: Vec<(i32, i32)>,
    color: u32,
    width: u32,
    name: Option<(String, i32, i32)>,
    fade_alpha: u8,
}
```

**2b: 公共 API**

```rust
pub struct GestureOverlay {
    sender: Sender<OverlayCommand>,
}

impl GestureOverlay {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("gesture-overlay".into())
            .spawn(move || overlay_thread_main(rx))
            .expect("Failed to spawn overlay thread");
        Self { sender: tx }
    }

    pub fn send(&self, cmd: OverlayCommand) {
        let _ = self.sender.send(cmd);
    }
}
```

**2c: overlay 线程主函数**

```rust
fn overlay_thread_main(rx: Receiver<OverlayCommand>) {
    // 创建透明窗口
    // 进入消息循环，同时用 PeekMessageW + 非阻塞 recv 处理命令
    // 收到 TrailPoint 时重绘
    // 收到 ShowName 时启动淡出定时器
}
```

**2d: 窗口创建**

关键 Windows API 调用：
- `RegisterClassW` + `CreateWindowExW` 使用 `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW`
- 窗口大小 = 屏幕尺寸 (GetSystemMetrics SM_CXSCREEN/SM_CYSCREEN)
- DPI 感知：用 `GetDeviceCaps(LOGPIXELSX)` 获取缩放

**2e: GDI 绘制核心函数**

```rust
fn render(hwnd: HWND, mem_dc: HDC, bitmap: HBITMAP, bits: *mut u8, w: i32, h: i32, state: &OverlayState) {
    // 1. 清零 bitmap（全透明）
    // 2. SelectObject(bitmap) 到 mem_dc
    // 3. 创建 Pen(PS_SOLID | PS_ENDCAP_ROUND, width, color)
    // 4. 遍历 points 画折线 (MoveToEx + LineTo)
    // 5. 如果有 name，画圆角矩形背景 + 文字
    // 6. 计算轨迹 bounding box，只对该区域设置 alpha（预乘）
    // 7. UpdateLayeredWindow 提交
}
```

**2f: 淡出动画**

用 `SetTimer` 启动定时器（50ms），每步减少 alpha 约 32，8 步后清除。

**Step: 在 mod.rs 中注册模块**

添加 `pub mod overlay;`

**Step: 编译验证**

Run: `cargo build --lib`

**Step: Commit**

```bash
git add src/winapi/overlay.rs src/winapi/mod.rs
git commit -m "feat(overlay): 创建透明覆盖层窗口模块"
```

---

### Task 3: Recognizer 新增位置更新事件

**Files:**
- Modify: `src/core/tracker.rs` — 在 `on_mouse_move` 中 emit `TrackerEvent::PositionUpdate`
- Modify: `src/core/recognizer.rs` — 转发为 `GestureRecognizerEvent::PositionUpdate`

**Step 1: TrackerEvent 新增变体**

在 `TrackerEvent` 枚举中添加：
```rust
PositionUpdate(Point),
```

在 `PathTracker::on_mouse_move` 的 `Tracking` 状态末尾（`self.last_point = Some(current_point);` 之前）添加：
```rust
self.emit_event(TrackerEvent::PositionUpdate(current_point));
```

**Step 2: GestureRecognizerEvent 新增变体**

在 `GestureRecognizerEvent` 枚举中添加：
```rust
PositionUpdate(crate::core::gesture::Point),
```

在 recognizer 的 `set_event_callback` match 中添加：
```rust
TrackerEvent::PositionUpdate(p) => GestureRecognizerEvent::PositionUpdate(p),
```

**Step 3: 编译验证**

Run: `cargo build --lib`

**Step 4: Commit**

```bash
git add src/core/tracker.rs src/core/recognizer.rs
git commit -m "feat(recognizer): 添加鼠标位置更新事件用于轨迹绘制"
```

---

### Task 4: 集成 Overlay 到应用

**Files:**
- Modify: `src/core/app.rs` — GestureApp 持有 Overlay，回调中发送命令
- Modify: `src/main.rs` — GestureOverlay 的创建和生命周期

**Step 1: 修改 GestureApp**

在 `GestureApp` struct 中添加：
```rust
overlay: crate::winapi::overlay::GestureOverlay,
```

在 `GestureApp::new()` 中创建 overlay，在事件回调中：
- `GestureStarted` → `overlay.send(StartTrail { x, y, color, width })` (根据 trigger_button 选色)
- `PositionUpdate(p)` → `overlay.send(TrailPoint { x: p.x, y: p.y })`
- `GestureCompleted` → 查找匹配的 entry，`overlay.send(ShowName { name, x, y })`
- `GestureCancelled` → `overlay.send(Clear)`

**Step 2: 修改 main.rs**

GestureOverlay 随 GestureApp 创建和销毁，无需在 main.rs 中单独管理。

**Step 3: 编译验证**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/core/app.rs src/main.rs
git commit -m "feat(app): 集成手势轨迹覆盖层到应用"
```

---

### Task 5: 编译和手动测试

**Step 1: 完整编译**

Run: `cargo build`

**Step 2: 手动测试**

1. 运行程序
2. 用中键画手势 → 应看到青蓝色轨迹线
3. 画完手势 → 应看到手势名称提示
4. 名称应在约 1.5 秒后淡出
5. 右键手势 → 应看到蓝色轨迹
6. 修改 config.json 中的颜色/线宽 → 重启后生效

**Step 3: Final Commit**

```bash
git add -A
git commit -m "feat: 实现 WGestures 风格手势轨迹显示和名称提示"
```
