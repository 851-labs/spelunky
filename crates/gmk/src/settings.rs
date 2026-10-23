use crate::reader::{Reader, Result};

/// Global Game Settings (GM 8.0 layout, version 800).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Settings {
    pub start_fullscreen: bool,
    pub interpolate_pixels: bool,
    pub dont_draw_border: bool,
    pub display_cursor: bool,
    /// -1 keeps the aspect ratio, 0 scales to fill, a positive value is a fixed percentage.
    pub scaling: i32,
    pub allow_window_resize: bool,
    pub always_on_top: bool,
    /// Colour outside the room, as GM's `0x00BBGGRR`.
    pub color_outside_room: u32,
    pub set_resolution: bool,
    pub color_depth: i32,
    pub resolution: i32,
    pub frequency: i32,
    pub dont_show_buttons: bool,
    pub use_vsync: bool,
    pub disable_screensavers: bool,
    pub f4_fullscreen: bool,
    pub f1_game_info: bool,
    pub esc_ends_game: bool,
    pub f5_save_f6_load: bool,
    pub f9_screenshot: bool,
    pub close_as_esc: bool,
    pub priority: i32,
    pub freeze_on_lose_focus: bool,
    /// 0 none, 1 default, 2 custom images.
    pub load_bar_mode: i32,
    pub load_bar_back: Option<Vec<u8>>,
    pub load_bar_front: Option<Vec<u8>>,
    pub show_custom_load_image: bool,
    pub custom_load_image: Option<Vec<u8>>,
    pub load_image_partially_transparent: bool,
    pub load_image_alpha: i32,
    pub scale_progress_bar: bool,
    pub icon: Vec<u8>,
    pub display_errors: bool,
    pub write_errors_to_log: bool,
    pub abort_on_error: bool,
    pub treat_uninitialized_as_zero: bool,
    pub author: String,
    pub version: String,
    pub last_changed: f64,
    pub information: String,
    pub version_major: i32,
    pub version_minor: i32,
    pub version_release: i32,
    pub version_build: i32,
    pub company: String,
    pub product: String,
    pub copyright: String,
    pub description: String,
}

fn optional_blob(r: &mut Reader<'_>) -> Result<Option<Vec<u8>>> {
    Ok(if r.bool()? {
        Some(r.blob()?.to_vec())
    } else {
        None
    })
}

pub(crate) fn read(r: &mut Reader<'_>) -> Result<Settings> {
    r.version("settings", 800)?;
    let block = r.inflate("settings")?;
    let s = &mut block.reader();
    let start_fullscreen = s.bool()?;
    let interpolate_pixels = s.bool()?;
    let dont_draw_border = s.bool()?;
    let display_cursor = s.bool()?;
    let scaling = s.i32()?;
    let allow_window_resize = s.bool()?;
    let always_on_top = s.bool()?;
    let color_outside_room = s.u32()?;
    let set_resolution = s.bool()?;
    let color_depth = s.i32()?;
    let resolution = s.i32()?;
    let frequency = s.i32()?;
    let dont_show_buttons = s.bool()?;
    let use_vsync = s.u32()? & 1 != 0;
    let disable_screensavers = s.bool()?;
    let f4_fullscreen = s.bool()?;
    let f1_game_info = s.bool()?;
    let esc_ends_game = s.bool()?;
    let f5_save_f6_load = s.bool()?;
    let f9_screenshot = s.bool()?;
    let close_as_esc = s.bool()?;
    let priority = s.i32()?;
    let freeze_on_lose_focus = s.bool()?;
    let load_bar_mode = s.i32()?;
    let (load_bar_back, load_bar_front) = if load_bar_mode == 2 {
        (optional_blob(s)?, optional_blob(s)?)
    } else {
        (None, None)
    };
    let show_custom_load_image = s.bool()?;
    let custom_load_image = if show_custom_load_image {
        optional_blob(s)?
    } else {
        None
    };
    let load_image_partially_transparent = s.bool()?;
    let load_image_alpha = s.i32()?;
    let scale_progress_bar = s.bool()?;
    let icon = s.blob()?.to_vec();
    let display_errors = s.bool()?;
    let write_errors_to_log = s.bool()?;
    let abort_on_error = s.bool()?;
    let treat_uninitialized_as_zero = s.i32()? & 1 != 0;
    let author = s.string()?;
    let version = s.string()?;
    let last_changed = s.f64()?;
    let information = s.string()?;
    let version_major = s.i32()?;
    let version_minor = s.i32()?;
    let version_release = s.i32()?;
    let version_build = s.i32()?;
    let company = s.string()?;
    let product = s.string()?;
    let copyright = s.string()?;
    let description = s.string()?;
    s.f64()?; // a second "last changed" timestamp
    s.finish()?;
    Ok(Settings {
        start_fullscreen,
        interpolate_pixels,
        dont_draw_border,
        display_cursor,
        scaling,
        allow_window_resize,
        always_on_top,
        color_outside_room,
        set_resolution,
        color_depth,
        resolution,
        frequency,
        dont_show_buttons,
        use_vsync,
        disable_screensavers,
        f4_fullscreen,
        f1_game_info,
        esc_ends_game,
        f5_save_f6_load,
        f9_screenshot,
        close_as_esc,
        priority,
        freeze_on_lose_focus,
        load_bar_mode,
        load_bar_back,
        load_bar_front,
        show_custom_load_image,
        custom_load_image,
        load_image_partially_transparent,
        load_image_alpha,
        scale_progress_bar,
        icon,
        display_errors,
        write_errors_to_log,
        abort_on_error,
        treat_uninitialized_as_zero,
        author,
        version,
        last_changed,
        information,
        version_major,
        version_minor,
        version_release,
        version_build,
        company,
        product,
        copyright,
        description,
    })
}
