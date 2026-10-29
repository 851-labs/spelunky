//! Extracts Spelunky Classic 1.1's assets from the reference zips into an asset bundle.
//!
//! ```sh
//! extract --reference reference/ --out assets/
//! extract --reference reference/ --out assets/config/ --program config
//! ```
//!
//! Reads `spelunky.gmk` from `spelunky_1_1_src.zip` and the `sound/` files from `spelunky_1_1.zip`
//! directly; nothing needs unzipping first. `--program config` extracts the settings program,
//! `config.gmk`, instead (its sounds are the game's). `scripts/extract` runs both with the repo's defaults.
//!
//! # Output
//!
//! The bundle layout and encoding are defined (and documented) by the `assets` crate, which is also what
//! the game uses to load it:
//!
//! - `bundle.bin`: all metadata in one [postcard](https://docs.rs/postcard) blob (magic, format version,
//!   then serde data): settings, room order, sprites (atlas rects, origin, mask settings and the collision
//!   masks GM 8.0 builds, bit-packed within each mask's box), backgrounds, sounds, scripts, objects with
//!   their full action lists, and rooms.
//! - `atlas/<n>.png`: sprite frames packed into 2048-pixel-wide RGBA pages with a transparent gutter.
//! - `backgrounds/<name>.png`: one RGBA image per background.
//! - `sounds/<file>`: the game's sound files, byte for byte.
//! - `gml/`: every script, object event, and room or instance creation code as a readable `.gml` file
//!   (LF line endings). Events made of several actions, or of drag-and-drop actions, get a header comment
//!   per action. For reading and grepping; the game loads the GML from `bundle.bin`.
//! - `rooms/<name>.json`: every room as the room editor placed it, for the wiki's source viewer
//!   (docs/wiki-media.md, "Source"): its settings, views and visible backgrounds, its creation code, its tiles,
//!   and its instances in creation order, one a line, with resources by name. The game loads the rooms from
//!   `bundle.bin`.
//!
//! Why this format: postcard is compact and decodes in milliseconds with no reflection, and it's
//! `no_std`, so the same loader works natively and on wasm. PNG keeps the pixels small and is decoded by
//! both browsers and the `png` crate. A JSON or RON metadata file would be easier to eyeball but is several
//! times larger and slower to parse; the `gml/` tree covers the reading use case.
//!
//! # Determinism
//!
//! The same input produces byte-identical output: everything is built in memory first, iterating only
//! over id-ordered lists and sorted maps; the atlas packer is a stable sort plus a shelf packer; PNG
//! settings are fixed. Before writing, the extractor removes the paths it owns (the ones above) from the
//! output directory, so stale files from an earlier run can't linger. It leaves anything else alone.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use assets::{AtlasPage, AtlasRect, Bundle, Mask, SoundFile};
use gmk::{Action, ActionKind, ArgKind, ExecType, Gmk};
use tools::gml_tree::{self, lf};
use tools::{UniqueNames, bgra_to_rgba};
use transpile::reference::Program;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const SOURCE_ZIP: &str = "spelunky_1_1_src.zip";
const GAME_ZIP: &str = "spelunky_1_1.zip";
const SOUND_DIR: &str = "sound/";

/// Paths in the output directory that the extractor owns and rewrites on every run.
const OWNED_PATHS: [&str; 6] = [
    assets::BUNDLE_FILE,
    "atlas",
    "backgrounds",
    "sounds",
    "gml",
    "rooms",
];

/// Atlas page width, and the maximum page height. WebGPU guarantees 8192; 2048 keeps pages modest.
const PAGE_SIZE: u32 = 2048;
/// Transparent pixels kept free on every side of a frame.
const GUTTER: u32 = 1;
/// Cap on a single zip entry, so a corrupt archive can't make the extractor allocate without bound.
const MAX_ZIP_ENTRY: u64 = 256 << 20;

fn main() -> ExitCode {
    match run(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("extract: {e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "usage: extract --reference <dir with the reference zips> --out <output dir> [--program game|config]";

fn run(mut args: impl Iterator<Item = String>) -> Result<()> {
    let (mut reference, mut out, mut program) = (None, None, None);
    while let Some(arg) = args.next() {
        let slot = match arg.as_str() {
            "--reference" => &mut reference,
            "--out" => &mut out,
            "--program" => &mut program,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            _ => return Err(format!("unexpected argument {arg:?}\n{USAGE}").into()),
        };
        *slot = Some(PathBuf::from(
            args.next()
                .ok_or_else(|| format!("{arg} needs a value\n{USAGE}"))?,
        ));
    }
    let (Some(reference), Some(out)) = (reference, out) else {
        return Err(USAGE.into());
    };

    let program = match program.as_deref().map(|p| p.to_str().unwrap_or_default()) {
        None => Program::Game,
        Some(name) => Program::from_name(name)
            .ok_or_else(|| format!("no program {name:?}: `game` or `config`\n{USAGE}"))?,
    };
    let entry = program.member();
    let gmk_bytes = read_zip_entry(&reference.join(SOURCE_ZIP), entry)?;
    // The settings program loads its two sounds from the same game directory's `sound/`, so they stay in the
    // game's bundle: a frontend gives it the game's sound bank.
    let sounds = match program {
        Program::Game => read_zip_dir(&reference.join(GAME_ZIP), SOUND_DIR)?,
        Program::Config => BTreeMap::new(),
    };
    let gmk = gmk::parse(&gmk_bytes).map_err(|e| format!("{entry}: {e}"))?;
    let files = build(gmk, sounds)?;
    write_bundle(&out, &files)?;
    let bytes: usize = files.values().map(Vec::len).sum();
    println!(
        "extract: wrote {} files ({} KiB) to {}",
        files.len(),
        bytes / 1024,
        out.display()
    );
    Ok(())
}

fn open_zip(path: &Path) -> Result<zip::ZipArchive<fs::File>> {
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(zip::ZipArchive::new(file).map_err(|e| format!("{}: {e}", path.display()))?)
}

fn read_entry(entry: impl Read, name: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    entry
        .take(MAX_ZIP_ENTRY + 1)
        .read_to_end(&mut out)
        .map_err(|e| format!("{name}: {e}"))?;
    if out.len() as u64 > MAX_ZIP_ENTRY {
        return Err(format!("{name}: larger than {MAX_ZIP_ENTRY} bytes").into());
    }
    Ok(out)
}

fn read_zip_entry(zip_path: &Path, name: &str) -> Result<Vec<u8>> {
    let mut zip = open_zip(zip_path)?;
    let entry = zip
        .by_name(name)
        .map_err(|e| format!("{}: {name}: {e}", zip_path.display()))?;
    read_entry(entry, name)
}

/// Every file directly inside `dir` in the zip, by file name. Anything else under `dir` (a subdirectory,
/// or a name that isn't plain ASCII) is an error rather than skipped, so a different archive than the one
/// this was written for can't silently lose files.
fn read_zip_dir(zip_path: &Path, dir: &str) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut zip = open_zip(zip_path)?;
    let mut out = BTreeMap::new();
    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        let full = entry.name()?.into_owned();
        let Some(name) = full.strip_prefix(dir) else {
            continue;
        };
        if entry.is_dir() || name.is_empty() {
            continue;
        }
        if !is_safe_file_name(name) {
            return Err(format!("{}: unexpected entry {full:?}", zip_path.display()).into());
        }
        let name = name.to_owned();
        let data = read_entry(entry, &name)?;
        out.insert(name, data);
    }
    if out.is_empty() {
        return Err(format!("{}: no files under {dir}", zip_path.display()).into());
    }
    Ok(out)
}

/// A plain file name: no directories, no traversal, nothing a filesystem would treat specially.
fn is_safe_file_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' '))
}

/// Builds every output file in memory, keyed by path relative to the output directory.
fn build(gmk: Gmk, sounds: BTreeMap<String, Vec<u8>>) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();

    let (atlas_pages, frame_rects) = pack_atlas(&gmk, &mut files)?;
    let mut sprites = Vec::new();
    for (id, sprite) in gmk.sprites.iter().enumerate() {
        let Some(s) = sprite else {
            sprites.push(None);
            continue;
        };
        let masks = s
            .collision_masks()
            .map_err(|e| format!("sprite {}: {e}", s.name))?;
        sprites.push(Some(assets::Sprite {
            name: s.name.clone(),
            width: s.width(),
            height: s.height(),
            origin_x: s.origin_x,
            origin_y: s.origin_y,
            frames: frame_rects[&id].clone(),
            shape: s.shape,
            bbox_mode: s.bbox_mode,
            alpha_tolerance: s.alpha_tolerance,
            separate_masks: s.separate_masks,
            masks: masks.iter().map(Mask::from_collision_mask).collect(),
        }));
    }

    let mut backgrounds = Vec::new();
    let mut names = UniqueNames::default();
    for (id, background) in gmk.backgrounds.iter().enumerate() {
        let Some(b) = background else {
            backgrounds.push(None);
            continue;
        };
        let image = &b.image;
        let file = if image.width > 0 && image.height > 0 {
            let path = format!("backgrounds/{}.png", names.get(&b.name, id));
            files.insert(
                path.clone(),
                encode_png(image.width, image.height, &bgra_to_rgba(&image.bgra))?,
            );
            Some(path)
        } else {
            None
        };
        backgrounds.push(Some(assets::Background {
            name: b.name.clone(),
            file,
            width: image.width,
            height: image.height,
            use_as_tileset: b.use_as_tileset,
            tile_width: b.tile_width,
            tile_height: b.tile_height,
            h_offset: b.h_offset,
            v_offset: b.v_offset,
            h_sep: b.h_sep,
            v_sep: b.v_sep,
        }));
    }

    let mut sound_files = Vec::new();
    for (name, data) in sounds {
        let file = format!("sounds/{name}");
        sound_files.push(SoundFile {
            name,
            file: file.clone(),
        });
        files.insert(file, data);
    }

    let rooms = room_names(&gmk);
    write_gml(&gmk, &rooms, &mut files);
    write_rooms(&gmk, &rooms, &mut files);

    let bundle = Bundle {
        settings: gmk.settings,
        room_order: gmk.room_order,
        atlas_pages,
        sprites,
        backgrounds,
        sounds: sound_files,
        scripts: gmk.scripts,
        objects: gmk.objects,
        rooms: gmk.rooms,
        last_tile_id: gmk.last_tile_id,
    };
    files.insert(assets::BUNDLE_FILE.to_owned(), bundle.to_bytes());
    Ok(files)
}

fn write_bundle(out: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    for owned in OWNED_PATHS {
        let path = out.join(owned);
        let result = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        match result {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(format!("{}: {e}", path.display()).into());
            }
            _ => {}
        }
    }
    let write = |path: &Path, data: &[u8]| -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(path, data).map_err(|e| format!("{}: {e}", path.display()).into())
    };
    // bundle.bin goes last, by rename, so a run that fails part-way leaves no bundle.bin and the game
    // reports a missing bundle rather than loading one whose files aren't all there.
    for (rel, data) in files.iter().filter(|(rel, _)| *rel != assets::BUNDLE_FILE) {
        write(&out.join(rel), data)?;
    }
    let bundle = files
        .get(assets::BUNDLE_FILE)
        .ok_or("no bundle.bin was built")?;
    let tmp = out.join(format!("{}.tmp", assets::BUNDLE_FILE));
    write(&tmp, bundle)?;
    let path = out.join(assets::BUNDLE_FILE);
    fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Atlas

/// Each sprite's frame rects, by sprite id.
type FrameRects = BTreeMap<usize, Vec<AtlasRect>>;

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>> {
    Ok(tools::encode_png(width, height, rgba)?)
}

/// Packs every sprite frame into atlas pages, adding the page PNGs to `files`. Returns the pages and,
/// per sprite id, its frames' rects.
///
/// Shelf packing, tallest frames first. Ties break on width and then on (sprite id, frame), so the
/// layout only depends on the input.
fn pack_atlas(
    gmk: &Gmk,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(Vec<AtlasPage>, FrameRects)> {
    let mut items = Vec::new();
    let mut rects = BTreeMap::new();
    for (id, sprite) in gmk.sprites.iter().enumerate() {
        let Some(s) = sprite else { continue };
        let empty = AtlasRect {
            page: 0,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
        rects.insert(id, vec![empty; s.frames.len()]);
        for (f, frame) in s.frames.iter().enumerate() {
            if frame.width == 0 || frame.height == 0 {
                continue;
            }
            if frame.width + 2 * GUTTER > PAGE_SIZE || frame.height + 2 * GUTTER > PAGE_SIZE {
                return Err(
                    format!("sprite {} frame {f} is too big for an atlas page", s.name).into(),
                );
            }
            items.push((id, f, frame));
        }
    }
    items.sort_by_key(|&(id, f, frame)| {
        (
            std::cmp::Reverse(frame.height),
            std::cmp::Reverse(frame.width),
            id,
            f,
        )
    });

    // Each page's pixel rows grow as shelves are added.
    let mut pages: Vec<Vec<u8>> = Vec::new();
    let (mut x, mut shelf_y, mut shelf_h) = (0, 0, 0);
    for (id, f, frame) in items {
        let (w, h) = (frame.width + 2 * GUTTER, frame.height + 2 * GUTTER);
        if x + w > PAGE_SIZE {
            (x, shelf_y, shelf_h) = (0, shelf_y + shelf_h, 0);
        }
        if pages.is_empty() || shelf_y + h > PAGE_SIZE {
            pages.push(Vec::new());
            (x, shelf_y, shelf_h) = (0, 0, 0);
        }
        let page = pages.len() - 1;
        let pixels = &mut pages[page];
        let needed = (shelf_y + h) as usize * PAGE_SIZE as usize * 4;
        if pixels.len() < needed {
            pixels.resize(needed, 0);
        }
        let (fx, fy) = (x + GUTTER, shelf_y + GUTTER);
        let row = frame.width as usize * 4;
        let rgba = bgra_to_rgba(&frame.bgra);
        for (r, src) in rgba.chunks_exact(row).enumerate() {
            let start = ((fy as usize + r) * PAGE_SIZE as usize + fx as usize) * 4;
            pixels[start..start + row].copy_from_slice(src);
        }
        rects.get_mut(&id).expect("sprite registered above")[f] = AtlasRect {
            page: page as u32,
            x: fx,
            y: fy,
            width: frame.width,
            height: frame.height,
        };
        x += w;
        shelf_h = shelf_h.max(h);
    }

    let mut out = Vec::new();
    for (i, pixels) in pages.iter().enumerate() {
        let height = (pixels.len() / (PAGE_SIZE as usize * 4)) as u32;
        let file = format!("atlas/{i}.png");
        files.insert(file.clone(), encode_png(PAGE_SIZE, height, pixels)?);
        out.push(AtlasPage {
            file,
            width: PAGE_SIZE,
            height,
        });
    }
    Ok((out, rects))
}

// ---------------------------------------------------------------------------------------------------------
// GML tree

/// Each room's file name, by id: its `gml/rooms/<name>/` directory and its `rooms/<name>.json`, which the
/// source viewer's page for it is named by.
fn room_names(gmk: &Gmk) -> Vec<Option<String>> {
    let mut names = UniqueNames::default();
    (gmk.rooms.iter().enumerate())
        .map(|(id, room)| room.as_ref().map(|r| names.get(&r.name, id)))
        .collect()
}

fn write_gml(gmk: &Gmk, room_names: &[Option<String>], files: &mut BTreeMap<String, Vec<u8>>) {
    for (script, file) in gmk.scripts.iter().zip(gml_tree::script_files(&gmk.scripts)) {
        if let (Some(s), Some(file)) = (script, file) {
            files.insert(file, lf(&s.code).into_bytes());
        }
    }

    for (object, event_files) in gmk.objects.iter().zip(gml_tree::event_files(&gmk.objects)) {
        let (Some(o), Some(event_files)) = (object, event_files) else {
            continue;
        };
        for (event, file) in o.events.iter().zip(event_files) {
            files.insert(file, event_gml(gmk, &event.actions).into_bytes());
        }
    }

    for (room, name) in gmk.rooms.iter().zip(room_names) {
        let (Some(r), Some(name)) = (room, name) else {
            continue;
        };
        let dir = format!("gml/rooms/{name}");
        if !r.creation_code.is_empty() {
            files.insert(
                format!("{dir}/creation.gml"),
                lf(&r.creation_code).into_bytes(),
            );
        }
        for inst in r.instances.iter().filter(|i| !i.creation_code.is_empty()) {
            let object = resource_name(&gmk.objects, inst.object, |o| &o.name);
            let header = format!(
                "// instance {} of {object} at ({}, {})\n",
                inst.id, inst.x, inst.y
            );
            let body = header + &lf(&inst.creation_code);
            files.insert(format!("{dir}/instance_{}.gml", inst.id), body.into_bytes());
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Rooms

/// A room's file, `rooms/<name>.json`: what the room editor shows of it, resources by name.
#[derive(serde::Serialize)]
struct RoomFile<'a> {
    name: &'a str,
    caption: &'a str,
    width: i32,
    height: i32,
    speed: i32,
    persistent: bool,
    /// `#rrggbb`, from GM's `0x00BBGGRR`.
    background_color: String,
    draw_background_color: bool,
    creation_code: String,
    /// The background slots that show one, or are visible without one, by slot.
    backgrounds: Vec<RoomBackgroundFile>,
    views_enabled: bool,
    /// The visible views, by slot.
    views: Vec<ViewFile>,
}

#[derive(serde::Serialize)]
struct RoomBackgroundFile {
    slot: usize,
    visible: bool,
    foreground: bool,
    background: Option<String>,
    x: i32,
    y: i32,
    tile_h: bool,
    tile_v: bool,
    hspeed: i32,
    vspeed: i32,
    stretch: bool,
}

#[derive(serde::Serialize)]
struct ViewFile {
    slot: usize,
    view_x: i32,
    view_y: i32,
    view_w: i32,
    view_h: i32,
    port_x: i32,
    port_y: i32,
    port_w: i32,
    port_h: i32,
    hborder: i32,
    vborder: i32,
    hspeed: i32,
    vspeed: i32,
    follow: Option<String>,
}

#[derive(serde::Serialize)]
struct TileFile {
    id: i32,
    background: String,
    x: i32,
    y: i32,
    bg_x: i32,
    bg_y: i32,
    width: i32,
    height: i32,
    depth: i32,
}

#[derive(serde::Serialize)]
struct InstanceFile {
    id: i32,
    object: String,
    x: i32,
    y: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    creation_code: String,
}

fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("a room serializes")
}

/// Every room as `rooms/<name>.json` (room_json).
fn write_rooms(gmk: &Gmk, room_names: &[Option<String>], files: &mut BTreeMap<String, Vec<u8>>) {
    let object = |id| resource_name(&gmk.objects, id, |o| &o.name);
    let background = |id| resource_name(&gmk.backgrounds, id, |b| &b.name);
    for (room, name) in gmk.rooms.iter().zip(room_names) {
        let (Some(r), Some(name)) = (room, name) else {
            continue;
        };
        files.insert(
            format!("rooms/{name}.json"),
            room_json(r, &object, &background).into_bytes(),
        );
    }
}

/// A room's file, its objects and backgrounds named by `object` and `background`: the settings
/// pretty-printed, then the tiles and the instances one a line, so a room's file stays a few hundred lines
/// that a change to one instance changes one of.
fn room_json(
    r: &gmk::Room,
    object: &dyn Fn(i32) -> String,
    background: &dyn Fn(i32) -> String,
) -> String {
    let [red, green, blue, _] = r.background_color.to_le_bytes();
    let settings = RoomFile {
        name: &r.name,
        caption: &r.caption,
        width: r.width,
        height: r.height,
        speed: r.speed,
        persistent: r.persistent,
        background_color: format!("#{red:02x}{green:02x}{blue:02x}"),
        draw_background_color: r.draw_background_color,
        creation_code: lf(&r.creation_code),
        backgrounds: (r.backgrounds.iter().enumerate())
            .filter(|(_, b)| b.visible || b.background >= 0)
            .map(|(slot, b)| RoomBackgroundFile {
                slot,
                visible: b.visible,
                foreground: b.foreground,
                background: (b.background >= 0).then(|| background(b.background)),
                x: b.x,
                y: b.y,
                tile_h: b.tile_h,
                tile_v: b.tile_v,
                hspeed: b.hspeed,
                vspeed: b.vspeed,
                stretch: b.stretch,
            })
            .collect(),
        views_enabled: r.views_enabled,
        views: (r.views.iter().enumerate())
            .filter(|(_, v)| v.visible)
            .map(|(slot, v)| ViewFile {
                slot,
                view_x: v.view_x,
                view_y: v.view_y,
                view_w: v.view_w,
                view_h: v.view_h,
                port_x: v.port_x,
                port_y: v.port_y,
                port_w: v.port_w,
                port_h: v.port_h,
                hborder: v.hborder,
                vborder: v.vborder,
                hspeed: v.hspeed,
                vspeed: v.vspeed,
                follow: (v.follow_object >= 0).then(|| object(v.follow_object)),
            })
            .collect(),
    };
    let tiles = r.tiles.iter().map(|t| TileFile {
        id: t.id,
        background: background(t.background),
        x: t.x,
        y: t.y,
        bg_x: t.bg_x,
        bg_y: t.bg_y,
        width: t.width,
        height: t.height,
        depth: t.depth,
    });
    let instances = r.instances.iter().map(|i| InstanceFile {
        id: i.id,
        object: object(i.object),
        x: i.x,
        y: i.y,
        creation_code: lf(&i.creation_code),
    });
    let mut out = serde_json::to_string_pretty(&settings).expect("a room serializes");
    out.truncate(out.len() - "\n}".len());
    for (key, items) in [
        ("tiles", tiles.map(|t| json(&t)).collect::<Vec<_>>()),
        ("instances", instances.map(|i| json(&i)).collect()),
    ] {
        let _ = write!(out, ",\n  \"{key}\": [");
        for (n, item) in items.iter().enumerate() {
            let _ = write!(out, "{}\n    {item}", if n == 0 { "" } else { "," });
        }
        out += if items.is_empty() { "]" } else { "\n  ]" };
    }
    out += "\n}\n";
    out
}

fn resource_name<T>(list: &[Option<T>], id: i32, name: impl Fn(&T) -> &String) -> String {
    match gmk::get(list, id) {
        Some(r) => name(r).clone(),
        None => format!("<{id}>"),
    }
}

/// An event's action list as GML. A lone code action is written as is; otherwise every action gets a
/// `// ==== action N: ... ====` header, followed by its code if it has any.
fn event_gml(gmk: &Gmk, actions: &[Action]) -> String {
    // Only a plain code action that runs on self goes without its header.
    if !gml_tree::has_headers(actions)
        && let [only] = actions
        && let Some(code) = only.code()
    {
        return lf(code);
    }
    let mut out = String::new();
    for (i, action) in actions.iter().enumerate() {
        let _ = writeln!(
            out,
            "// ==== action {i}: {} ====",
            describe_action(gmk, action)
        );
        if let Some(code) = action.code() {
            out += &lf(code);
        }
    }
    out
}

fn describe_action(gmk: &Gmk, a: &Action) -> String {
    let args: Vec<String> = a
        .args
        .iter()
        .map(|arg| arg_text(gmk, arg.kind, &arg.value))
        .collect();
    let mut s = match (a.kind, a.exec_type) {
        (ActionKind::Code, _) => "code".to_owned(),
        (ActionKind::Variable, _) if args.len() >= 2 => {
            format!(
                "set {} {} {}",
                args[0],
                if a.relative { "+=" } else { "=" },
                args[1]
            )
        }
        (ActionKind::Normal, ExecType::Function) => format!("{}({})", a.function, args.join(", ")),
        (ActionKind::Normal, ExecType::Code) => {
            format!(
                "{{ {} }}",
                a.exec_code.replace("\r\n", " ").replace(['\r', '\n'], " ")
            )
        }
        // Library 1's comment action (id 605) runs nothing; its argument is the comment.
        (ActionKind::Normal, ExecType::None) => format!("comment: {}", args.join(", ")),
        (kind, _) => format!(
            "{kind:?} lib {} id {} [{}]",
            a.library_id,
            a.action_id,
            args.join(", ")
        ),
    };
    if a.invert {
        s.insert_str(0, "NOT ");
    }
    if a.has_applies_to && a.applies_to != -1 {
        let target = match a.applies_to {
            -2 => "other".to_owned(),
            id => resource_name(&gmk.objects, id, |o| &o.name),
        };
        let _ = write!(s, " [applies to {target}]");
    }
    if a.relative && a.kind != ActionKind::Variable {
        s += " [relative]";
    }
    format!("lib {} id {}: {s}", a.library_id, a.action_id)
}

/// An argument's value, with resource ids replaced by names.
fn arg_text(gmk: &Gmk, kind: ArgKind, value: &str) -> String {
    let id = value.trim().parse::<i32>().ok();
    let name = id.and_then(|id| match kind {
        ArgKind::Sprite => gmk::get(&gmk.sprites, id).map(|r| r.name.clone()),
        ArgKind::Sound => gmk::get(&gmk.sounds, id).map(|r| r.name.clone()),
        ArgKind::Background => gmk::get(&gmk.backgrounds, id).map(|r| r.name.clone()),
        ArgKind::Path => gmk::get(&gmk.paths, id).map(|r| r.name.clone()),
        ArgKind::Script => gmk::get(&gmk.scripts, id).map(|r| r.name.clone()),
        ArgKind::Object => gmk::get(&gmk.objects, id).map(|r| r.name.clone()),
        ArgKind::Room => gmk::get(&gmk.rooms, id).map(|r| r.name.clone()),
        ArgKind::Font => gmk::get(&gmk.fonts, id).map(|r| r.name.clone()),
        ArgKind::Timeline => gmk::get(&gmk.timelines, id).map(|r| r.name.clone()),
        _ => None,
    });
    let text = name.unwrap_or_else(|| value.to_owned());
    // Keep the header on one line.
    text.replace("\r\n", "\\n").replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_sound_names() {
        assert!(is_safe_file_name("jump.wav"));
        assert!(!is_safe_file_name(".."));
        assert!(!is_safe_file_name("a/b.wav"));
        assert!(!is_safe_file_name("a\\b.wav"));
    }

    #[test]
    fn rooms_list_their_instances_one_a_line() {
        let room = gmk::Room {
            name: "rTest".to_owned(),
            width: 320,
            height: 240,
            speed: 30,
            // GM's 0x00BBGGRR.
            background_color: 0x0030_2010,
            creation_code: "a = 1;\r\n".to_owned(),
            // Eight empty slots of each, as GM saves them.
            backgrounds: vec![
                gmk::RoomBackground {
                    background: -1,
                    ..Default::default()
                };
                8
            ],
            views: vec![
                gmk::View {
                    follow_object: -1,
                    ..Default::default()
                };
                8
            ],
            instances: vec![
                gmk::Instance {
                    x: 16,
                    y: 32,
                    object: 0,
                    id: 100001,
                    ..Default::default()
                },
                gmk::Instance {
                    x: 48,
                    y: 32,
                    object: 9,
                    id: 100002,
                    creation_code: "hp = 2;".to_owned(),
                    locked: true,
                },
            ],
            ..Default::default()
        };
        let object = |id| {
            ["oBrick"]
                .get(id as usize)
                .map_or(format!("<{id}>"), |n| (*n).to_owned())
        };
        let json = room_json(&room, &object, &|id| format!("<{id}>"));
        assert_eq!(
            json,
            r##"{
  "name": "rTest",
  "caption": "",
  "width": 320,
  "height": 240,
  "speed": 30,
  "persistent": false,
  "background_color": "#102030",
  "draw_background_color": false,
  "creation_code": "a = 1;\n",
  "backgrounds": [],
  "views_enabled": false,
  "views": [],
  "tiles": [],
  "instances": [
    {"id":100001,"object":"oBrick","x":16,"y":32},
    {"id":100002,"object":"<9>","x":48,"y":32,"creation_code":"hp = 2;\n"}
  ]
}
"##
        );
        serde_json::from_str::<serde_json::Value>(&json).expect("the file is JSON");
    }

    #[test]
    fn arguments_are_required() {
        assert!(run(["--out".to_owned(), "x".to_owned()].into_iter()).is_err());
        assert!(run(["--reference".to_owned()].into_iter()).is_err());
        assert!(run(["--bogus".to_owned()].into_iter()).is_err());
    }
}
