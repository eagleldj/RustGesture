//! Configuration dialog - Slint UI based settings window
//!
//! Uses a persistent UI thread to avoid "platform initialized in another thread" errors.
//! The Slint platform is initialized once and reused for all dialog show/hide cycles.

use slint::SharedString;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use tracing::{error, info};

use crate::config::config::{
    Action, GestureConfig, GestureEntry, KeyboardAction, MouseAction, MouseActionType, MouseButton,
    RunAction, WindowAction, WindowCommand,
};

// Import the compiled Slint module generated from gesture_app.slint
slint::include_modules!();

// ---------- Persistent UI thread infrastructure ----------

enum UiCommand {
    ShowSettings { config_path: PathBuf },
    Shutdown,
}

/// Lazily spawn the persistent Slint UI thread and return its command sender.
/// The thread is created once and stays alive for the entire application lifetime.
fn ui_sender() -> &'static Sender<UiCommand> {
    use std::sync::OnceLock;
    static SENDER: OnceLock<Sender<UiCommand>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("slint-ui".into())
            .spawn(move || ui_thread_main(rx))
            .expect("Failed to spawn Slint UI thread");
        tx
    })
}

/// Main loop for the persistent UI thread.
/// Waits for commands and creates/shows Slint windows as needed.
fn ui_thread_main(rx: Receiver<UiCommand>) {
    info!("Slint UI thread started");
    loop {
        match rx.recv() {
            Ok(UiCommand::ShowSettings { config_path }) => {
                info!("UI thread: creating settings window");
                run_settings_window(&config_path);
                info!("UI thread: settings window closed");
            }
            Ok(UiCommand::Shutdown) | Err(_) => {
                info!("Slint UI thread shutting down");
                break;
            }
        }
    }
}

// ---------- Helper functions ----------

fn vec_to_model<T: Clone + 'static>(items: Vec<T>) -> slint::ModelRc<T> {
    let model: Rc<slint::VecModel<T>> = Rc::new(slint::VecModel::from(items));
    model.into()
}

/// Convert a gesture name like "Right -> Down" to a mnemonic arrow string like "→↓"
fn gesture_name_to_mnemonic(name: &str) -> String {
    let dir_map: HashMap<&str, &str> = [
        ("Up", "\u{2191}"),
        ("Down", "\u{2193}"),
        ("Left", "\u{2190}"),
        ("Right", "\u{2192}"),
        ("UpLeft", "\u{2196}"),
        ("UpRight", "\u{2197}"),
        ("DownLeft", "\u{2199}"),
        ("DownRight", "\u{2198}"),
    ]
    .iter()
    .cloned()
    .collect();

    let button_map: HashMap<&str, &str> = [("M_", "M"), ("R_", "R"), ("X1_", "X1"), ("X2_", "X2")]
        .iter()
        .cloned()
        .collect();

    // Extract button prefix and directions
    let (button_label, rest) = button_map
        .iter()
        .find(|(prefix, _)| name.starts_with(*prefix))
        .map(|(prefix, label)| (*label, &name[prefix.len()..]))
        .unwrap_or(("", name));

    let arrows: String = rest
        .split(" → ")
        .map(|part| {
            dir_map
                .get(part.trim())
                .map(|s| s.to_string())
                .unwrap_or_else(|| part.trim().to_string())
        })
        .collect::<Vec<_>>()
        .join("");

    if button_label.is_empty() {
        arrows
    } else {
        format!("{} {}", button_label, arrows)
    }
}

/// Parse a gesture key into trigger button and directions.
/// "M_Right → Down" → (Middle, ["Right", "Down"])
/// "R_Up" → (Right, ["Up"])
/// "Up" (legacy, no prefix) → (Middle, ["Up"])
fn parse_gesture_key(key: &str) -> (crate::core::gesture::GestureTriggerButton, Vec<String>) {
    use crate::core::gesture::GestureTriggerButton;

    let prefixes: [(&str, GestureTriggerButton); 4] = [
        ("M_", GestureTriggerButton::Middle),
        ("R_", GestureTriggerButton::Right),
        ("X1_", GestureTriggerButton::X1),
        ("X2_", GestureTriggerButton::X2),
    ];

    for (prefix, button) in &prefixes {
        if key.starts_with(prefix) {
            let rest = &key[prefix.len()..];
            let dirs: Vec<String> = rest
                .split(" → ")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return (*button, dirs);
        }
    }

    // Legacy format without prefix - default to Middle button
    let dirs: Vec<String> = key
        .split(" → ")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    (GestureTriggerButton::Middle, dirs)
}

fn action_to_type_index(action: &Action) -> i32 {
    match action {
        Action::Keyboard(_) => 0,
        Action::Mouse(_) => 1,
        Action::Window(_) => 2,
        Action::Run(_) => 3,
    }
}

/// Convert a single VK code to a friendly display name
fn vk_to_display_name(vk: &str) -> String {
    match vk {
        "VK_CONTROL" | "VK_LCONTROL" | "VK_RCONTROL" => "Ctrl".to_string(),
        "VK_MENU" | "VK_LMENU" | "VK_RMENU" => "Alt".to_string(),
        "VK_SHIFT" | "VK_LSHIFT" | "VK_RSHIFT" => "Shift".to_string(),
        "VK_LWIN" | "VK_RWIN" => "Win".to_string(),
        s if s.starts_with("VK_F") && s[5..].parse::<u32>().is_ok() => s[3..].to_string(), // VK_F1 → F1
        "VK_BACK" => "Backspace".to_string(),
        "VK_TAB" => "Tab".to_string(),
        "VK_RETURN" => "Enter".to_string(),
        "VK_ESCAPE" => "Esc".to_string(),
        "VK_SPACE" => "Space".to_string(),
        "VK_DELETE" => "Delete".to_string(),
        "VK_INSERT" => "Insert".to_string(),
        "VK_HOME" => "Home".to_string(),
        "VK_END" => "End".to_string(),
        "VK_PRIOR" => "PageUp".to_string(),
        "VK_NEXT" => "PageDown".to_string(),
        "VK_LEFT" => "←".to_string(),
        "VK_RIGHT" => "→".to_string(),
        "VK_UP" => "↑".to_string(),
        "VK_DOWN" => "↓".to_string(),
        "VK_CAPITAL" => "CapsLock".to_string(),
        "VK_NUMLOCK" => "NumLock".to_string(),
        "VK_SNAPSHOT" => "PrtScn".to_string(),
        "VK_SCROLL" => "ScrollLock".to_string(),
        "VK_PAUSE" => "Pause".to_string(),
        // Single character VK codes like "VK_A" → "A"
        s if s.starts_with("VK_") && s.len() == 4 => s[3..].to_string(),
        // Already friendly or unknown — return as-is
        s => s.to_string(),
    }
}

/// Format keyboard keys as friendly display string, e.g. "Ctrl + Alt + F1"
fn format_keys_display(keys: &[String]) -> String {
    keys.iter()
        .map(|k| vk_to_display_name(k))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Convert a Windows VK code (integer) to (display_name, vk_string)
fn vk_code_to_info(vk: i32) -> (String, String) {
    match vk as u8 {
        // A-Z (0x41-0x5A)
        b @ 0x41..=0x5A => {
            let ch = b as char;
            (ch.to_string(), format!("VK_{}", ch))
        }
        // 0-9 (0x30-0x39)
        b @ 0x30..=0x39 => {
            let ch = b as char;
            (ch.to_string(), format!("VK_{}", ch))
        }
        // F1-F12 (0x70-0x7B)
        b @ 0x70..=0x7B => {
            let n = b - 0x70 + 1;
            (format!("F{}", n), format!("VK_F{}", n))
        }
        // Special keys
        0x08 => ("Backspace".to_string(), "VK_BACK".to_string()),
        0x09 => ("Tab".to_string(), "VK_TAB".to_string()),
        0x0D => ("Enter".to_string(), "VK_RETURN".to_string()),
        0x1B => ("Esc".to_string(), "VK_ESCAPE".to_string()),
        0x20 => ("Space".to_string(), "VK_SPACE".to_string()),
        0x21 => ("PageUp".to_string(), "VK_PRIOR".to_string()),
        0x22 => ("PageDown".to_string(), "VK_NEXT".to_string()),
        0x23 => ("End".to_string(), "VK_END".to_string()),
        0x24 => ("Home".to_string(), "VK_HOME".to_string()),
        0x25 => ("←".to_string(), "VK_LEFT".to_string()),
        0x26 => ("↑".to_string(), "VK_UP".to_string()),
        0x27 => ("→".to_string(), "VK_RIGHT".to_string()),
        0x28 => ("↓".to_string(), "VK_DOWN".to_string()),
        0x2C => ("PrtScn".to_string(), "VK_SNAPSHOT".to_string()),
        0x2D => ("Insert".to_string(), "VK_INSERT".to_string()),
        0x2E => ("Delete".to_string(), "VK_DELETE".to_string()),
        // Numpad 0-9 (0x60-0x69)
        b @ 0x60..=0x69 => {
            let n = b - 0x60;
            (format!("Num{}", n), format!("VK_NUMPAD{}", n))
        }
        // Other: use hex code
        _ => (format!("0x{:02X}", vk), format!("VK_0x{:02X}", vk)),
    }
}

fn action_to_detail(action: &Action) -> String {
    match action {
        Action::Keyboard(kb) => kb.keys.join("+"),
        Action::Mouse(m) => {
            let action_str = match m.action_type {
                crate::config::config::MouseActionType::Click => "Click",
                crate::config::config::MouseActionType::DoubleClick => "DoubleClick",
            };
            format!("{} {}", m.button.as_str(), action_str)
        }
        Action::Window(w) => format!("{:?}", w.command),
        Action::Run(r) => {
            if let Some(args) = &r.args {
                format!("{} {}", r.command, args)
            } else {
                r.command.clone()
            }
        }
    }
}

fn type_index_to_display(index: i32) -> &'static str {
    match index {
        0 => "键盘快捷键",
        1 => "鼠标操作",
        2 => "窗口管理",
        3 => "运行程序",
        _ => "未知",
    }
}

/// Generate a concise Chinese description for the action type (used in gesture list "操作类型" column)
fn action_type_display(action: &Action) -> String {
    match action {
        Action::Keyboard(_) => "键盘快捷键".to_string(),
        Action::Mouse(_) => "鼠标操作".to_string(),
        Action::Window(_) => "窗口管理".to_string(),
        Action::Run(_) => "运行程序".to_string(),
    }
}

/// Generate the params display string for the gesture list "参数" column
fn action_params_display(action: &Action) -> String {
    match action {
        Action::Keyboard(kb) => format_keys_display(&kb.keys),
        Action::Mouse(m) => {
            let btn = match m.button {
                MouseButton::Left => "左键",
                MouseButton::Right => "右键",
                MouseButton::Middle => "中键",
                MouseButton::X1 => "X1键",
                MouseButton::X2 => "X2键",
            };
            let act = match m.action_type {
                MouseActionType::Click => "单击",
                MouseActionType::DoubleClick => "双击",
            };
            format!("{}{}", btn, act)
        }
        Action::Window(w) => match w.command {
            WindowCommand::Minimize => "最小化",
            WindowCommand::Maximize => "最大化",
            WindowCommand::Restore => "还原",
            WindowCommand::Close => "关闭窗口",
            WindowCommand::ShowDesktop => "显示桌面",
        }
        .to_string(),
        Action::Run(r) => {
            if let Some(args) = &r.args {
                format!("{} {}", r.command, args)
            } else {
                r.command.clone()
            }
        }
    }
}

fn window_command_to_index(cmd: &WindowCommand) -> i32 {
    match cmd {
        WindowCommand::Minimize => 0,
        WindowCommand::Maximize => 1,
        WindowCommand::Restore => 2,
        WindowCommand::Close => 3,
        WindowCommand::ShowDesktop => 4,
    }
}

fn index_to_window_command(idx: i32) -> WindowCommand {
    match idx {
        0 => WindowCommand::Minimize,
        1 => WindowCommand::Maximize,
        2 => WindowCommand::Restore,
        3 => WindowCommand::Close,
        4 => WindowCommand::ShowDesktop,
        _ => WindowCommand::Minimize,
    }
}

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

// ---------- Internal state ----------

struct DialogState {
    config_path: PathBuf,
    config: GestureConfig,
    current_app: String,
    app_names: Vec<String>,
    // Edit dialog state
    edit_directions: Vec<String>,
    edit_trigger_button: crate::core::gesture::GestureTriggerButton,
    edit_original_name: Option<String>,
}

impl DialogState {
    fn load_config(path: &PathBuf) -> anyhow::Result<GestureConfig> {
        let content = std::fs::read_to_string(path)?;
        let config: GestureConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    fn save_config(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(&self.config_path, json)?;
        info!("Configuration auto-saved");
        Ok(())
    }

    fn build_app_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.config.app_gestures.keys().cloned().collect();
        names.sort();
        names.insert(0, "global".to_string());
        names.dedup();
        names
    }

    fn gesture_map_for_app(&self, app_name: &str) -> HashMap<String, GestureEntry> {
        if app_name == "global" {
            self.config.global_gestures.clone()
        } else {
            self.config
                .app_gestures
                .get(app_name)
                .cloned()
                .unwrap_or_default()
        }
    }

    fn set_gesture(&mut self, gesture_name: String, entry: GestureEntry) {
        let app = &self.current_app;
        if app == "global" {
            self.config.global_gestures.insert(gesture_name, entry);
        } else {
            self.config
                .app_gestures
                .entry(app.clone())
                .or_default()
                .insert(gesture_name, entry);
        }
        if let Err(e) = self.save_config() {
            error!("Auto-save failed: {}", e);
        }
    }

    fn remove_gesture(&mut self, gesture_name: &str) {
        let app = &self.current_app;
        if app == "global" {
            self.config.global_gestures.remove(gesture_name);
        } else if let Some(map) = self.config.app_gestures.get_mut(app) {
            map.remove(gesture_name);
        }
        if let Err(e) = self.save_config() {
            error!("Auto-save failed: {}", e);
        }
    }

    fn gesture_pairs(&self) -> Vec<(String, GestureEntry)> {
        let map = self.gesture_map_for_app(&self.current_app);
        let mut pairs: Vec<(String, GestureEntry)> = map.into_iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    fn display_name_for_app(app_name: &str) -> String {
        if app_name == "global" {
            "全局".to_string()
        } else {
            std::path::Path::new(app_name)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| app_name.to_string())
        }
    }
}

// ---------- Model builders ----------

fn build_app_model(state: &DialogState) -> slint::ModelRc<AppItem> {
    let items: Vec<AppItem> = state
        .app_names
        .iter()
        .map(|name| AppItem {
            name: SharedString::from(name.as_str()),
            display_name: SharedString::from(DialogState::display_name_for_app(name).as_str()),
            selected: name == &state.current_app,
        })
        .collect();
    vec_to_model(items)
}

fn build_gesture_model(state: &DialogState) -> slint::ModelRc<GestureItem> {
    let pairs = state.gesture_pairs();
    let items: Vec<GestureItem> = pairs
        .iter()
        .map(|(name, entry)| {
            let display_name = if entry.name.is_empty() {
                name.as_str()
            } else {
                entry.name.as_str()
            };
            let item = GestureItem {
                name: SharedString::from(display_name),
                mnemonic: SharedString::from(gesture_name_to_mnemonic(name).as_str()),
                action_type: SharedString::from(action_type_display(&entry.action).as_str()),
                action_params: SharedString::from(action_params_display(&entry.action).as_str()),
            };
            info!(
                "GestureItem: name='{}', mnemonic='{}', type='{}', params='{}'",
                item.name, item.mnemonic, item.action_type, item.action_params
            );
            item
        })
        .collect();
    info!("build_gesture_model: total {} items", items.len());
    vec_to_model(items)
}

// ---------- Window creation and event loop ----------

/// Create the Slint window, wire up callbacks, and run the event loop.
/// Called from the persistent UI thread. Blocks until the window is closed.
fn run_settings_window(config_path: &PathBuf) {
    let config = match DialogState::load_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load config: {}", e);
            return;
        }
    };

    info!(
        "Loaded config: version={}, global_gestures_count={}, app_gestures_apps={}",
        config.version,
        config.global_gestures.len(),
        config.app_gestures.len()
    );

    let current_app = "global".to_string();
    let app_names = {
        let mut names: Vec<String> = config.app_gestures.keys().cloned().collect();
        names.sort();
        names.insert(0, "global".to_string());
        names.dedup();
        names
    };

    let state = Rc::new(RefCell::new(DialogState {
        config_path: config_path.clone(),
        config,
        current_app,
        app_names,
        edit_directions: Vec::new(),
        edit_trigger_button: crate::core::gesture::GestureTriggerButton::Middle,
        edit_original_name: None,
    }));

    let window = match GestureAppWindow::new() {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to create Slint window: {:?}", e);
            return;
        }
    };

    // Populate initial data
    {
        let state_ref = state.borrow();
        window.set_app_list(build_app_model(&state_ref));

        let pairs = state_ref.gesture_pairs();
        let names: Vec<&str> = pairs.iter().map(|(n, _)| n.as_str()).collect();
        info!(
            "Init gestures for app='{}': count={}, keys={:?}",
            state_ref.current_app,
            pairs.len(),
            names
        );

        window.set_gesture_list(build_gesture_model(&state_ref));
    }
    window.set_selected_gesture_index(-1);
    window.set_action_detail(SharedString::from(""));
    window.set_current_action_type_index(0);
    window.set_current_app_name(SharedString::from("global"));
    window.set_param_section_title(SharedString::from("参数设置"));
    window.set_edit_dialog_visible(false);
    window.set_edit_dialog_title(SharedString::from("添加手势"));
    window.set_edit_direction_display(SharedString::from(""));
    window.set_edit_action_type_index(0);
    window.set_edit_action_detail(SharedString::from(""));
    window.set_edit_window_command_index(0);
    window.set_edit_shortcut_display(SharedString::from(""));

    setup_callbacks(&window, &state);

    // Timer to poll for gesture capture results
    let capture_state = state.clone();
    let capture_window_weak = window.as_weak();
    let _capture_timer = slint::Timer::default();
    _capture_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            if let Some(win) = capture_window_weak.upgrade() {
                if !win.get_edit_dialog_visible() || !win.get_edit_capturing() {
                    return;
                }

                if let Some(captured) = crate::core::capture::take_capture_result() {
                    // Store directions and trigger button in state
                    {
                        let mut st = capture_state.borrow_mut();
                        st.edit_directions = captured.directions.clone();
                        st.edit_trigger_button = captured.trigger_button;
                    }
                    // Build gesture key with button prefix
                    let button_prefix = match captured.trigger_button {
                        crate::core::gesture::GestureTriggerButton::Right => "R_",
                        crate::core::gesture::GestureTriggerButton::Middle => "M_",
                        crate::core::gesture::GestureTriggerButton::X1 => "X1_",
                        crate::core::gesture::GestureTriggerButton::X2 => "X2_",
                    };
                    let display = gesture_name_to_mnemonic(&format!(
                        "{}{}",
                        button_prefix,
                        captured.directions.join(" → ")
                    ));
                    win.set_edit_direction_display(SharedString::from(display.as_str()));
                    win.set_edit_has_directions(true);
                    win.set_edit_capturing(false);
                    win.set_edit_trigger_button_index(trigger_button_to_index(
                        &captured.trigger_button,
                    ));
                }
            }
        },
    );

    // Timer to poll for keyboard shortcut capture via GetAsyncKeyState
    let shortcut_window_weak = window.as_weak();
    let shortcut_key_states = Rc::new(RefCell::new([false; 256]));
    let _shortcut_timer = slint::Timer::default();
    _shortcut_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(50),
        move || {
            use windows::Win32::UI::Input::KeyboardAndMouse::*;

            if let Some(win) = shortcut_window_weak.upgrade() {
                if !win.get_edit_shortcut_capturing() || !win.get_edit_dialog_visible() {
                    return;
                }

                let mut prev = shortcut_key_states.borrow_mut();

                // Modifier VK codes to skip
                let modifier_vks: [usize; 10] = [
                    VK_SHIFT.0 as usize,
                    VK_LSHIFT.0 as usize,
                    VK_RSHIFT.0 as usize,
                    VK_CONTROL.0 as usize,
                    VK_LCONTROL.0 as usize,
                    VK_RCONTROL.0 as usize,
                    VK_MENU.0 as usize,
                    VK_LMENU.0 as usize,
                    // VK_RMENU omitted intentionally
                    VK_LWIN.0 as usize,
                    VK_RWIN.0 as usize,
                ];

                // Scan for newly pressed non-modifier keys
                for vk in 1u8..=254u8 {
                    let vk_idx = vk as usize;

                    // Skip modifier keys
                    if modifier_vks.contains(&vk_idx) {
                        continue;
                    }

                    let is_down = unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 };
                    let was_down = prev[vk_idx];
                    prev[vk_idx] = is_down;

                    if is_down && !was_down {
                        // Escape cancels capture
                        if vk == VK_ESCAPE.0 as u8 {
                            win.set_edit_shortcut_display(SharedString::from(""));
                            win.set_edit_action_detail(SharedString::from(""));
                            win.set_edit_shortcut_capturing(false);
                            // Reset all states
                            for s in prev.iter_mut() {
                                *s = false;
                            }
                            return;
                        }

                        // Read current modifier state
                        let ctrl =
                            unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0 };
                        let alt =
                            unsafe { GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0 };
                        let shift =
                            unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0 };
                        let win_key =
                            unsafe { GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000 != 0 };

                        // Map VK code to display name and VK string
                        let (key_display, key_vk) = vk_code_to_info(vk as i32);

                        let mut display_parts: Vec<&str> = Vec::new();
                        let mut vk_parts: Vec<String> = Vec::new();

                        if win_key {
                            display_parts.push("Win");
                            vk_parts.push("VK_LWIN".to_string());
                        }
                        if ctrl {
                            display_parts.push("Ctrl");
                            vk_parts.push("VK_CONTROL".to_string());
                        }
                        if alt {
                            display_parts.push("Alt");
                            vk_parts.push("VK_MENU".to_string());
                        }
                        if shift {
                            display_parts.push("Shift");
                            vk_parts.push("VK_SHIFT".to_string());
                        }

                        let display_str = if display_parts.is_empty() {
                            key_display.clone()
                        } else {
                            format!("{} + {}", display_parts.join(" + "), key_display)
                        };
                        vk_parts.push(key_vk);

                        let vk_str = vk_parts.join("+");

                        info!("Shortcut captured: display='{}', vk='{}'", display_str, vk_str);

                        win.set_edit_shortcut_display(SharedString::from(display_str.as_str()));
                        win.set_edit_action_detail(SharedString::from(vk_str.as_str()));
                        win.set_edit_shortcut_capturing(false);

                        // Reset all states
                        for s in prev.iter_mut() {
                            *s = false;
                        }
                        return;
                    }
                }

                // Also update modifier key states
                for &vk_idx in &modifier_vks {
                    if vk_idx < 256 {
                        let is_down =
                            unsafe { GetAsyncKeyState(vk_idx as i32) as u16 & 0x8000 != 0 };
                        prev[vk_idx] = is_down;
                    }
                }
            }
        },
    );

    if let Err(e) = window.run() {
        error!("Slint window error: {:?}", e);
    }
}

fn setup_callbacks(window: &GestureAppWindow, state: &Rc<RefCell<DialogState>>) {
    // --- app-selected ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_app_selected(move |idx: i32| {
        let idx = idx as usize;
        let mut st = state_cb.borrow_mut();
        if idx >= st.app_names.len() {
            return;
        }
        st.current_app = st.app_names[idx].clone();
        let app_model = build_app_model(&st);
        let gesture_model = build_gesture_model(&st);
        if let Some(win) = window_weak.upgrade() {
            win.set_app_list(app_model);
            win.set_gesture_list(gesture_model);
            win.set_selected_gesture_index(-1);
            win.set_action_detail(SharedString::from(""));
            win.set_current_action_type_index(0);
            win.set_current_app_name(SharedString::from(st.current_app.as_str()));
            win.set_param_section_title(SharedString::from("参数设置"));
        }
    });

    // --- gesture-selected ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_gesture_selected(move |idx: i32| {
        let st = state_cb.borrow();
        let pairs = st.gesture_pairs();
        if idx < 0 || (idx as usize) >= pairs.len() {
            return;
        }
        let (_, entry) = &pairs[idx as usize];
        let type_idx = action_to_type_index(&entry.action);
        let detail = action_to_detail(&entry.action);
        if let Some(win) = window_weak.upgrade() {
            win.set_selected_gesture_index(idx);
            win.set_current_action_type_index(type_idx);
            win.set_action_detail(SharedString::from(detail.as_str()));
        }
    });

    // --- add-app-clicked ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_add_app_clicked(move || {
        let new_app = format!(
            "app_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
        let mut st = state_cb.borrow_mut();
        st.config
            .app_gestures
            .entry(new_app.clone())
            .or_insert_with(HashMap::new);
        if let Err(e) = st.save_config() {
            error!("Auto-save failed: {}", e);
        }
        st.app_names = st.build_app_names();
        st.current_app = new_app.clone();
        let app_model = build_app_model(&st);
        let gesture_model = build_gesture_model(&st);
        if let Some(win) = window_weak.upgrade() {
            win.set_app_list(app_model);
            win.set_gesture_list(gesture_model);
            win.set_selected_gesture_index(-1);
            win.set_action_detail(SharedString::from(""));
            win.set_current_action_type_index(0);
            win.set_current_app_name(SharedString::from(new_app.as_str()));
        }
    });

    // --- add-gesture-clicked: show edit dialog in "add" mode ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
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
            win.set_edit_trigger_button_index(0);
            win.set_edit_shortcut_display(SharedString::from(""));
            win.set_edit_dialog_visible(true);
        }
    });

    // --- remove-gesture-clicked ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_remove_gesture_clicked(move |idx: i32| {
        let mut st = state_cb.borrow_mut();
        let pairs = st.gesture_pairs();
        if idx < 0 || (idx as usize) >= pairs.len() {
            return;
        }
        let name = pairs[idx as usize].0.clone();
        st.remove_gesture(&name);
        let gesture_model = build_gesture_model(&st);
        if let Some(win) = window_weak.upgrade() {
            win.set_gesture_list(gesture_model);
            win.set_selected_gesture_index(-1);
            win.set_action_detail(SharedString::from(""));
            win.set_current_action_type_index(0);
        }
    });

    // --- edit-gesture-clicked: show edit dialog in "edit" mode ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_edit_gesture_clicked(move |idx: i32| {
        // Read data from state
        let (
            trigger_button,
            directions,
            original_name,
            entry_name,
            display,
            type_idx,
            detail,
            window_cmd_idx,
            shortcut_display,
        ) = {
            let st = state_cb.borrow();
            let pairs = st.gesture_pairs();
            if idx < 0 || (idx as usize) >= pairs.len() {
                return;
            }
            let (name, entry) = &pairs[idx as usize];
            // Parse key: "M_Right → Down" → trigger_button=Middle, dirs=["Right", "Down"]
            let (tb, dirs) = parse_gesture_key(name);
            let disp = gesture_name_to_mnemonic(name);
            let t_idx = action_to_type_index(&entry.action);
            let det = action_to_detail(&entry.action);
            let wc_idx = match &entry.action {
                Action::Window(w) => window_command_to_index(&w.command),
                _ => 0,
            };
            let sc_disp = match &entry.action {
                Action::Keyboard(kb) => format_keys_display(&kb.keys),
                _ => String::new(),
            };
            (
                tb,
                dirs,
                name.clone(),
                entry.name.clone(),
                disp,
                t_idx,
                det,
                wc_idx,
                sc_disp,
            )
        };

        // Update edit state
        {
            let mut st = state_cb.borrow_mut();
            st.edit_directions = directions;
            st.edit_trigger_button = trigger_button;
            st.edit_original_name = Some(original_name);
        }

        // Show dialog
        if let Some(win) = window_weak.upgrade() {
            win.set_edit_dialog_title(SharedString::from("编辑手势"));
            win.set_edit_gesture_name(SharedString::from(entry_name.as_str()));
            win.set_edit_direction_display(SharedString::from(display.as_str()));
            win.set_edit_has_directions(true);
            win.set_edit_capturing(false);
            win.set_edit_action_type_index(type_idx);
            win.set_edit_action_detail(SharedString::from(detail.as_str()));
            win.set_edit_window_command_index(window_cmd_idx);
            win.set_edit_trigger_button_index(trigger_button_to_index(&trigger_button));
            win.set_edit_shortcut_display(SharedString::from(shortcut_display.as_str()));
            win.set_edit_dialog_visible(true);
        }
    });

    // --- action-type-changed ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_action_type_changed(move |new_type_idx: i32| {
        let mut st = state_cb.borrow_mut();
        let selected_idx = window_weak
            .upgrade()
            .map(|w| w.get_selected_gesture_index())
            .unwrap_or(-1);
        if selected_idx < 0 {
            return;
        }
        let pairs = st.gesture_pairs();
        if (selected_idx as usize) >= pairs.len() {
            return;
        }
        let (gesture_name, old_entry) = &pairs[selected_idx as usize];
        let detail = action_to_detail(&old_entry.action);
        let new_action = match new_type_idx {
            0 => Action::Keyboard(crate::config::config::KeyboardAction { keys: vec![detail] }),
            1 => Action::Mouse(crate::config::config::MouseAction {
                button: crate::config::config::MouseButton::Left,
                action_type: crate::config::config::MouseActionType::Click,
            }),
            2 => Action::Window(crate::config::config::WindowAction {
                command: crate::config::config::WindowCommand::Minimize,
            }),
            3 => Action::Run(crate::config::config::RunAction {
                command: detail,
                args: None,
            }),
            _ => return,
        };
        let new_entry = GestureEntry {
            name: old_entry.name.clone(),
            action: new_action,
        };
        st.set_gesture(gesture_name.clone(), new_entry);
        if let Some(win) = window_weak.upgrade() {
            win.set_gesture_list(build_gesture_model(&st));
        }
    });

    // --- action-detail-changed ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_action_detail_changed(move |new_detail: SharedString| {
        let detail_str = new_detail.to_string();
        let mut st = state_cb.borrow_mut();
        let selected_idx = window_weak
            .upgrade()
            .map(|w| w.get_selected_gesture_index())
            .unwrap_or(-1);
        if selected_idx < 0 {
            return;
        }
        let pairs = st.gesture_pairs();
        if (selected_idx as usize) >= pairs.len() {
            return;
        }
        let (gesture_name, old_entry) = &pairs[selected_idx as usize];
        let type_idx = action_to_type_index(&old_entry.action);
        let new_action = match type_idx {
            0 => Action::Keyboard(crate::config::config::KeyboardAction {
                keys: detail_str
                    .split('+')
                    .map(|s| s.trim().to_string())
                    .collect(),
            }),
            1 => Action::Mouse(crate::config::config::MouseAction {
                button: crate::config::config::MouseButton::Left,
                action_type: crate::config::config::MouseActionType::Click,
            }),
            2 => Action::Window(crate::config::config::WindowAction {
                command: crate::config::config::WindowCommand::Minimize,
            }),
            3 => Action::Run(crate::config::config::RunAction {
                command: detail_str,
                args: None,
            }),
            _ => return,
        };
        let new_entry = GestureEntry {
            name: old_entry.name.clone(),
            action: new_action,
        };
        st.set_gesture(gesture_name.clone(), new_entry);
        if let Some(win) = window_weak.upgrade() {
            win.set_gesture_list(build_gesture_model(&st));
        }
    });

    // --- edit-capture-clicked ---
    let window_weak = window.as_weak();
    window.on_edit_capture_clicked(move || {
        if let Some(win) = window_weak.upgrade() {
            let currently_capturing = win.get_edit_capturing();
            if currently_capturing {
                crate::core::capture::cancel_capture();
                win.set_edit_capturing(false);
            } else {
                crate::core::capture::start_capture();
                win.set_edit_capturing(true);
            }
        }
    });

    // --- edit-clear-directions ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
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

    // --- edit-dialog-confirmed ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_edit_dialog_confirmed(move || {
        // Cancel any active capture
        crate::core::capture::cancel_capture();
        // Read values from window before borrowing state
        let (
            action_type_idx,
            action_detail,
            window_cmd_idx,
            gesture_name_input,
            trigger_button_idx,
        ) = {
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

        let trigger_button = index_to_trigger_button(trigger_button_idx);

        let mut st = state_cb.borrow_mut();

        // Need at least one direction
        if st.edit_directions.is_empty() {
            return;
        }

        // Build gesture key with button prefix (e.g., "M_Right → Down")
        let button_prefix = match trigger_button {
            crate::core::gesture::GestureTriggerButton::Right => "R_",
            crate::core::gesture::GestureTriggerButton::Middle => "M_",
            crate::core::gesture::GestureTriggerButton::X1 => "X1_",
            crate::core::gesture::GestureTriggerButton::X2 => "X2_",
        };
        st.edit_trigger_button = trigger_button;
        let gesture_key = format!("{}{}", button_prefix, st.edit_directions.join(" → "));

        // If editing and key changed, remove old gesture
        let old_name_to_remove = st.edit_original_name.as_ref().and_then(|original_name| {
            if original_name != &gesture_key {
                Some(original_name.clone())
            } else {
                None
            }
        });
        if let Some(old_name) = old_name_to_remove {
            st.remove_gesture(&old_name);
        }

        // Build action based on type
        let new_action = if action_type_idx == 2 {
            // Window action - use window command index
            Action::Window(WindowAction {
                command: index_to_window_command(window_cmd_idx),
            })
        } else {
            match action_type_idx {
                0 => Action::Keyboard(KeyboardAction {
                    keys: if action_detail.is_empty() {
                        vec!["VK_UNKNOWN".to_string()]
                    } else {
                        action_detail
                            .split('+')
                            .map(|s| s.trim().to_string())
                            .collect()
                    },
                }),
                1 => Action::Mouse(MouseAction {
                    button: MouseButton::Left,
                    action_type: MouseActionType::Click,
                }),
                3 => Action::Run(RunAction {
                    command: if action_detail.is_empty() {
                        "notepad.exe".to_string()
                    } else {
                        action_detail
                    },
                    args: None,
                }),
                _ => return,
            }
        };

        let entry = GestureEntry {
            name: gesture_name_input,
            action: new_action,
        };
        st.set_gesture(gesture_key, entry);

        let gesture_model = build_gesture_model(&st);
        st.edit_original_name = None;
        drop(st);

        if let Some(win) = window_weak.upgrade() {
            win.set_gesture_list(gesture_model);
            win.set_selected_gesture_index(-1);
            win.set_edit_has_directions(false);
            win.set_edit_capturing(false);
            win.set_edit_dialog_visible(false);
        }
    });

    // --- edit-dialog-cancelled ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
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

}

// ---------- Public API ----------

pub struct ConfigDialog {
    config_path: PathBuf,
}

impl ConfigDialog {
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// Send a command to the persistent UI thread to show the settings window.
    /// This is non-blocking - it returns immediately after sending the message.
    pub fn show(&self, _parent_hwnd: Option<windows::Win32::Foundation::HWND>) {
        let sender = ui_sender();
        if let Err(e) = sender.send(UiCommand::ShowSettings {
            config_path: self.config_path.clone(),
        }) {
            error!("Failed to send UI command: {}", e);
        }
    }
}
