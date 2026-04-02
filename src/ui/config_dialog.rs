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
use tracing::{error, info, warn};

use crate::config::config::{Action, GestureConfig};

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

    name.split(" -> ")
        .map(|part| {
            dir_map
                .get(part.trim())
                .map(|s| s.to_string())
                .unwrap_or_else(|| part.trim().to_string())
        })
        .collect::<Vec<_>>()
        .join("")
}

fn action_to_type_index(action: &Action) -> i32 {
    match action {
        Action::Keyboard(_) => 0,
        Action::Mouse(_) => 1,
        Action::Window(_) => 2,
        Action::Run(_) => 3,
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

// ---------- Internal state ----------

struct DialogState {
    config_path: PathBuf,
    config: GestureConfig,
    current_app: String,
    app_names: Vec<String>,
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

    fn gesture_map_for_app(&self, app_name: &str) -> HashMap<String, Action> {
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

    fn set_gesture(&mut self, gesture_name: String, action: Action) {
        let app = &self.current_app;
        if app == "global" {
            self.config.global_gestures.insert(gesture_name, action);
        } else {
            self.config
                .app_gestures
                .entry(app.clone())
                .or_default()
                .insert(gesture_name, action);
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

    fn gesture_pairs(&self) -> Vec<(String, Action)> {
        let map = self.gesture_map_for_app(&self.current_app);
        let mut pairs: Vec<(String, Action)> = map.into_iter().collect();
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
    let items: Vec<GestureItem> = state
        .gesture_pairs()
        .iter()
        .map(|(name, action)| GestureItem {
            name: SharedString::from(name.as_str()),
            mnemonic: SharedString::from(gesture_name_to_mnemonic(name).as_str()),
            action_description: SharedString::from(action.display_info().as_str()),
            action_type: SharedString::from(type_index_to_display(action_to_type_index(action))),
        })
        .collect();
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
        window.set_gesture_list(build_gesture_model(&state_ref));
    }
    window.set_selected_gesture_index(-1);
    window.set_action_detail(SharedString::from(""));
    window.set_current_action_type_index(0);
    window.set_current_app_name(SharedString::from("global"));
    window.set_param_section_title(SharedString::from("参数设置"));

    setup_callbacks(&window, &state);

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
        let (_, action) = &pairs[idx as usize];
        let type_idx = action_to_type_index(action);
        let detail = action_to_detail(action);
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
        st.config.app_gestures.entry(new_app.clone()).or_insert_with(HashMap::new);
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

    // --- add-gesture-clicked ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_add_gesture_clicked(move || {
        let mut st = state_cb.borrow_mut();
        let gesture_name = format!("Gesture_{}", st.gesture_pairs().len() + 1);
        let default_action = Action::Keyboard(crate::config::config::KeyboardAction {
            keys: vec!["VK_UNKNOWN".to_string()],
        });
        st.set_gesture(gesture_name.clone(), default_action);
        let gesture_model = build_gesture_model(&st);
        let pairs = st.gesture_pairs();
        let new_idx = pairs
            .iter()
            .position(|(n, _)| n == &gesture_name)
            .map(|i| i as i32)
            .unwrap_or(-1);
        if let Some(win) = window_weak.upgrade() {
            win.set_gesture_list(gesture_model);
            win.set_selected_gesture_index(new_idx);
            win.set_action_detail(SharedString::from("VK_UNKNOWN"));
            win.set_current_action_type_index(0);
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

    // --- edit-gesture-clicked ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_edit_gesture_clicked(move |idx: i32| {
        let st = state_cb.borrow();
        let pairs = st.gesture_pairs();
        if idx < 0 || (idx as usize) >= pairs.len() {
            return;
        }
        let (_, action) = &pairs[idx as usize];
        if let Some(win) = window_weak.upgrade() {
            win.set_selected_gesture_index(idx);
            win.set_current_action_type_index(action_to_type_index(action));
            win.set_action_detail(SharedString::from(action_to_detail(action).as_str()));
        }
    });

    // --- action-type-changed ---
    let state_cb = state.clone();
    let window_weak = window.as_weak();
    window.on_action_type_changed(move |new_type_idx: i32| {
        let mut st = state_cb.borrow_mut();
        let selected_idx = window_weak.upgrade().map(|w| w.get_selected_gesture_index()).unwrap_or(-1);
        if selected_idx < 0 {
            return;
        }
        let pairs = st.gesture_pairs();
        if (selected_idx as usize) >= pairs.len() {
            return;
        }
        let (gesture_name, old_action) = &pairs[selected_idx as usize];
        let detail = action_to_detail(old_action);
        let new_action = match new_type_idx {
            0 => Action::Keyboard(crate::config::config::KeyboardAction { keys: vec![detail] }),
            1 => Action::Mouse(crate::config::config::MouseAction {
                button: crate::config::config::MouseButton::Left,
                action_type: crate::config::config::MouseActionType::Click,
            }),
            2 => Action::Window(crate::config::config::WindowAction {
                command: crate::config::config::WindowCommand::Minimize,
            }),
            3 => Action::Run(crate::config::config::RunAction { command: detail, args: None }),
            _ => return,
        };
        st.set_gesture(gesture_name.clone(), new_action);
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
        let selected_idx = window_weak.upgrade().map(|w| w.get_selected_gesture_index()).unwrap_or(-1);
        if selected_idx < 0 {
            return;
        }
        let pairs = st.gesture_pairs();
        if (selected_idx as usize) >= pairs.len() {
            return;
        }
        let (gesture_name, old_action) = &pairs[selected_idx as usize];
        let type_idx = action_to_type_index(old_action);
        let new_action = match type_idx {
            0 => Action::Keyboard(crate::config::config::KeyboardAction {
                keys: detail_str.split('+').map(|s| s.trim().to_string()).collect(),
            }),
            1 => Action::Mouse(crate::config::config::MouseAction {
                button: crate::config::config::MouseButton::Left,
                action_type: crate::config::config::MouseActionType::Click,
            }),
            2 => Action::Window(crate::config::config::WindowAction {
                command: crate::config::config::WindowCommand::Minimize,
            }),
            3 => Action::Run(crate::config::config::RunAction { command: detail_str, args: None }),
            _ => return,
        };
        st.set_gesture(gesture_name.clone(), new_action);
        if let Some(win) = window_weak.upgrade() {
            win.set_gesture_list(build_gesture_model(&st));
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
