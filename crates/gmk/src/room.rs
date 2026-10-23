use crate::reader::{Reader, Result};
use crate::resources::read_section;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoomBackground {
    pub visible: bool,
    pub foreground: bool,
    /// Background id, or -1.
    pub background: i32,
    pub x: i32,
    pub y: i32,
    pub tile_h: bool,
    pub tile_v: bool,
    pub hspeed: i32,
    pub vspeed: i32,
    pub stretch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct View {
    pub visible: bool,
    pub view_x: i32,
    pub view_y: i32,
    pub view_w: i32,
    pub view_h: i32,
    pub port_x: i32,
    pub port_y: i32,
    pub port_w: i32,
    pub port_h: i32,
    pub hborder: i32,
    pub vborder: i32,
    pub hspeed: i32,
    pub vspeed: i32,
    /// Object id to follow, or -1.
    pub follow_object: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Instance {
    pub x: i32,
    pub y: i32,
    pub object: i32,
    pub id: i32,
    pub creation_code: String,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tile {
    pub x: i32,
    pub y: i32,
    pub background: i32,
    pub bg_x: i32,
    pub bg_y: i32,
    pub width: i32,
    pub height: i32,
    pub depth: i32,
    pub id: i32,
    pub locked: bool,
}

/// Room-editor state saved with the room. Not used by the runner.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoomEditorSettings {
    pub remember_window_size: bool,
    pub width: i32,
    pub height: i32,
    pub show_grid: bool,
    pub show_objects: bool,
    pub show_tiles: bool,
    pub show_backgrounds: bool,
    pub show_foregrounds: bool,
    pub show_views: bool,
    pub delete_underlying_objects: bool,
    pub delete_underlying_tiles: bool,
    pub tab: i32,
    pub scroll_x: i32,
    pub scroll_y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Room {
    pub name: String,
    pub caption: String,
    pub width: i32,
    pub height: i32,
    pub snap_x: i32,
    pub snap_y: i32,
    pub isometric: bool,
    pub speed: i32,
    pub persistent: bool,
    /// GM's `0x00BBGGRR`.
    pub background_color: u32,
    pub draw_background_color: bool,
    pub creation_code: String,
    pub backgrounds: Vec<RoomBackground>,
    pub views_enabled: bool,
    pub views: Vec<View>,
    /// In creation order.
    pub instances: Vec<Instance>,
    pub tiles: Vec<Tile>,
    pub editor: RoomEditorSettings,
}

pub(crate) fn read_rooms(r: &mut Reader<'_>) -> Result<Vec<Option<Room>>> {
    read_section(r, "room", 800, |s, name| {
        s.version("room", 541)?;
        let caption = s.string()?;
        let width = s.i32()?;
        let height = s.i32()?;
        // GM stores snap y before snap x.
        let snap_y = s.i32()?;
        let snap_x = s.i32()?;
        let isometric = s.bool()?;
        let speed = s.i32()?;
        let persistent = s.bool()?;
        let background_color = s.u32()?;
        let draw_background_color = s.bool()?;
        let creation_code = s.string()?;
        let n = s.count(40)?;
        let mut backgrounds = Vec::new();
        for _ in 0..n {
            backgrounds.push(RoomBackground {
                visible: s.bool()?,
                foreground: s.bool()?,
                background: s.i32()?,
                x: s.i32()?,
                y: s.i32()?,
                tile_h: s.bool()?,
                tile_v: s.bool()?,
                hspeed: s.i32()?,
                vspeed: s.i32()?,
                stretch: s.bool()?,
            });
        }
        let views_enabled = s.bool()?;
        let n = s.count(56)?;
        let mut views = Vec::new();
        for _ in 0..n {
            views.push(View {
                visible: s.bool()?,
                view_x: s.i32()?,
                view_y: s.i32()?,
                view_w: s.i32()?,
                view_h: s.i32()?,
                port_x: s.i32()?,
                port_y: s.i32()?,
                port_w: s.i32()?,
                port_h: s.i32()?,
                hborder: s.i32()?,
                vborder: s.i32()?,
                hspeed: s.i32()?,
                vspeed: s.i32()?,
                follow_object: s.i32()?,
            });
        }
        let n = s.count(24)?;
        let mut instances = Vec::new();
        for _ in 0..n {
            instances.push(Instance {
                x: s.i32()?,
                y: s.i32()?,
                object: s.i32()?,
                id: s.i32()?,
                creation_code: s.string()?,
                locked: s.bool()?,
            });
        }
        let n = s.count(40)?;
        let mut tiles = Vec::new();
        for _ in 0..n {
            tiles.push(Tile {
                x: s.i32()?,
                y: s.i32()?,
                background: s.i32()?,
                bg_x: s.i32()?,
                bg_y: s.i32()?,
                width: s.i32()?,
                height: s.i32()?,
                depth: s.i32()?,
                id: s.i32()?,
                locked: s.bool()?,
            });
        }
        let editor = RoomEditorSettings {
            remember_window_size: s.bool()?,
            width: s.i32()?,
            height: s.i32()?,
            show_grid: s.bool()?,
            show_objects: s.bool()?,
            show_tiles: s.bool()?,
            show_backgrounds: s.bool()?,
            show_foregrounds: s.bool()?,
            show_views: s.bool()?,
            delete_underlying_objects: s.bool()?,
            delete_underlying_tiles: s.bool()?,
            tab: s.i32()?,
            scroll_x: s.i32()?,
            scroll_y: s.i32()?,
        };
        Ok(Room {
            name,
            caption,
            width,
            height,
            snap_x,
            snap_y,
            isometric,
            speed,
            persistent,
            background_color,
            draw_background_color,
            creation_code,
            backgrounds,
            views_enabled,
            views,
            instances,
            tiles,
            editor,
        })
    })
}
