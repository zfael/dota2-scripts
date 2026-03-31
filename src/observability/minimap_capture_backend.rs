#[derive(Debug, Clone)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub enum CaptureBackendResult {
    WindowNotFound,
    CaptureError(String),
    Success {
        window_rect: WindowRect,
        pixels: Vec<u8>,
        width: u32,
        height: u32,
    },
}

/// Find the Dota 2 window and return its client-area rect.
/// Returns `WindowNotFound` if the window cannot be located.
pub fn find_dota2_window_rect() -> CaptureBackendResult {
    #[cfg(windows)]
    {
        find_dota2_window_rect_win32()
    }
    #[cfg(not(windows))]
    {
        CaptureBackendResult::WindowNotFound
    }
}

#[cfg(windows)]
fn find_dota2_window_rect_win32() -> CaptureBackendResult {
    use windows::core::w;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetClientRect, IsWindow};

    let hwnd = match unsafe { FindWindowW(None, w!("Dota 2")) } {
        Ok(h) => h,
        Err(_) => return CaptureBackendResult::WindowNotFound,
    };

    let is_valid = unsafe { IsWindow(hwnd) };
    if !is_valid.as_bool() {
        return CaptureBackendResult::WindowNotFound;
    }

    let mut rect = RECT::default();
    let ok = unsafe { GetClientRect(hwnd, &mut rect) };

    match ok {
        Ok(()) => CaptureBackendResult::Success {
            window_rect: WindowRect {
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left) as u32,
                height: (rect.bottom - rect.top) as u32,
            },
            pixels: Vec::new(),
            width: 0,
            height: 0,
        },
        Err(e) => CaptureBackendResult::CaptureError(format!("GetClientRect failed: {}", e)),
    }
}

/// Capture a sub-region of the Dota 2 window using BitBlt.
///
/// `region_x`, `region_y`, `region_width`, `region_height` are relative to the
/// Dota 2 client area.
///
/// Returns raw RGBA pixel data on success.
pub fn capture_window_region(
    region_x: u32,
    region_y: u32,
    region_width: u32,
    region_height: u32,
) -> CaptureBackendResult {
    if region_width == 0 || region_height == 0 {
        return CaptureBackendResult::CaptureError(
            "capture region has zero width or height".to_string(),
        );
    }

    #[cfg(windows)]
    {
        capture_window_region_win32(region_x, region_y, region_width, region_height)
    }
    #[cfg(not(windows))]
    {
        let _ = (region_x, region_y, region_width, region_height);
        CaptureBackendResult::WindowNotFound
    }
}

#[cfg(windows)]
fn capture_window_region_win32(
    region_x: u32,
    region_y: u32,
    region_width: u32,
    region_height: u32,
) -> CaptureBackendResult {
    use windows::core::w;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };
    use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS, PW_CLIENTONLY};
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetClientRect, IsWindow};

    // PW_RENDERFULLCONTENT (0x2) captures DWM-composed content including GPU surfaces
    const PW_RENDERFULLCONTENT: u32 = 2;

    // Find the Dota 2 window
    let hwnd = match unsafe { FindWindowW(None, w!("Dota 2")) } {
        Ok(h) => h,
        Err(_) => return CaptureBackendResult::WindowNotFound,
    };

    let is_valid = unsafe { IsWindow(hwnd) };
    if !is_valid.as_bool() {
        return CaptureBackendResult::WindowNotFound;
    }

    // Get client rect to validate region bounds
    let mut client_rect = RECT::default();
    if let Err(e) = unsafe { GetClientRect(hwnd, &mut client_rect) } {
        return CaptureBackendResult::CaptureError(format!("GetClientRect failed: {}", e));
    }

    let client_width = (client_rect.right - client_rect.left) as u32;
    let client_height = (client_rect.bottom - client_rect.top) as u32;

    if region_x + region_width > client_width || region_y + region_height > client_height {
        return CaptureBackendResult::CaptureError(format!(
            "capture region ({}+{}, {}+{}) exceeds client area ({}x{})",
            region_x, region_width, region_y, region_height, client_width, client_height
        ));
    }

    // Get a screen DC for creating compatible objects
    let hdc_screen = unsafe { GetDC(HWND::default()) };
    if hdc_screen.is_invalid() {
        return CaptureBackendResult::CaptureError(
            "GetDC(screen) returned invalid handle".to_string(),
        );
    }

    // Create full-window DC and bitmap for PrintWindow
    let hdc_full = unsafe { CreateCompatibleDC(hdc_screen) };
    if hdc_full.is_invalid() {
        unsafe { ReleaseDC(HWND::default(), hdc_screen) };
        return CaptureBackendResult::CaptureError("CreateCompatibleDC (full) failed".to_string());
    }

    let hbm_full = unsafe {
        CreateCompatibleBitmap(hdc_screen, client_width as i32, client_height as i32)
    };
    if hbm_full.is_invalid() {
        unsafe {
            let _ = DeleteDC(hdc_full);
            ReleaseDC(HWND::default(), hdc_screen);
        };
        return CaptureBackendResult::CaptureError(
            "CreateCompatibleBitmap (full) failed".to_string(),
        );
    }

    let old_full = unsafe { SelectObject(hdc_full, hbm_full) };

    // Capture the full client area directly from the window via DWM
    let print_ok = unsafe {
        PrintWindow(
            hwnd,
            hdc_full,
            PRINT_WINDOW_FLAGS(PW_CLIENTONLY.0 | PW_RENDERFULLCONTENT),
        )
    };

    if !print_ok.as_bool() {
        unsafe {
            SelectObject(hdc_full, old_full);
            let _ = DeleteObject(hbm_full);
            let _ = DeleteDC(hdc_full);
            ReleaseDC(HWND::default(), hdc_screen);
        };
        return CaptureBackendResult::CaptureError("PrintWindow failed".to_string());
    }

    // Create region DC and bitmap for the minimap sub-region
    let hdc_region = unsafe { CreateCompatibleDC(hdc_screen) };
    if hdc_region.is_invalid() {
        unsafe {
            SelectObject(hdc_full, old_full);
            let _ = DeleteObject(hbm_full);
            let _ = DeleteDC(hdc_full);
            ReleaseDC(HWND::default(), hdc_screen);
        };
        return CaptureBackendResult::CaptureError(
            "CreateCompatibleDC (region) failed".to_string(),
        );
    }

    let hbm_region =
        unsafe { CreateCompatibleBitmap(hdc_screen, region_width as i32, region_height as i32) };
    if hbm_region.is_invalid() {
        unsafe {
            SelectObject(hdc_full, old_full);
            let _ = DeleteObject(hbm_full);
            let _ = DeleteDC(hdc_full);
            let _ = DeleteDC(hdc_region);
            ReleaseDC(HWND::default(), hdc_screen);
        };
        return CaptureBackendResult::CaptureError(
            "CreateCompatibleBitmap (region) failed".to_string(),
        );
    }

    let old_region = unsafe { SelectObject(hdc_region, hbm_region) };

    // BitBlt the minimap region from the full capture
    let blt_result = unsafe {
        BitBlt(
            hdc_region,
            0,
            0,
            region_width as i32,
            region_height as i32,
            hdc_full,
            region_x as i32,
            region_y as i32,
            SRCCOPY,
        )
    };

    if let Err(e) = blt_result {
        unsafe {
            SelectObject(hdc_region, old_region);
            let _ = DeleteObject(hbm_region);
            let _ = DeleteDC(hdc_region);
            SelectObject(hdc_full, old_full);
            let _ = DeleteObject(hbm_full);
            let _ = DeleteDC(hdc_full);
            ReleaseDC(HWND::default(), hdc_screen);
        };
        return CaptureBackendResult::CaptureError(format!("BitBlt failed: {}", e));
    }

    // Extract pixel data from the region bitmap
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: region_width as i32,
            biHeight: -(region_height as i32), // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [Default::default()],
    };

    let mut pixels = vec![0u8; (region_width * region_height * 4) as usize];

    let lines = unsafe {
        GetDIBits(
            hdc_region,
            hbm_region,
            0,
            region_height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };

    // Cleanup all GDI resources
    unsafe {
        SelectObject(hdc_region, old_region);
        let _ = DeleteObject(hbm_region);
        let _ = DeleteDC(hdc_region);
        SelectObject(hdc_full, old_full);
        let _ = DeleteObject(hbm_full);
        let _ = DeleteDC(hdc_full);
        ReleaseDC(HWND::default(), hdc_screen);
    };

    if lines == 0 {
        return CaptureBackendResult::CaptureError("GetDIBits returned 0 lines".to_string());
    }

    // Convert BGRA → RGBA (swap B and R channels)
    for chunk in pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }

    let window_rect = WindowRect {
        x: client_rect.left,
        y: client_rect.top,
        width: client_width,
        height: client_height,
    };

    CaptureBackendResult::Success {
        window_rect,
        pixels,
        width: region_width,
        height: region_height,
    }
}
