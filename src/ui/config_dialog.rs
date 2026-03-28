//! Configuration dialog - native Win32 settings UI

use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::core::PCWSTR;
use windows::core::w;
use tracing::{info, warn, error};
use std::path::PathBuf;

use crate::config::config::{GestureConfig, Settings, TriggerButton};

// Control IDs (i32 for GetDlgItem compatibility)
const IDC_TRIGGER_COMBO: i32 = 1001;
const IDC_MIN_DISTANCE: i32 = 1002;
const IDC_EFFECTIVE_MOVE: i32 = 1003;
const IDC_STAY_TIMEOUT: i32 = 1004;
const IDC_8DIRECTION: i32 = 1005;
const IDC_DISABLE_FULLSCREEN: i32 = 1006;
const IDC_APPLY_BTN: i32 = 1010;
const IDC_OK_BTN: i32 = 1011;
const IDC_CANCEL_BTN: i32 = 1012;

// Layout
const DLG_W: i32 = 380;
const DLG_H: i32 = 300;
const MG: i32 = 20;
const LABEL_W: i32 = 140;
const CTRL_W: i32 = 80;
const COMBO_W: i32 = 160;
const ROW_H: i32 = 30;
const BTN_W: i32 = 80;
const BTN_H: i32 = 28;

struct DialogData {
    config_path: PathBuf,
}

pub struct ConfigDialog {
    config_path: PathBuf,
}

fn hmenu_from_id(id: i32) -> HMENU {
    HMENU(id as *mut core::ffi::c_void)
}

fn get_ctrl_text(hwnd: HWND, id: i32) -> String {
    unsafe {
        let ctrl = GetDlgItem(hwnd, id).unwrap_or_default();
        let len = SendMessageW(ctrl, WM_GETTEXTLENGTH, WPARAM(0), LPARAM(0)).0 as usize;
        if len == 0 { return String::new(); }
        let mut buf = vec![0u16; len + 1];
        SendMessageW(ctrl, WM_GETTEXT, WPARAM(len + 1), LPARAM(buf.as_mut_ptr() as isize));
        String::from_utf16_lossy(&buf[..len])
    }
}

fn set_ctrl_text(hwnd: HWND, id: i32, text: &str) {
    unsafe {
        let ctrl = GetDlgItem(hwnd, id).unwrap_or_default();
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        SendMessageW(ctrl, WM_SETTEXT, WPARAM(0), LPARAM(wide.as_ptr() as isize));
    }
}

impl ConfigDialog {
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
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

    pub fn show(&self, parent_hwnd: Option<HWND>) {
        unsafe {
            let hmodule = match GetModuleHandleW(PCWSTR::null()) {
                Ok(h) => h,
                Err(e) => { error!("GetModuleHandleW failed: {:?}", e); return; }
            };
            let hinst = HINSTANCE(hmodule.0);

            let config = match Self::load_config(&self.config_path) {
                Ok(c) => c,
                Err(e) => { error!("Load config failed: {}", e); return; }
            };

            // Register class
            let wc = WNDCLASSW {
                hInstance: hinst,
                lpszClassName: PCWSTR(w!("RustGestureCfg").as_ptr()),
                lpfnWndProc: Some(Self::dialog_proc),
                style: CS_HREDRAW | CS_VREDRAW,
                cbClsExtra: 0, cbWndExtra: 0,
                hIcon: HICON::default(),
                hCursor: LoadCursorW(HINSTANCE::default(), IDC_ARROW).unwrap_or_default(),
                hbrBackground: HBRUSH::default(),
                lpszMenuName: PCWSTR::null(),
            };
            let atom = RegisterClassW(&wc);
            if atom == 0 {
                warn!("RegisterClassW failed (may already exist)");
            }

            // Center on parent
            let (x, y) = match parent_hwnd {
                Some(parent) if !parent.is_invalid() => {
                    let mut rect = RECT::default();
                    let _ = GetWindowRect(parent, &mut rect);
                    ((rect.left + rect.right) / 2 - DLG_W / 2,
                     (rect.top + rect.bottom) / 2 - DLG_H / 2)
                }
                _ => (CW_USEDEFAULT, CW_USEDEFAULT),
            };

            // Store data
            let data = Box::new(DialogData { config_path: self.config_path.clone() });

            // Create window
            let hwnd_result = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(w!("RustGestureCfg").as_ptr()),
                PCWSTR(w!("RustGesture Settings").as_ptr()),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                x, y, DLG_W, DLG_H,
                None,
                None,
                hinst,
                None,
            );

            let hwnd = match hwnd_result {
                Ok(h) if !h.is_invalid() => h,
                _ => {
                    error!("CreateWindowExW failed");
                    let _ = Box::from_raw(Box::into_raw(data) as *mut DialogData);
                    return;
                }
            };

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(data) as isize);
            Self::create_controls(hwnd, hinst, &config.settings);
            let _ = ShowWindow(hwnd, SW_SHOW);

            // Modal loop
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    unsafe fn create_controls(hwnd: HWND, hinst: HINSTANCE, s: &Settings) {
        let mut y = MG + 8;

        // --- Trigger Button ---
        CreateWindowExW(WINDOW_EX_STYLE(0),
            PCWSTR(w!("STATIC").as_ptr()), PCWSTR(w!("Trigger Button:").as_ptr()),
            WS_CHILD | WS_VISIBLE, MG, y, LABEL_W, 20,
            hwnd, None, hinst, None).ok();

        let combo = CreateWindowExW(WINDOW_EX_STYLE(0),
            PCWSTR(w!("COMBOBOX").as_ptr()), PCWSTR(w!("").as_ptr()),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST as u32) | WS_VSCROLL,
            MG + LABEL_W + 8, y, COMBO_W, 150,
            hwnd, hmenu_from_id(IDC_TRIGGER_COMBO), hinst, None)
            .unwrap_or_default();

        let buttons = ["Right", "Middle", "X1", "X2"];
        let sel = match s.trigger_button {
            TriggerButton::Right => 0, TriggerButton::Middle => 1,
            TriggerButton::X1 => 2, TriggerButton::X2 => 3,
        };
        for btn in &buttons {
            let t: Vec<u16> = btn.encode_utf16().chain(std::iter::once(0)).collect();
            SendMessageW(combo, CB_ADDSTRING, WPARAM(0), LPARAM(t.as_ptr() as isize));
        }
        SendMessageW(combo, CB_SETCURSEL, WPARAM(sel as usize), LPARAM(0));
        y += ROW_H;

        // --- Min Distance ---
        CreateWindowExW(WINDOW_EX_STYLE(0),
            PCWSTR(w!("STATIC").as_ptr()), PCWSTR(w!("Min Distance (px):").as_ptr()),
            WS_CHILD | WS_VISIBLE, MG, y, LABEL_W, 20,
            hwnd, None, hinst, None).ok();

        CreateWindowExW(WS_EX_CLIENTEDGE,
            PCWSTR(w!("EDIT").as_ptr()), PCWSTR(w!("").as_ptr()),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(ES_NUMBER as u32),
            MG + LABEL_W + 8, y, CTRL_W, 22,
            hwnd, hmenu_from_id(IDC_MIN_DISTANCE), hinst, None).ok();
        set_ctrl_text(hwnd, IDC_MIN_DISTANCE, &s.min_distance.to_string());
        y += ROW_H;

        // --- Effective Move ---
        CreateWindowExW(WINDOW_EX_STYLE(0),
            PCWSTR(w!("STATIC").as_ptr()), PCWSTR(w!("Effective Move (px):").as_ptr()),
            WS_CHILD | WS_VISIBLE, MG, y, LABEL_W, 20,
            hwnd, None, hinst, None).ok();

        CreateWindowExW(WS_EX_CLIENTEDGE,
            PCWSTR(w!("EDIT").as_ptr()), PCWSTR(w!("").as_ptr()),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(ES_NUMBER as u32),
            MG + LABEL_W + 8, y, CTRL_W, 22,
            hwnd, hmenu_from_id(IDC_EFFECTIVE_MOVE), hinst, None).ok();
        set_ctrl_text(hwnd, IDC_EFFECTIVE_MOVE, &s.effective_move.to_string());
        y += ROW_H;

        // --- Timeout ---
        CreateWindowExW(WINDOW_EX_STYLE(0),
            PCWSTR(w!("STATIC").as_ptr()), PCWSTR(w!("Timeout (ms):").as_ptr()),
            WS_CHILD | WS_VISIBLE, MG, y, LABEL_W, 20,
            hwnd, None, hinst, None).ok();

        CreateWindowExW(WS_EX_CLIENTEDGE,
            PCWSTR(w!("EDIT").as_ptr()), PCWSTR(w!("").as_ptr()),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(ES_NUMBER as u32),
            MG + LABEL_W + 8, y, CTRL_W, 22,
            hwnd, hmenu_from_id(IDC_STAY_TIMEOUT), hinst, None).ok();
        set_ctrl_text(hwnd, IDC_STAY_TIMEOUT, &s.stay_timeout.to_string());
        y += ROW_H + 8;

        // --- 8-Direction checkbox ---
        let cb8 = CreateWindowExW(WINDOW_EX_STYLE(0),
            PCWSTR(w!("BUTTON").as_ptr()), PCWSTR(w!("Enable 8-Direction Gestures").as_ptr()),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            MG, y, DLG_W - MG * 2, 22,
            hwnd, hmenu_from_id(IDC_8DIRECTION), hinst, None)
            .unwrap_or_default();
        if s.enable_8_direction {
            SendMessageW(cb8, BM_SETCHECK, WPARAM(1), LPARAM(0));
        }
        y += ROW_H;

        // --- Disable fullscreen checkbox ---
        let cbfs = CreateWindowExW(WINDOW_EX_STYLE(0),
            PCWSTR(w!("BUTTON").as_ptr()), PCWSTR(w!("Disable in Fullscreen Apps").as_ptr()),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            MG, y, DLG_W - MG * 2, 22,
            hwnd, hmenu_from_id(IDC_DISABLE_FULLSCREEN), hinst, None)
            .unwrap_or_default();
        if s.disable_in_fullscreen {
            SendMessageW(cbfs, BM_SETCHECK, WPARAM(1), LPARAM(0));
        }
        y += ROW_H + 24;

        // --- Buttons ---
        CreateWindowExW(WINDOW_EX_STYLE(0),
            PCWSTR(w!("BUTTON").as_ptr()), PCWSTR(w!("Apply").as_ptr()),
            WS_CHILD | WS_VISIBLE, MG, y, BTN_W, BTN_H,
            hwnd, hmenu_from_id(IDC_APPLY_BTN), hinst, None).ok();

        CreateWindowExW(WINDOW_EX_STYLE(0),
            PCWSTR(w!("BUTTON").as_ptr()), PCWSTR(w!("OK").as_ptr()),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
            MG + BTN_W + 10, y, BTN_W, BTN_H,
            hwnd, hmenu_from_id(IDC_OK_BTN), hinst, None).ok();

        CreateWindowExW(WINDOW_EX_STYLE(0),
            PCWSTR(w!("BUTTON").as_ptr()), PCWSTR(w!("Cancel").as_ptr()),
            WS_CHILD | WS_VISIBLE,
            MG + (BTN_W + 10) * 2, y, BTN_W, BTN_H,
            hwnd, hmenu_from_id(IDC_CANCEL_BTN), hinst, None).ok();
    }

    unsafe fn collect_settings(hwnd: HWND) -> Option<Settings> {
        let combo = GetDlgItem(hwnd, IDC_TRIGGER_COMBO).unwrap_or_default();
        let sel = SendMessageW(combo, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as i32;
        let trigger_button = match sel {
            0 => TriggerButton::Right, 1 => TriggerButton::Middle,
            2 => TriggerButton::X1, 3 => TriggerButton::X2,
            _ => { warn!("Invalid trigger"); return None; }
        };

        let min_distance: u32 = get_ctrl_text(hwnd, IDC_MIN_DISTANCE).parse().ok()?;
        let effective_move: u32 = get_ctrl_text(hwnd, IDC_EFFECTIVE_MOVE).parse().ok()?;
        let stay_timeout: u32 = get_ctrl_text(hwnd, IDC_STAY_TIMEOUT).parse().ok()?;
        if min_distance == 0 || effective_move == 0 || stay_timeout == 0 { return None; }

        let cb8 = GetDlgItem(hwnd, IDC_8DIRECTION).unwrap_or_default();
        let enable_8_direction = SendMessageW(cb8, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;

        let cbfs = GetDlgItem(hwnd, IDC_DISABLE_FULLSCREEN).unwrap_or_default();
        let disable_in_fullscreen = SendMessageW(cbfs, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;

        Some(Settings { trigger_button, min_distance, effective_move, stay_timeout, enable_8_direction, disable_in_fullscreen })
    }

    unsafe fn do_apply(hwnd: HWND) -> bool {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const DialogData;
        if ptr.is_null() { return false; }
        let data = &*ptr;

        let settings = match Self::collect_settings(hwnd) {
            Some(s) => s,
            None => {
                let _ = MessageBoxW(hwnd,
                    PCWSTR(w!("Invalid values.").as_ptr()),
                    PCWSTR(w!("Error").as_ptr()),
                    MB_OK | MB_ICONWARNING);
                return false;
            }
        };

        match std::fs::read_to_string(&data.config_path) {
            Ok(content) => match serde_json::from_str::<GestureConfig>(&content) {
                Ok(mut config) => {
                    config.settings = settings;
                    match Self::save_config(&config, &data.config_path) {
                        Ok(()) => { info!("Settings saved"); return true; }
                        Err(e) => error!("Save: {}", e),
                    }
                }
                Err(e) => error!("Parse: {}", e),
            },
            Err(e) => error!("Read: {}", e),
        }

        let _ = MessageBoxW(hwnd,
            PCWSTR(w!("Failed to save.").as_ptr()),
            PCWSTR(w!("Error").as_ptr()),
            MB_OK | MB_ICONERROR);
        false
    }

    unsafe extern "system" fn dialog_proc(
        hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_COMMAND => {
                let id = (wparam.0 & 0xFFFF) as i32;
                match id {
                    IDC_OK_BTN => { if Self::do_apply(hwnd) { DestroyWindow(hwnd); } LRESULT(0) }
                    IDC_APPLY_BTN => { Self::do_apply(hwnd); LRESULT(0) }
                    IDC_CANCEL_BTN => { DestroyWindow(hwnd); LRESULT(0) }
                    _ => DefWindowProcW(hwnd, msg, wparam, lparam),
                }
            }
            WM_CLOSE => { DestroyWindow(hwnd); LRESULT(0) }
            WM_DESTROY => {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DialogData;
                if !ptr.is_null() {
                    let _ = Box::from_raw(ptr);
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
