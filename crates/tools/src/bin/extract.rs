//! Extracts Spelunky Classic 1.1's assets from the reference zips into an asset bundle.
//!
//! ```sh
//! extract --reference reference/ --out assets/
//! ```
//!
//! Reads `spelunky.gmk` from `spelunky_1_1_src.zip` and the `sound/` files from `spelunky_1_1.zip`
//! directly; nothing needs unzipping first. `scripts/extract` runs it with the repo's defaults.
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

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use assets::{AtlasPage, AtlasRect, Bundle, Mask, SoundFile};
use gmk::{Action, ActionKind, ArgKind, Event, EventType, ExecType, Gmk};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const SOURCE_ZIP: &str = "spelunky_1_1_src.zip";
const GAME_ZIP: &str = "spelunky_1_1.zip";
const GMK_ENTRY: &str = "spelunky.gmk";
const SOUND_DIR: &str = "sound/";

/// Paths in the output directory that the extractor owns and rewrites on every run.
const OWNED_PATHS: [&str; 5] = [assets::BUNDLE_FILE, "atlas", "backgrounds", "sounds", "gml"];

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

const USAGE: &str = "usage: extract --reference <dir with the reference zips> --out <output dir>";

fn run(mut args: impl Iterator<Item = String>) -> Result<()> {
    let (mut reference, mut out) = (None, None);
    while let Some(arg) = args.next() {
        let slot = match arg.as_str() {
            "--reference" => &mut reference,
            "--out" => &mut out,
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

    let gmk_bytes = read_zip_entry(&reference.join(SOURCE_ZIP), GMK_ENTRY)?;
    let sounds = read_zip_dir(&reference.join(GAME_ZIP), SOUND_DIR)?;
    let gmk = gmk::parse(&gmk_bytes).map_err(|e| format!("{GMK_ENTRY}: {e}"))?;
    let files = build(&gmk, sounds)?;
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

/// Every file directly inside `dir` in the zip, by file name.
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
fn build(gmk: &Gmk, sounds: BTreeMap<String, Vec<u8>>) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();

    let (atlas_pages, frame_rects) = pack_atlas(gmk, &mut files)?;
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

    write_gml(gmk, &mut files);

    let bundle = Bundle {
        settings: gmk.settings.clone(),
        room_order: gmk.room_order.clone(),
        atlas_pages,
        sprites,
        backgrounds,
        sounds: sound_files,
        scripts: gmk.scripts.clone(),
        objects: gmk.objects.clone(),
        rooms: gmk.rooms.clone(),
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
    for (rel, data) in files {
        let path = out.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&path, data).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Atlas

/// Each sprite's frame rects, by sprite id.
type FrameRects = BTreeMap<usize, Vec<AtlasRect>>;

fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    bgra.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[b, g, r, a]| [r, g, b, a])
        .collect()
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    writer.finish()?;
    Ok(out)
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

/// Hands out file-system-safe names, unique within one directory.
#[derive(Default)]
struct UniqueNames(BTreeSet<String>);

impl UniqueNames {
    fn get(&mut self, name: &str, id: usize) -> String {
        let mut safe: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if safe.is_empty() || !self.0.insert(safe.to_ascii_lowercase()) {
            // Keep the id so a duplicate (or, on case-insensitive file systems, a case-only duplicate)
            // can't overwrite another resource's file.
            safe = format!("{safe}__{id}");
            self.0.insert(safe.to_ascii_lowercase());
        }
        safe
    }
}

fn lf(code: &str) -> String {
    let mut s = code.replace("\r\n", "\n");
    if !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

fn write_gml(gmk: &Gmk, files: &mut BTreeMap<String, Vec<u8>>) {
    let mut names = UniqueNames::default();
    for (id, script) in gmk.scripts.iter().enumerate() {
        if let Some(s) = script {
            files.insert(
                format!("gml/scripts/{}.gml", names.get(&s.name, id)),
                lf(&s.code).into_bytes(),
            );
        }
    }

    let mut names = UniqueNames::default();
    for (id, object) in gmk.objects.iter().enumerate() {
        let Some(o) = object else { continue };
        let dir = format!("gml/objects/{}", names.get(&o.name, id));
        let mut event_names = UniqueNames::default();
        for (i, event) in o.events.iter().enumerate() {
            let name = event_names.get(&event_name(gmk, event), i);
            files.insert(
                format!("{dir}/{name}.gml"),
                event_gml(gmk, &event.actions).into_bytes(),
            );
        }
    }

    let mut names = UniqueNames::default();
    for (id, room) in gmk.rooms.iter().enumerate() {
        let Some(r) = room else { continue };
        let dir = format!("gml/rooms/{}", names.get(&r.name, id));
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

fn resource_name<T>(list: &[Option<T>], id: i32, name: impl Fn(&T) -> &String) -> String {
    match gmk::get(list, id) {
        Some(r) => name(r).clone(),
        None => format!("<{id}>"),
    }
}

/// A file name for an event, e.g. `alarm_0`, `step_end`, `collision_oSolid`, `keypress_F1`.
fn event_name(gmk: &Gmk, event: &Event) -> String {
    let sub = event.sub;
    let base = event.event_type.name();
    match event.event_type {
        EventType::Create | EventType::Destroy | EventType::Draw => base.to_owned(),
        EventType::Alarm | EventType::Trigger => format!("{base}_{sub}"),
        EventType::Step => match sub {
            0 => "step".to_owned(),
            1 => "step_begin".to_owned(),
            2 => "step_end".to_owned(),
            _ => format!("step_{sub}"),
        },
        EventType::Collision => format!("{base}_{}", resource_name(&gmk.objects, sub, |o| &o.name)),
        EventType::Keyboard | EventType::KeyPress | EventType::KeyRelease => {
            format!("{base}_{}", key_name(sub))
        }
        EventType::Mouse => format!("{base}_{}", mouse_event_name(sub)),
        EventType::Other => format!("{base}_{}", other_event_name(sub)),
    }
}

fn key_name(code: i32) -> String {
    let named = match code {
        0 => "nokey",
        1 => "anykey",
        8 => "backspace",
        9 => "tab",
        13 => "enter",
        16 => "shift",
        17 => "control",
        18 => "alt",
        19 => "pause",
        27 => "escape",
        32 => "space",
        33 => "pageup",
        34 => "pagedown",
        35 => "end",
        36 => "home",
        37 => "left",
        38 => "up",
        39 => "right",
        40 => "down",
        45 => "insert",
        46 => "delete",
        _ => "",
    };
    match code {
        _ if !named.is_empty() => named.to_owned(),
        48..=57 | 65..=90 => char::from(code as u8).to_string(),
        96..=105 => format!("numpad{}", code - 96),
        112..=123 => format!("F{}", code - 111),
        _ => code.to_string(),
    }
}

fn mouse_event_name(sub: i32) -> String {
    const NAMES: [&str; 12] = [
        "left_button",
        "right_button",
        "middle_button",
        "no_button",
        "left_press",
        "right_press",
        "middle_press",
        "left_release",
        "right_release",
        "middle_release",
        "enter",
        "leave",
    ];
    const GLOBAL: [&str; 9] = [
        "global_left_button",
        "global_right_button",
        "global_middle_button",
        "global_left_press",
        "global_right_press",
        "global_middle_press",
        "global_left_release",
        "global_right_release",
        "global_middle_release",
    ];
    let name = match sub {
        0..=11 => NAMES.get(sub as usize).copied(),
        50..=58 => GLOBAL.get((sub - 50) as usize).copied(),
        60 => Some("wheel_up"),
        61 => Some("wheel_down"),
        _ => None,
    };
    name.map_or_else(|| sub.to_string(), str::to_owned)
}

fn other_event_name(sub: i32) -> String {
    let name = match sub {
        0 => "outside",
        1 => "boundary",
        2 => "game_start",
        3 => "game_end",
        4 => "room_start",
        5 => "room_end",
        6 => "no_more_lives",
        7 => "animation_end",
        8 => "end_of_path",
        9 => "no_more_health",
        10..=25 => return format!("user{}", sub - 10),
        30 => "close_button",
        _ => return sub.to_string(),
    };
    name.to_owned()
}

/// An event's action list as GML. A lone code action is written as is; otherwise every action gets a
/// `// ==== action N: ... ====` header, followed by its code if it has any.
fn event_gml(gmk: &Gmk, actions: &[Action]) -> String {
    if let [only] = actions
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
            format!("{{ {} }}", a.exec_code.replace("\r\n", " "))
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
    fn unique_names_are_safe_and_distinct() {
        let mut n = UniqueNames::default();
        assert_eq!(n.get("oPlayer1", 0), "oPlayer1");
        assert_eq!(n.get("oplayer1", 1), "oplayer1__1");
        assert_eq!(n.get("../x y", 2), "___x_y");
        assert_eq!(n.get("", 3), "__3");
    }

    #[test]
    fn event_and_key_names() {
        assert_eq!(key_name(13), "enter");
        assert_eq!(key_name(77), "M");
        assert_eq!(key_name(113), "F2");
        assert_eq!(key_name(300), "300");
        assert_eq!(mouse_event_name(12), "12");
        assert_eq!(mouse_event_name(60), "wheel_up");
        assert_eq!(other_event_name(7), "animation_end");
        assert_eq!(other_event_name(12), "user2");
    }

    #[test]
    fn rejects_unsafe_sound_names() {
        assert!(is_safe_file_name("jump.wav"));
        assert!(!is_safe_file_name(".."));
        assert!(!is_safe_file_name("a/b.wav"));
        assert!(!is_safe_file_name("a\\b.wav"));
    }

    #[test]
    fn arguments_are_required() {
        assert!(run(["--out".to_owned(), "x".to_owned()].into_iter()).is_err());
        assert!(run(["--reference".to_owned()].into_iter()).is_err());
        assert!(run(["--bogus".to_owned()].into_iter()).is_err());
    }
}
