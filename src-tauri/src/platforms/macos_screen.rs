pub const MAIN_WINDOW_TOP_SAFE_INSET_PX: i32 = 76;
pub const DETAIL_POPUP_HITTEST_PAD_PX: i32 = 24;
pub const RECENT_MOUSE_DOWN_WINDOW_MS: f64 = 120.0;

pub fn main_window_top_safe_inset_px() -> i32 {
    #[cfg(target_os = "macos")]
    {
        MAIN_WINDOW_TOP_SAFE_INSET_PX
    }
    #[cfg(not(target_os = "macos"))]
    {
        0
    }
}

pub fn detail_popup_hittest_pad_px() -> i32 {
    DETAIL_POPUP_HITTEST_PAD_PX
}

pub fn recent_mouse_down_window_ms() -> f64 {
    RECENT_MOUSE_DOWN_WINDOW_MS
}

fn point_in_rect(px: i32, py: i32, x: i32, y: i32, w: u32, h: u32, pad: i32) -> bool {
    let left = x.saturating_sub(pad);
    let top = y.saturating_sub(pad);
    let right = x.saturating_add(w as i32).saturating_add(pad);
    let bottom = y.saturating_add(h as i32).saturating_add(pad);
    px >= left && px <= right && py >= top && py <= bottom
}

#[cfg(target_os = "macos")]
fn scaled_point(px: i32, py: i32, factor: f64) -> Option<(i32, i32)> {
    if !factor.is_finite() || factor <= 0.0 {
        return None;
    }
    let sx = (px as f64 * factor).round();
    let sy = (py as f64 * factor).round();
    if sx < i32::MIN as f64 || sx > i32::MAX as f64 || sy < i32::MIN as f64 || sy > i32::MAX as f64 {
        return None;
    }
    Some((sx as i32, sy as i32))
}

fn scaled_rect(x: i32, y: i32, w: u32, h: u32, factor: f64) -> Option<(i32, i32, u32, u32)> {
    if !factor.is_finite() || factor <= 0.0 {
        return None;
    }
    let sx = (x as f64 * factor).round();
    let sy = (y as f64 * factor).round();
    let sw = (w as f64 * factor).round().max(1.0);
    let sh = (h as f64 * factor).round().max(1.0);
    if sx < i32::MIN as f64 || sx > i32::MAX as f64 || sy < i32::MIN as f64 || sy > i32::MAX as f64 {
        return None;
    }
    if sw > u32::MAX as f64 || sh > u32::MAX as f64 {
        return None;
    }
    Some((sx as i32, sy as i32, sw as u32, sh as u32))
}

pub fn is_point_inside_window(window: &tauri::WebviewWindow, mx: i32, my: i32) -> bool {
    #[cfg(not(target_os = "macos"))]
    {
        let pad = detail_popup_hittest_pad_px();
        let mut rects: Vec<(i32, i32, u32, u32)> = Vec::new();

        if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
            rects.push((pos.x, pos.y, size.width, size.height));
        }
        if let (Ok(pos), Ok(size)) = (window.inner_position(), window.inner_size()) {
            rects.push((pos.x, pos.y, size.width, size.height));
        }
        if rects.is_empty() {
            return false;
        }

        let scale = window.scale_factor().ok().filter(|s| *s > 0.0).unwrap_or(1.0);
        let mut rect_candidates: Vec<(i32, i32, u32, u32)> = rects.clone();
        for (x, y, w, h) in &rects {
            if let Some(r) = scaled_rect(*x, *y, *w, *h, scale) {
                rect_candidates.push(r);
            }
            if let Some(r) = scaled_rect(*x, *y, *w, *h, 1.0 / scale.max(0.1)) {
                rect_candidates.push(r);
            }
        }

        return rect_candidates
            .iter()
            .any(|(x, y, w, h)| point_in_rect(mx, my, *x, *y, *w, *h, pad));
    }

    #[cfg(target_os = "macos")]
    {
    let pad = detail_popup_hittest_pad_px();

    let mut rects: Vec<(i32, i32, u32, u32)> = Vec::new();
    if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
        rects.push((pos.x, pos.y, size.width, size.height));
    }
    if let (Ok(pos), Ok(size)) = (window.inner_position(), window.inner_size()) {
        rects.push((pos.x, pos.y, size.width, size.height));
    }
    if rects.is_empty() {
        return false;
    }

    let scale = window.scale_factor().ok().filter(|s| *s > 0.0).unwrap_or(1.0);
    let mut point_candidates: Vec<(i32, i32)> = vec![(mx, my)];
    if let Some(p) = scaled_point(mx, my, 1.0 / scale) {
        point_candidates.push(p);
    }
    if let Some(p) = scaled_point(mx, my, scale) {
        point_candidates.push(p);
    }
    // Extra Retina fallback for environments that report inconsistent scale.
    if let Some(p) = scaled_point(mx, my, 0.5) {
        point_candidates.push(p);
    }
    if let Some(p) = scaled_point(mx, my, 2.0) {
        point_candidates.push(p);
    }

    let inset = main_window_top_safe_inset_px();
    point_candidates.push((mx, my.saturating_sub(inset)));
    point_candidates.push((mx, my.saturating_add(inset)));

    let mut rect_candidates: Vec<(i32, i32, u32, u32)> = rects.clone();
    for (x, y, w, h) in &rects {
        if let Some(r) = scaled_rect(*x, *y, *w, *h, scale) {
            rect_candidates.push(r);
        }
        if let Some(r) = scaled_rect(*x, *y, *w, *h, 1.0 / scale.max(0.1)) {
            rect_candidates.push(r);
        }
    }

    for (px, py) in point_candidates {
        if rect_candidates
            .iter()
            .any(|(x, y, w, h)| point_in_rect(px, py, *x, *y, *w, *h, pad))
        {
            return true;
        }
    }

    false
    }
}
