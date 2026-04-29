//! System tray icon module
//!
//! This module provides Windows system tray icon functionality with context menu.

use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::Graphics::Gdi::*;
use windows::core::PCWSTR;
use windows::core::w;
use tracing::{info, warn};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::PathBuf;

use super::config_dialog::ConfigDialog;

/// Structure to hold window user data
struct WindowUserData {
    enabled: *const AtomicBool,
    shutdown_tx: Option<std::sync::mpsc::Sender<()>>,
    config_path: PathBuf,
}

/// System tray icon manager
pub struct TrayIcon {
    enabled_ptr: *const AtomicBool,
    hwnd: HWND,
    icon_added: bool,
    shutdown_tx: Option<std::sync::mpsc::Sender<()>>,
    config_path: PathBuf,
}

impl TrayIcon {
    /// Create a new system tray icon
    pub fn new(enabled: Arc<AtomicBool>, shutdown_tx: std::sync::mpsc::Sender<()>, config_path: PathBuf) -> anyhow::Result<Self> {
        info!("Creating system tray icon with context menu");

        unsafe {
            // Get module handle
            let hinstance = GetModuleHandleW(PCWSTR::null())
                .map_err(|e| anyhow::anyhow!("Failed to get module handle: {:?}", e))?;

            // Register window class for tray icon
            let wnd_class = WNDCLASSW {
                hInstance: HINSTANCE(hinstance.0),
                lpszClassName: PCWSTR(w!("RustGestureTrayClass").as_ptr()),
                lpfnWndProc: Some(Self::window_proc),
                style: CS_HREDRAW | CS_VREDRAW,
                cbClsExtra: 0,
                cbWndExtra: std::mem::size_of::<usize>() as i32, // Store pointer to enabled
                hIcon: HICON::default(),
                hCursor: HCURSOR::default(),
                hbrBackground: HBRUSH::default(),
                lpszMenuName: PCWSTR::null(),
            };

            let atom = RegisterClassW(&wnd_class);
            if atom == 0 {
                let error = GetLastError();
                warn!("Failed to register window class (may already exist): {:?}", error);
            }

            // Create hidden window
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(w!("RustGestureTrayClass").as_ptr()),
                PCWSTR(w!("").as_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                hinstance,
                None
            )?;

            if hwnd.is_invalid() {
                let error = GetLastError();
                return Err(anyhow::anyhow!("Failed to create window: {:?}", error));
            }

            info!("Created tray window: {:?}", hwnd);

            // Store user data in window
            let enabled_ptr = Arc::into_raw(enabled.clone()) as *const AtomicBool;
            let shutdown_tx_clone = shutdown_tx.clone();
            let user_data = Box::new(WindowUserData {
                enabled: enabled_ptr,
                shutdown_tx: Some(shutdown_tx),
                config_path: config_path.clone(),
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::leak(user_data) as *mut WindowUserData as isize);

            // Load system icon (IDI_APPLICATION = 32512)
            let icon_result = LoadIconW(HINSTANCE::default(), PCWSTR::from_raw(32512u16 as *const u16));
            let icon = match icon_result {
                Ok(hicon) if !hicon.is_invalid() => {
                    info!("✅ System icon loaded successfully: {:?}", hicon);
                    hicon
                }
                _ => {
                    warn!("Failed to load system icon, using default");
                    HICON::default()
                }
            };

            // Add tray icon
            let nid = Self::create_notifyicon_data(hwnd, icon);
            info!("Adding tray icon to notification area...");
            if Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                info!("✅ System tray icon added successfully");
            } else {
                warn!("Failed to add system tray icon");
            }

            Ok(Self {
                enabled_ptr,
                hwnd,
                icon_added: true,
                shutdown_tx: Some(shutdown_tx_clone),
                config_path,
            })
        }
    }

    /// Create NOTIFYICONDATAW structure
    unsafe fn create_notifyicon_data(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NOTIFY_ICON_DATA_FLAGS(NIF_ICON.0 | NIF_MESSAGE.0 | NIF_TIP.0),
            uCallbackMessage: (WM_USER + 100) as u32,
            hIcon: icon,
            szTip: [0; 128],
            dwState: NOTIFY_ICON_STATE(0),
            dwStateMask: NOTIFY_ICON_STATE(0),
            szInfo: [0; 256],
            szInfoTitle: [0; 64],
            dwInfoFlags: NIIF_NONE,
            Anonymous: Default::default(),
            guidItem: Default::default(),
            hBalloonIcon: HICON::default(),
        }
    }

    /// Update tray icon tooltip
    pub fn update_tooltip(&self, enabled: bool) {
        unsafe {
            let hinstance = GetModuleHandleW(PCWSTR::null()).unwrap();
            let icon = LoadIconW(HINSTANCE(hinstance.0), PCWSTR::from_raw(32512 as *const u16))
                .unwrap_or(HICON::default());
            let mut nid = Self::create_notifyicon_data(self.hwnd, icon);

            let tooltip = if enabled {
                "RustGesture - Enabled"
            } else {
                "RustGesture - Disabled"
            };

            // Copy tooltip to szTip
            let tooltip_bytes = tooltip.encode_utf16().collect::<Vec<u16>>();
            for (i, &byte) in tooltip_bytes.iter().enumerate() {
                if i < 127 {
                    nid.szTip[i] = byte;
                }
            }

            if Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool() {
                info!("Tray tooltip updated: {}", tooltip);
            } else {
                warn!("Failed to update tray tooltip");
            }
        }
    }

    /// Toggle enabled state
    pub fn toggle(&self) -> bool {
        unsafe {
            let old_state = (*self.enabled_ptr).load(Ordering::SeqCst);
            let new_state = !old_state;
            (*self.enabled_ptr).store(new_state, Ordering::SeqCst);
            self.update_tooltip(new_state);
            new_state
        }
    }

    /// Show context menu at cursor position
    unsafe fn show_context_menu(&self) {
        let _hinstance = GetModuleHandleW(PCWSTR::null()).unwrap();

        // Get current cursor position
        let mut point = POINT { x: 0, y: 0 };
        GetCursorPos(&mut point);

        // Create context menu
        let hmenu = CreatePopupMenu().unwrap();

        let enabled = (*self.enabled_ptr).load(Ordering::SeqCst);

        AppendMenuW(hmenu, MF_STRING, 1, PCWSTR(w!("Enable Gesture Recognition").as_ptr()));
        AppendMenuW(hmenu, MF_STRING, 3, PCWSTR(w!("Settings...").as_ptr()));
        AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        AppendMenuW(hmenu, MF_STRING, 2, PCWSTR(w!("Exit").as_ptr()));

        // Check/Uncheck the Enable item
        CheckMenuItem(hmenu, 1, if enabled { MF_CHECKED.0 } else { MF_UNCHECKED.0 });

        // Enable/disable items based on state
        EnableMenuItem(hmenu, 1, MF_ENABLED);
        EnableMenuItem(hmenu, 2, MF_ENABLED);

        // Track popup menu
        let result = TrackPopupMenu(
            hmenu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x as i32,
            point.y as i32,
            0,
            self.hwnd,
            None
        );

        DestroyMenu(hmenu);

        if result.0 == 1 {
            // Enable/Disable
            self.toggle();
            info!("Gesture recognition {}", if (*self.enabled_ptr).load(Ordering::SeqCst) { "enabled" } else { "disabled" });
        } else if result.0 == 3 {
            // Settings - show() is non-blocking (sends to persistent UI thread)
            info!("Opening config dialog from menu");
            let dialog = ConfigDialog::new(self.config_path.clone());
            dialog.show(None);
        } else if result.0 == 2 {
            // Exit
            info!("Exit requested from tray menu");
            // Send shutdown signal to main thread
            if let Some(ref tx) = self.shutdown_tx {
                let _ = tx.send(());
            }
            // Also post quit message for message loop thread
            PostQuitMessage(0);
        }
    }

    /// Window procedure for tray icon messages
    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        const TRAY_ICON_MESSAGE: u32 = WM_USER + 100;

        match msg {
            TRAY_ICON_MESSAGE => {
                // Tray icon message
                let event = lparam.0 as u32;
                // info!("📨 Tray icon message received: event={}", event);
                match event {
                    WM_LBUTTONDBLCLK => {
                        info!("Tray icon double-clicked - opening config dialog");
                        let user_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowUserData;
                        if !user_data_ptr.is_null() {
                            let user_data = &*user_data_ptr;
                            let dialog = ConfigDialog::new(user_data.config_path.clone());
                            dialog.show(None);
                        }
                    }
                    WM_RBUTTONUP => {
                        info!("Tray icon right-clicked - showing context menu");
                        // Retrieve user data from window
                        let user_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowUserData;
                        if !user_data_ptr.is_null() {
                            let user_data = &*user_data_ptr;
                            let tray = TrayIcon {
                                enabled_ptr: user_data.enabled,
                                hwnd,
                                icon_added: false,
                                shutdown_tx: user_data.shutdown_tx.clone(),
                                config_path: user_data.config_path.clone(),
                            };
                            tray.show_context_menu();
                        }
                    }
                    WM_CONTEXTMENU => {
                        info!("Tray icon context menu - showing context menu");
                        // Retrieve user data from window
                        let user_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowUserData;
                        if !user_data_ptr.is_null() {
                            let user_data = &*user_data_ptr;
                            let tray = TrayIcon {
                                enabled_ptr: user_data.enabled,
                                hwnd,
                                icon_added: false,
                                shutdown_tx: user_data.shutdown_tx.clone(),
                                config_path: user_data.config_path.clone(),
                            };
                            tray.show_context_menu();
                        }
                    }
                    _ => {
                        // info!("🔔 Unknown tray icon event: {}", event);
                    }
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                // Clean up window user data
                let user_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowUserData;
                if !user_data_ptr.is_null() {
                    let user_data = &*user_data_ptr;
                    // Clean up enabled pointer
                    let _ = Arc::from_raw(user_data.enabled);
                    // Drop the Box<WindowUserData>
                    let _ = Box::from_raw(user_data_ptr as *mut WindowUserData);
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        info!("Removing system tray icon");
        if self.icon_added {
            unsafe {
                let mut nid = NOTIFYICONDATAW::default();
                nid.hWnd = self.hwnd;
                nid.uID = 1;
                nid.uFlags = NIF_ICON;

                if Shell_NotifyIconW(NIM_DELETE, &nid).as_bool() {
                    info!("System tray icon removed");
                } else {
                    warn!("Failed to remove system tray icon");
                }

                // Clean up enabled pointer
                let enabled_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *const AtomicBool;
                if !enabled_ptr.is_null() {
                    let _ = Arc::from_raw(enabled_ptr);
                }

                // Destroy window
                if !self.hwnd.is_invalid() {
                    DestroyWindow(self.hwnd);
                }
            }
        }
    }
}
