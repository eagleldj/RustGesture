//! 透明覆盖层窗口模块
//!
//! 实现 WGestures 风格的鼠标手势轨迹显示。
//! 在独立线程中创建一个透明分层窗口，使用 SetLayeredWindowAttributes + GDI 绘制轨迹和手势名称。

use std::sync::mpsc::{self, Receiver, Sender};
use tracing::{debug, error, info, warn};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;

// ---------------------------------------------------------------------------
// 公共类型
// ---------------------------------------------------------------------------

/// 发送给覆盖层线程的绘制命令
pub enum OverlayCommand {
    /// 开始新轨迹（重置状态，设置颜色和线宽）
    StartTrail { x: i32, y: i32, color: u32, width: u32 },
    /// 追加轨迹点并立即重绘
    TrailPoint { x: i32, y: i32 },
    /// 显示手势名称（在屏幕中央底部），并启动淡出动画
    ShowName { name: String },
    /// 隐藏名称标签（保留轨迹）
    HideName,
    /// 启动淡出动画（保留轨迹和名称，自然消失）
    FadeOut,
    /// 立即清空并停止淡出定时器
    Clear,
    /// 关闭窗口并退出线程
    Shutdown,
}

/// 覆盖层状态（仅在 overlay 线程中访问）
struct OverlayState {
    points: Vec<(i32, i32)>,
    color: u32,
    width: u32,
    name: Option<(String, i32, i32)>,
    fade_alpha: u8,
    /// 淡出前的等待计数（每次 timer tick 减 1，到 0 才开始减 alpha）
    fade_wait: u8,
    /// 是否期望收到 WM_TIMER（防止 KillTimer 后残留消息被误处理）
    fading: bool,
    /// 窗口偏移（虚拟屏幕原点，用于坐标转换）
    origin_x: i32,
    origin_y: i32,
    /// 窗口尺寸
    screen_w: i32,
    screen_h: i32,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            color: 0x00FF9600,
            width: 3,
            name: None,
            fade_alpha: 0,
            fade_wait: 0,
            fading: false,
            origin_x: 0,
            origin_y: 0,
            screen_w: 0,
            screen_h: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// 公共 API
// ---------------------------------------------------------------------------

/// 手势轨迹覆盖层
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
        if let Err(e) = self.sender.send(cmd) {
            warn!("Overlay send failed: {}", e);
        }
    }

    pub fn sender(&self) -> Sender<OverlayCommand> {
        self.sender.clone()
    }
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 解析 "#RRGGBB" 格式的颜色字符串为 Windows COLORREF (0x00BBGGRR)
pub fn parse_hex_color(s: &str) -> u32 {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return 0x00FFFFFF;
    }
    let Ok(rgb) = u32::from_str_radix(s, 16) else {
        return 0x00FFFFFF;
    };
    let r = (rgb >> 16) & 0xFF;
    let g = (rgb >> 8) & 0xFF;
    let b = rgb & 0xFF;
    (b << 16) | (g << 8) | r
}

// ---------------------------------------------------------------------------
// 窗口类名
// ---------------------------------------------------------------------------

const OVERLAY_CLASS_NAME: &str = "RustGestureOverlay\0";

/// 组合标志：COLORKEY 让黑色透明，ALPHA 控制整体不透明度
const OVERLAY_LAYERED_FLAGS: LAYERED_WINDOW_ATTRIBUTES_FLAGS = LAYERED_WINDOW_ATTRIBUTES_FLAGS(LWA_COLORKEY.0 | LWA_ALPHA.0);


// ---------------------------------------------------------------------------
// Overlay 线程主函数
// ---------------------------------------------------------------------------

fn overlay_thread_main(rx: Receiver<OverlayCommand>) {
    let screen_w: i32;
    let screen_h: i32;
    let screen_x: i32;
    let screen_y: i32;
    let hwnd: HWND;

    unsafe {
        // 虚拟屏幕覆盖所有显示器
        screen_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        screen_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        screen_w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        screen_h = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        // 注册窗口类
        let class_name: Vec<u16> = OVERLAY_CLASS_NAME.encode_utf16().collect();
        let hinstance: HINSTANCE =
            windows::Win32::System::LibraryLoader::GetModuleHandleW(PCWSTR::null())
                .map(Into::into)
                .unwrap_or_default();

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(def_window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
        };

        if RegisterClassW(&wc) == 0 {
            let err = GetLastError();
            if err.0 != 0 {
                info!("RegisterClassW returned {}, proceeding anyway", err.0);
            }
        }

        let class_name_ptr = PCWSTR::from_raw(class_name.as_ptr());
        let window_name: Vec<u16> = "RustGestureOverlay\0".encode_utf16().collect();
        let window_name_ptr = PCWSTR::from_raw(window_name.as_ptr());

        let ex_style = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW;

        hwnd = CreateWindowExW(
            ex_style,
            class_name_ptr,
            window_name_ptr,
            WS_POPUP,
            screen_x,
            screen_y,
            screen_w,
            screen_h,
            HWND::default(),
            HMENU::default(),
            hinstance,
            None,
        )
        .unwrap_or_default();

        if hwnd.is_invalid() || hwnd == HWND::default() {
            error!("Failed to create overlay window");
            return;
        }

        // 显示窗口并设为置顶
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        // 初始状态：alpha=255，但全黑窗口在 COLORKEY 下不可见
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, OVERLAY_LAYERED_FLAGS);

        info!(
            "Overlay window created: {:?} at ({},{}) size {}x{}",
            hwnd, screen_x, screen_y, screen_w, screen_h
        );
    }

    // 消息循环 + 命令处理
    unsafe {
        let mut state = OverlayState {
            origin_x: screen_x,
            origin_y: screen_y,
            screen_w,
            screen_h,
            ..OverlayState::default()
        };
        let mut msg = MSG::default();

        loop {
            // 处理 Windows 消息
            loop {
                let has_msg = PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE);
                if !has_msg.as_bool() {
                    break;
                }
                if msg.message == WM_TIMER {
                    // 忽略残留的 WM_TIMER（KillTimer 不清除队列中已有消息）
                    if !state.fading {
                        continue;
                    }
                    if state.fade_alpha > 0 {
                        // 先短暂等待再快速淡出
                        if state.fade_wait > 0 {
                            state.fade_wait -= 1;
                        } else if state.fade_alpha <= 32 {
                            state.fade_alpha = 0;
                        } else {
                            state.fade_alpha -= 32;
                        }
                        if state.fade_alpha == 0 {
                            let _ = KillTimer(hwnd, 1);
                            state.points.clear();
                            state.name = None;
                            state.fade_wait = 0;
                            state.fading = false;
                            // 全黑填充在 COLORKEY 下透明，保持 alpha=255 避免合成器丢弃窗口
                            state.fade_alpha = 255;
                            paint(hwnd, &state);
                        } else {
                            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), state.fade_alpha, OVERLAY_LAYERED_FLAGS);
                            paint(hwnd, &state);
                        }
                    }
                } else if msg.message == WM_QUIT {
                    let _ = DestroyWindow(hwnd);
                    return;
                } else {
                    let _ = TranslateMessage(&msg);
                    let _ = DispatchMessageW(&msg);
                }
            }

            // 非阻塞处理通道命令
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    OverlayCommand::StartTrail { x: _, y: _, color, width } => {
                        let mut pt = POINT::default();
                        let _ = GetCursorPos(&mut pt);
                        // 将窗口移到光标所在的显示器上，确保窗口和光标在同一 DPI 空间
                        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
                        let mut mi = MONITORINFO::default();
                        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                        let _ = GetMonitorInfoW(hmon, &mut mi);
                        let mr = mi.rcMonitor;
                        let mw = mr.right - mr.left;
                        let mh = mr.bottom - mr.top;
                        let _ = MoveWindow(hwnd, mr.left, mr.top, mw, mh, false);
                        state.origin_x = mr.left;
                        state.origin_y = mr.top;
                        state.screen_w = mw;
                        state.screen_h = mh;
                        // 窗口已在同一显示器，ScreenToClient 正确工作
                        let _ = ScreenToClient(hwnd, &mut pt);
                        state.points.clear();
                        state.name = None;
                        state.fade_alpha = 255;
                        state.fade_wait = 0;
                        state.fading = false;
                        state.color = color;
                        state.width = width;
                        state.points.push((pt.x, pt.y));
                        let _ = KillTimer(hwnd, 1);
                        let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                        paint(hwnd, &state);
                    }
                    OverlayCommand::TrailPoint { x: _, y: _ } => {
                        let mut pt = POINT::default();
                        let _ = GetCursorPos(&mut pt);
                        let _ = ScreenToClient(hwnd, &mut pt);
                        if state.points.last() != Some(&(pt.x, pt.y)) {
                            state.points.push((pt.x, pt.y));
                        }
                        paint(hwnd, &state);
                    }
                    OverlayCommand::ShowName { name } => {
                        let cx = state.screen_w / 2;
                        let cy = state.screen_h - 60;
                        state.name = Some((name, cx, cy));
                        state.fade_alpha = 255;
                        let _ = KillTimer(hwnd, 1);
                        paint(hwnd, &state);
                    }
                    OverlayCommand::HideName => {
                        state.name = None;
                        paint(hwnd, &state);
                    }
                    OverlayCommand::FadeOut => {
                        if state.fade_alpha > 0 {
                            let _ = KillTimer(hwnd, 1);
                            state.fade_wait = 2; // 短暂保持后快速淡出
                            state.fading = true;
                            SetTimer(hwnd, 1, 30, None);
                        }
                    }
                    OverlayCommand::Clear => {
                        let _ = KillTimer(hwnd, 1);
                        state.points.clear();
                        state.name = None;
                        state.fade_alpha = 255;
                        state.fade_wait = 0;
                        state.fading = false;
                        paint(hwnd, &state);
                    }
                    OverlayCommand::Shutdown => {
                        let _ = KillTimer(hwnd, 1);
                        let _ = DestroyWindow(hwnd);
                        info!("Overlay thread shutting down");
                        return;
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(4));
        }
    }
}

// ---------------------------------------------------------------------------
// 绘制函数
// ---------------------------------------------------------------------------

/// 使用普通 GDI 在窗口上绘制轨迹和名称，通过 SetLayeredWindowAttributes 控制整体透明度
/// 黑色 (RGB 0,0,0) 是 LWA_COLORKEY 的透明色，填充黑色区域即为透明
unsafe fn paint(hwnd: HWND, state: &OverlayState) {
    let hdc = GetDC(hwnd);

    // 清空窗口（用黑色填充作为透明背景）
    let black_brush = CreateSolidBrush(COLORREF(0));
    let mut client_rect = RECT::default();
    let _ = GetClientRect(hwnd, &mut client_rect);
    let _ = FillRect(hdc, &client_rect, black_brush);
    let _ = DeleteObject(black_brush);

    if state.points.is_empty() && state.name.is_none() {
        ReleaseDC(hwnd, hdc);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), state.fade_alpha, OVERLAY_LAYERED_FLAGS);
        return;
    }

    // 绘制轨迹折线
    if state.points.len() >= 2 {
        let pen = CreatePen(PS_SOLID, state.width as i32, COLORREF(state.color));
        let old_pen = SelectObject(hdc, pen);
        let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));

        let (px0, py0) = state.points[0];
        let _ = MoveToEx(hdc, px0, py0, None);
        for &(px, py) in &state.points[1..] {
            let _ = LineTo(hdc, px, py);
        }

        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen);
    } else if state.points.len() == 1 {
        let (px, py) = state.points[0];
        let r = state.width as i32;
        let brush = CreateSolidBrush(COLORREF(state.color));
        let old_brush = SelectObject(hdc, brush);
        let old_pen = SelectObject(hdc, GetStockObject(NULL_PEN));
        let _ = Ellipse(hdc, px - r, py - r, px + r + 1, py + r + 1);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        let _ = DeleteObject(brush);
    }

    // 绘制名称标签
    if let Some((ref name, nx, ny)) = state.name {
        draw_name_label(hdc, name, nx, ny);
    }

    ReleaseDC(hwnd, hdc);

    // 设置透明度
    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), state.fade_alpha, OVERLAY_LAYERED_FLAGS);
}

/// 绘制手势名称标签（圆角矩形背景 + 白色文字）
unsafe fn draw_name_label(dc: HDC, name: &str, x: i32, y: i32) {
    let text_y = y - 20;

    let mut text_wide: Vec<u16> = name.encode_utf16().collect();
    text_wide.push(0);
    if text_wide.len() <= 1 {
        return;
    }

    let mut text_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    DrawTextW(dc, &mut text_wide, &mut text_rect as *mut _, DT_CALCRECT | DT_SINGLELINE);

    let text_w = text_rect.right - text_rect.left;
    let text_h = text_rect.bottom - text_rect.top;

    let pad = 8;
    let bg_left = x - text_w / 2 - pad;
    let bg_top = text_y - text_h / 2 - pad;
    let bg_right = x + text_w / 2 + pad;
    let bg_bottom = text_y + text_h / 2 + pad;

    let bg_brush = CreateSolidBrush(COLORREF(0x00202020));
    let old_brush = SelectObject(dc, bg_brush);
    let old_pen = SelectObject(dc, GetStockObject(NULL_PEN));
    let _ = RoundRect(dc, bg_left, bg_top, bg_right, bg_bottom, 8, 8);
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    let _ = DeleteObject(bg_brush);

    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, COLORREF(0x00FFFFFF));

    let mut draw_rect = RECT {
        left: x - text_w / 2,
        top: text_y - text_h / 2,
        right: x + text_w / 2,
        bottom: text_y + text_h / 2,
    };
    DrawTextW(dc, &mut text_wide, &mut draw_rect as *mut _, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
}

/// 默认窗口过程
unsafe extern "system" fn def_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#FF0000"), 0x000000FF);
        assert_eq!(parse_hex_color("#00FF00"), 0x0000FF00);
        assert_eq!(parse_hex_color("#0000FF"), 0x00FF0000);
        assert_eq!(parse_hex_color("#FFFFFF"), 0x00FFFFFF);
        assert_eq!(parse_hex_color("#0096FF"), 0x00FF9600);
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert_eq!(parse_hex_color("invalid"), 0x00FFFFFF);
        assert_eq!(parse_hex_color("#FFF"), 0x00FFFFFF);
    }
}
