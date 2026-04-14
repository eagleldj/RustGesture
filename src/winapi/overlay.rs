//! 透明覆盖层窗口模块
//!
//! 实现 WGestures 风格的鼠标手势轨迹显示。
//! 在独立线程中创建一个透明分层窗口，使用 GDI + UpdateLayeredWindow 绘制轨迹和手势名称。

use std::sync::mpsc::{self, Receiver, Sender};
use tracing::{error, info, warn};
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
    /// 显示手势名称并启动淡出动画
    ShowName { name: String, x: i32, y: i32 },
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
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            color: 0x00FF9600, // 默认橙色 (COLORREF)
            width: 3,
            name: None,
            fade_alpha: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// 公共 API
// ---------------------------------------------------------------------------

/// 手势轨迹覆盖层
///
/// 在独立线程中运行一个透明的全屏分层窗口，
/// 通过 mpsc channel 接收绘制命令。
pub struct GestureOverlay {
    sender: Sender<OverlayCommand>,
}

impl GestureOverlay {
    /// 创建覆盖层窗口并启动后台渲染线程
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("gesture-overlay".into())
            .spawn(move || overlay_thread_main(rx))
            .expect("Failed to spawn overlay thread");
        Self { sender: tx }
    }

    /// 向覆盖层发送绘制命令（非阻塞）
    pub fn send(&self, cmd: OverlayCommand) {
        if let Err(e) = self.sender.send(cmd) {
            warn!("Overlay send failed: {}", e);
        }
    }

    /// 获取 sender 的克隆，用于在闭包中发送命令
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
        return 0x00FFFFFF; // 白色作为默认
    }
    let Ok(rgb) = u32::from_str_radix(s, 16) else {
        return 0x00FFFFFF;
    };
    let r = (rgb >> 16) & 0xFF;
    let g = (rgb >> 8) & 0xFF;
    let b = rgb & 0xFF;
    // COLORREF = 0x00BBGGRR
    (b << 16) | (g << 8) | r
}

/// 计算点集的包围盒 (left, top, right, bottom)
#[allow(dead_code)]
fn compute_bbox(points: &[(i32, i32)], padding: i32) -> (i32, i32, i32, i32) {
    if points.is_empty() {
        return (0, 0, 0, 0);
    }
    let mut min_x = points[0].0;
    let mut min_y = points[0].1;
    let mut max_x = points[0].0;
    let mut max_y = points[0].1;
    for &(x, y) in &points[1..] {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    (
        min_x - padding,
        min_y - padding,
        max_x + padding,
        max_y + padding,
    )
}

// ---------------------------------------------------------------------------
// 窗口类名
// ---------------------------------------------------------------------------

const OVERLAY_CLASS_NAME: &str = "RustGestureOverlay\0";

// ---------------------------------------------------------------------------
// Overlay 线程主函数
// ---------------------------------------------------------------------------

fn overlay_thread_main(rx: Receiver<OverlayCommand>) {
    let screen_w: i32;
    let screen_h: i32;
    let hwnd: HWND;

    unsafe {
        screen_w = GetSystemMetrics(SM_CXSCREEN);
        screen_h = GetSystemMetrics(SM_CYSCREEN);

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
            // Class already registered is fine (error 1410)
            if err.0 != 0 {
                info!("RegisterClassW returned {}, proceeding anyway", err.0);
            }
        }

        // 创建分层透明窗口
        let class_name_ptr = PCWSTR::from_raw(class_name.as_ptr());
        let window_name: Vec<u16> = "RustGestureOverlay\0".encode_utf16().collect();
        let window_name_ptr = PCWSTR::from_raw(window_name.as_ptr());

        let ex_style = WS_EX_LAYERED
            | WS_EX_TOPMOST
            | WS_EX_TRANSPARENT
            | WS_EX_TOOLWINDOW;

        hwnd = CreateWindowExW(
            ex_style,
            class_name_ptr,
            window_name_ptr,
            WS_POPUP,
            0,
            0,
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

        info!(
            "Overlay window created: {:?} ({}x{})",
            hwnd, screen_w, screen_h
        );
    }

    // 创建内存 DC 和 32 位 ARGB DIB section（双缓冲）
    unsafe {
        let screen_dc = GetDC(hwnd);
        let mem_dc = CreateCompatibleDC(screen_dc);

        // BITMAPINFOHEADER for 32-bit ARGB
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: screen_w,
                biHeight: -screen_h, // top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            ..std::mem::zeroed()
        };

        let mut p_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(
            screen_dc,
            &bmi,
            DIB_RGB_COLORS,
            &mut p_bits,
            HANDLE::default(),
            0,
        ) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to create DIB section: {:?}", e);
                ReleaseDC(hwnd, screen_dc);
                let _ = DestroyWindow(hwnd);
                return;
            }
        };

        if bitmap.is_invalid() || p_bits.is_null() {
            error!("Failed to create DIB section (invalid bitmap or null bits)");
            ReleaseDC(hwnd, screen_dc);
            let _ = DestroyWindow(hwnd);
            return;
        }

        ReleaseDC(hwnd, screen_dc);

        let bits_ptr = p_bits as *mut u8;

        // 消息循环 + 命令处理
        let mut state = OverlayState::default();
        let mut msg = MSG::default();

        loop {
            // 处理所有排队的 Windows 消息
            loop {
                let has_msg = PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE);
                if !has_msg.as_bool() {
                    break;
                }
                if msg.message == WM_TIMER {
                    // 淡出定时器
                    if state.fade_alpha > 0 {
                        if state.fade_alpha <= 32 {
                            state.fade_alpha = 0;
                        } else {
                            state.fade_alpha -= 32;
                        }
                        if state.fade_alpha == 0 {
                            let _ = KillTimer(hwnd, 1);
                            state.points.clear();
                            state.name = None;
                            clear_bitmap(bits_ptr, screen_w, screen_h);
                            update_layered(hwnd, mem_dc, bitmap, screen_w, screen_h);
                        } else {
                            render(
                                hwnd,
                                mem_dc,
                                bitmap,
                                bits_ptr,
                                screen_w,
                                screen_h,
                                &state,
                            );
                        }
                    }
                } else if msg.message == WM_QUIT {
                    cleanup(hwnd, mem_dc, bitmap);
                    return;
                } else {
                    let _ = TranslateMessage(&msg);
                    let _ = DispatchMessageW(&msg);
                }
            }

            // 非阻塞处理通道命令
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    OverlayCommand::StartTrail { x, y, color, width } => {
                        state.points.clear();
                        state.name = None;
                        state.fade_alpha = 0;
                        state.color = color;
                        state.width = width;
                        state.points.push((x, y));
                        let _ = KillTimer(hwnd, 1);
                        // 清空并绘制第一个点
                        clear_bitmap(bits_ptr, screen_w, screen_h);
                        update_layered(hwnd, mem_dc, bitmap, screen_w, screen_h);
                    }
                    OverlayCommand::TrailPoint { x, y } => {
                        // 去重：避免连续相同的点
                        if state.points.last() != Some(&(x, y)) {
                            state.points.push((x, y));
                        }
                        render(hwnd, mem_dc, bitmap, bits_ptr, screen_w, screen_h, &state);
                    }
                    OverlayCommand::ShowName { name, x, y } => {
                        state.name = Some((name, x, y));
                        state.fade_alpha = 255;
                        render(hwnd, mem_dc, bitmap, bits_ptr, screen_w, screen_h, &state);
                        // 启动淡出定时器 (50ms 间隔)
                        SetTimer(hwnd, 1, 50, None);
                    }
                    OverlayCommand::Clear => {
                        let _ = KillTimer(hwnd, 1);
                        state.points.clear();
                        state.name = None;
                        state.fade_alpha = 0;
                        clear_bitmap(bits_ptr, screen_w, screen_h);
                        update_layered(hwnd, mem_dc, bitmap, screen_w, screen_h);
                    }
                    OverlayCommand::Shutdown => {
                        let _ = KillTimer(hwnd, 1);
                        cleanup(hwnd, mem_dc, bitmap);
                        info!("Overlay thread shutting down");
                        return;
                    }
                }
            }

            // 短暂休眠避免空转 CPU
            std::thread::sleep(std::time::Duration::from_millis(4));
        }
    }
}

// ---------------------------------------------------------------------------
// 绘制函数
// ---------------------------------------------------------------------------

/// 清零整个 bitmap（全透明）
unsafe fn clear_bitmap(bits: *mut u8, w: i32, h: i32) {
    let size = (w * h * 4) as usize;
    std::ptr::write_bytes(bits, 0u8, size);
}

/// 渲染当前状态到 bitmap 并通过 UpdateLayeredWindow 提交
unsafe fn render(
    hwnd: HWND,
    mem_dc: HDC,
    bitmap: HBITMAP,
    bits: *mut u8,
    w: i32,
    h: i32,
    state: &OverlayState,
) {
    // 1. 清零 bitmap
    clear_bitmap(bits, w, h);

    // 2. Select bitmap
    let old_bmp = SelectObject(mem_dc, bitmap);

    // 3. 绘制轨迹折线
    if state.points.len() >= 2 {
        // PS_SOLID | PS_ENDCAP_ROUND  — 两者值恰好相同(0|0=0)，直接用 PS_SOLID
        let pen = CreatePen(PS_SOLID, state.width as i32, COLORREF(state.color));
        let old_pen = SelectObject(mem_dc, pen);
        let old_brush = SelectObject(mem_dc, GetStockObject(NULL_BRUSH));

        let (px0, py0) = state.points[0];
        let _ = MoveToEx(mem_dc, px0, py0, None);
        for &(px, py) in &state.points[1..] {
            let _ = LineTo(mem_dc, px, py);
        }

        SelectObject(mem_dc, old_brush);
        SelectObject(mem_dc, old_pen);
        let _ = DeleteObject(pen);
    } else if state.points.len() == 1 {
        // 单个点：画一个小圆点
        let (px, py) = state.points[0];
        let r = state.width as i32;
        let brush = CreateSolidBrush(COLORREF(state.color));
        let old_brush = SelectObject(mem_dc, brush);
        let old_pen = SelectObject(mem_dc, GetStockObject(NULL_PEN));
        let _ = Ellipse(mem_dc, px - r, py - r, px + r + 1, py + r + 1);
        SelectObject(mem_dc, old_pen);
        SelectObject(mem_dc, old_brush);
        let _ = DeleteObject(brush);
    }

    // 4. 绘制名称标签
    if let Some((ref name, nx, ny)) = state.name {
        draw_name_label(mem_dc, name, nx, ny);
    }

    // 5. Select 回原 bitmap
    SelectObject(mem_dc, old_bmp);

    // 6. 设置 alpha 通道（预乘 alpha）
    apply_alpha(bits, w, h, state.fade_alpha);

    // 7. 提交
    update_layered(hwnd, mem_dc, bitmap, w, h);
}

/// 绘制手势名称标签（圆角矩形背景 + 白色文字）
unsafe fn draw_name_label(dc: HDC, name: &str, x: i32, y: i32) {
    let text_y = y - 20; // 在指定位置上方 20px

    let mut text_wide: Vec<u16> = name.encode_utf16().collect();
    // DrawTextW 需要 null terminator
    text_wide.push(0);
    if text_wide.len() <= 1 {
        return;
    }

    // 计算文字大小
    let mut text_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // DrawTextW 签名: (hdc, &mut [u16], *mut RECT, format)
    DrawTextW(
        dc,
        &mut text_wide,
        &mut text_rect as *mut _,
        DT_CALCRECT | DT_SINGLELINE,
    );

    let text_w = text_rect.right - text_rect.left;
    let text_h = text_rect.bottom - text_rect.top;

    // 标签背景区域（加 padding）
    let pad = 8;
    let bg_left = x - text_w / 2 - pad;
    let bg_top = text_y - text_h / 2 - pad;
    let bg_right = x + text_w / 2 + pad;
    let bg_bottom = text_y + text_h / 2 + pad;

    // 画半透明黑色圆角矩形背景
    let bg_brush = CreateSolidBrush(COLORREF(0x00202020)); // 深灰
    let old_brush = SelectObject(dc, bg_brush);
    let old_pen = SelectObject(dc, GetStockObject(NULL_PEN));
    let _ = RoundRect(dc, bg_left, bg_top, bg_right, bg_bottom, 8, 8);
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    let _ = DeleteObject(bg_brush);

    // 画白色文字
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, COLORREF(0x00FFFFFF)); // 白色

    let mut draw_rect = RECT {
        left: x - text_w / 2,
        top: text_y - text_h / 2,
        right: x + text_w / 2,
        bottom: text_y + text_h / 2,
    };
    DrawTextW(
        dc,
        &mut text_wide,
        &mut draw_rect as *mut _,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );
}

/// 对 bounding box 区域的像素设置 alpha 通道（预乘 alpha）
unsafe fn apply_alpha(bits: *mut u8, w: i32, h: i32, fade_alpha: u8) {
    // 优化：只处理包含内容的区域
    let total = (w * h) as usize;
    let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, total);

    // 快速扫描：找到第一个和最后一个非零像素的位置来缩小区间
    let mut first = total;
    let mut last = 0usize;
    for (i, &px) in pixels.iter().enumerate() {
        if px != 0 {
            let r = ((px >> 16) & 0xFF) as u8;
            let g = ((px >> 8) & 0xFF) as u8;
            let b = (px & 0xFF) as u8;
            if r != 0 || g != 0 || b != 0 {
                if i < first { first = i; }
                if i > last { last = i; }
            }
        }
    }

    if first > last {
        return; // 没有内容需要处理
    }

    // 计算扫描行范围
    let row_first = first / (w as usize);
    let row_last = last / (w as usize);
    let row_len = w as usize;

    for row in row_first..=row_last {
        let start = row * row_len;
        let end = start + row_len;
        for i in start..end {
            let argb = pixels[i];
            if argb == 0 {
                continue;
            }
            let b = (argb & 0xFF) as u8;
            let g = ((argb >> 8) & 0xFF) as u8;
            let r = ((argb >> 16) & 0xFF) as u8;

            if r == 0 && g == 0 && b == 0 {
                continue;
            }

            let a = ((255u16 * fade_alpha as u16) >> 8) as u8;
            let pr = ((r as u16 * a as u16) >> 8) as u8;
            let pg = ((g as u16 * a as u16) >> 8) as u8;
            let pb = ((b as u16 * a as u16) >> 8) as u8;

            pixels[i] = ((a as u32) << 24) | ((pr as u32) << 16) | ((pg as u32) << 8) | (pb as u32);
        }
    }
}

/// 通过 UpdateLayeredWindow 提交 bitmap 到分层窗口
unsafe fn update_layered(
    hwnd: HWND,
    mem_dc: HDC,
    bitmap: HBITMAP,
    w: i32,
    h: i32,
) {
    let screen_dc = GetDC(HWND::default());
    let old_bmp = SelectObject(mem_dc, bitmap);

    let pt_dst = POINT { x: 0, y: 0 };
    let pt_src = POINT { x: 0, y: 0 };
    let size = SIZE { cx: w, cy: h };

    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };

    let _ = UpdateLayeredWindow(
        hwnd,
        screen_dc,
        Some(&pt_dst),
        Some(&size),
        mem_dc,
        Some(&pt_src),
        COLORREF(0),
        Some(&blend),
        ULW_ALPHA,
    );

    SelectObject(mem_dc, old_bmp);
    ReleaseDC(HWND::default(), screen_dc);
}

/// 清理 GDI 资源并销毁窗口
unsafe fn cleanup(hwnd: HWND, mem_dc: HDC, bitmap: HBITMAP) {
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(mem_dc);
    let _ = DestroyWindow(hwnd);
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
        // "#RRGGBB" -> COLORREF 0x00BBGGRR
        assert_eq!(parse_hex_color("#FF0000"), 0x000000FF); // 红
        assert_eq!(parse_hex_color("#00FF00"), 0x0000FF00); // 绿
        assert_eq!(parse_hex_color("#0000FF"), 0x00FF0000); // 蓝
        assert_eq!(parse_hex_color("#FFFFFF"), 0x00FFFFFF); // 白
        assert_eq!(parse_hex_color("#0096FF"), 0x00FF9600); // 青蓝
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert_eq!(parse_hex_color("invalid"), 0x00FFFFFF); // 回退白色
        assert_eq!(parse_hex_color("#FFF"), 0x00FFFFFF); // 太短
    }

    #[test]
    fn test_compute_bbox() {
        let points = vec![(10, 20), (30, 40), (50, 10)];
        let (l, t, r, b) = compute_bbox(&points, 5);
        assert_eq!(l, 5);
        assert_eq!(t, 5);
        assert_eq!(r, 55);
        assert_eq!(b, 45);
    }

    #[test]
    fn test_compute_bbox_empty() {
        let (l, t, r, b) = compute_bbox(&[], 10);
        assert_eq!((l, t, r, b), (0, 0, 0, 0));
    }

    #[test]
    fn test_compute_bbox_single_point() {
        let points = vec![(100, 200)];
        let (l, t, r, b) = compute_bbox(&points, 10);
        assert_eq!((l, t, r, b), (90, 190, 110, 210));
    }
}
