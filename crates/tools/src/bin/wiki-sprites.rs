//! Exports every sprite in an extracted asset bundle as the wiki's media: frames, strips and animated
//! previews, with JSON metadata and an object-to-sprites index.
//!
//! ```sh
//! wiki-sprites --assets assets/ --out target/wiki-media/sprites [--scale 4]
//! ```
//!
//! `scripts/wiki-sprites` runs it with those defaults. The input is what `scripts/extract` writes:
//! `bundle.bin` for the sprites, objects and rooms, and the atlas pages for the pixels. The output layout
//! and the metadata are described in `docs/wiki-media.md`.
//!
//! # Animation speed
//!
//! GM advances an instance's `image_index` by `image_speed` every step, and a room runs `room_speed`
//! steps a second, so a sprite plays at `image_speed * room_speed` frames a second. Most speeds are set
//! by code at run time, often depending on state, so only the plainest case counts as known: an object
//! whose own sprite this is sets `image_speed` to a positive number at the top level of its create event
//! (or inherits one from its parent's). Failing that, an object that uses the sprite and sets a speed
//! that way is a weaker hint. Anything else plays at [`DEFAULT_IMAGE_SPEED`], and the metadata says
//! which it was.
//!
//! The previews are APNG rather than animated WebP: the `png` crate the workspace already uses writes
//! APNG, every current browser plays it, and it keeps full alpha losslessly. Animated WebP would need
//! libwebp (a C library) or a hand-written WebP muxer, for files a few hundred bytes smaller.
//!
//! # Determinism
//!
//! The same bundle gives byte-identical output: everything is built in memory first, from id-ordered
//! lists and sorted maps, with fixed PNG settings. The output directory is replaced as a whole, through
//! a sibling `.partial` directory, so a failed run leaves the previous output intact.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use assets::{AtlasRect, Bbox, Bundle, EventType, Object, Sprite};
use gml::ast::{AssignOp, ExprKind, StmtKind};
use gml::visit::{self, Visitor};
use serde::Serialize;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const USAGE: &str =
    "usage: wiki-sprites --assets <extracted bundle dir> --out <output dir> [--scale <1-16>]";

/// The upscaled images' factor unless `--scale` says otherwise: 16-pixel sprites become 64 pixels,
/// legible in an infobox without the page having to scale them.
const DEFAULT_SCALE: u32 = 4;
const MAX_SCALE: u32 = 16;

/// The `image_speed` a preview plays at when the game's isn't known: 0.5, the value the game's code
/// sets most often (15 frames a second at Spelunky's 30 steps a second).
const DEFAULT_IMAGE_SPEED: f64 = 0.5;

/// Bumped when the metadata's shape changes, so the site can tell.
const FORMAT: u32 = 1;

/// The copyright notice the license asks for (`docs/wiki-license.md`), in the index and in every PNG.
const NOTICE: &str = "Spelunky is \u{a9} 2008, 2009 Derek Yu and Mossmouth, LLC, and is used under the \
Spelunky User License v1.1b (https://spelunky.ai/LICENSE.txt). Unofficial; not affiliated with or \
endorsed by Mossmouth.";
const LICENSE_URL: &str = "https://spelunky.ai/LICENSE.txt";

const INDEX_FILE: &str = "index.json";

fn main() -> ExitCode {
    match run(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("wiki-sprites: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(mut args: impl Iterator<Item = String>) -> Result<()> {
    let (mut assets_dir, mut out, mut scale) = (None, None, DEFAULT_SCALE);
    while let Some(arg) = args.next() {
        if arg == "-h" || arg == "--help" {
            println!("{USAGE}");
            return Ok(());
        }
        let value = match arg.as_str() {
            "--assets" | "--out" | "--scale" => args
                .next()
                .ok_or_else(|| format!("{arg} needs a value\n{USAGE}"))?,
            _ => return Err(format!("unexpected argument {arg:?}\n{USAGE}").into()),
        };
        match arg.as_str() {
            "--assets" => assets_dir = Some(PathBuf::from(value)),
            "--out" => out = Some(PathBuf::from(value)),
            _ => {
                scale = value
                    .parse()
                    .ok()
                    .filter(|s| (1..=MAX_SCALE).contains(s))
                    .ok_or_else(|| format!("bad --scale {value:?}: 1 to {MAX_SCALE}"))?;
            }
        }
    }
    let (Some(assets_dir), Some(out)) = (assets_dir, out) else {
        return Err(USAGE.into());
    };

    let bundle_path = assets_dir.join(assets::BUNDLE_FILE);
    let bytes = fs::read(&bundle_path)
        .map_err(|e| format!("{}: {e} (run scripts/extract first)", bundle_path.display()))?;
    let bundle =
        Bundle::from_bytes(&bytes).map_err(|e| format!("{}: {e}", bundle_path.display()))?;
    let mut pages = Vec::new();
    for page in &bundle.atlas_pages {
        let path = assets_dir.join(&page.file);
        let png = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        pages.push(tools::decode_png(&png).map_err(|e| format!("{}: {e}", path.display()))?);
    }

    let files = build(&bundle, &pages, scale)?;
    write_tree(&out, &files)?;
    let bytes: usize = files.values().map(Vec::len).sum();
    println!(
        "wiki-sprites: wrote {} files ({} KiB) to {}",
        files.len(),
        bytes / 1024,
        out.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Metadata

#[derive(Serialize)]
struct Index {
    format: u32,
    notice: &'static str,
    license: &'static str,
    /// The upscaled images' factor (their file names end in `@<scale>x`).
    scale: u32,
    /// Steps a second, which turns an `image_speed` into frames a second.
    room_speed: u32,
    /// The frame rate of previews whose speed isn't known.
    default_fps: f64,
    sprites: Vec<IndexSprite>,
    /// Objects that use at least one sprite, by object id.
    objects: Vec<IndexObject>,
}

#[derive(Serialize)]
struct IndexSprite {
    id: usize,
    name: String,
    /// The sprite's `sprite.json`, relative to the output directory.
    meta: String,
    frames: usize,
    width: u32,
    height: u32,
    fps: f64,
}

#[derive(Serialize)]
struct IndexObject {
    id: usize,
    name: String,
    parent: Option<String>,
    /// The object's own sprite (`sprite_index` when an instance is created).
    sprite: Option<String>,
    /// Its collision mask's sprite, if it isn't `sprite`.
    mask: Option<String>,
    /// Every sprite it uses: its own, its mask, and every sprite its events name in code or in an
    /// action's argument, in sprite id order. Sprites used only through a script it calls (the player's,
    /// set by `characterSprite`) aren't followed.
    sprites: Vec<String>,
}

#[derive(Serialize)]
struct SpriteMeta {
    id: usize,
    name: String,
    width: u32,
    height: u32,
    origin: Point,
    /// The union of the frames' collision boxes, inclusive; `None` if every frame's mask is empty.
    bbox: Option<Box4>,
    fps: f64,
    /// Where `fps` came from: the object whose create event sets the speed, or `None` for the default.
    fps_source: Option<FpsSource>,
    /// Paths relative to the output directory. `anim` is only there for sprites with several frames.
    strip: Option<String>,
    strip_scaled: Option<String>,
    anim: Option<String>,
    anim_scaled: Option<String>,
    frames: Vec<FrameMeta>,
    /// Objects whose own sprite this is.
    objects: Vec<String>,
    /// Objects that use it in any of the ways `IndexObject::sprites` lists.
    used_by: Vec<String>,
}

#[derive(Serialize)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Serialize)]
struct Box4 {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

#[derive(Serialize)]
struct FpsSource {
    object: String,
    /// `"sprite"` if this is the object's own sprite, `"uses"` if the object only uses it, a weaker hint
    /// (its speed may change with the sprite).
    via: &'static str,
    /// The value GM8 runs with, unrounded: its parser reads `0.6` as 0.6000000000000001
    /// (`gml::lexer::decimal_value`).
    image_speed: f64,
}

#[derive(Serialize)]
struct FrameMeta {
    file: String,
    file_scaled: String,
    /// This frame's collision box (the shared one unless the sprite has separate masks).
    bbox: Option<Box4>,
}

fn bbox(b: Bbox) -> Option<Box4> {
    (!b.is_empty()).then_some(Box4 {
        left: b.left,
        top: b.top,
        right: b.right,
        bottom: b.bottom,
    })
}

// ---------------------------------------------------------------------------------------------------------
// GML analysis

/// What the objects' events say about sprites.
struct Usage {
    /// Per object id, every sprite id it uses (see `IndexObject::sprites`).
    sprites: Vec<BTreeSet<usize>>,
    /// Per object id, the `image_speed` its create event leaves it with, if that's a plain number.
    create_speed: Vec<Option<f64>>,
}

fn analyse(bundle: &Bundle) -> Result<Usage> {
    let sprite_ids: BTreeMap<&str, usize> = bundle
        .sprites
        .iter()
        .enumerate()
        .filter_map(|(id, s)| Some((s.as_ref()?.name.as_str(), id)))
        .collect();
    let mut sprites = Vec::new();
    let mut own_speed = Vec::new();
    for object in &bundle.objects {
        let mut used = BTreeSet::new();
        let mut speed = OwnSpeed::Unset;
        if let Some(o) = object {
            for id in [o.sprite, o.mask] {
                if gmk::get(&bundle.sprites, id).is_some() {
                    used.insert(id as usize);
                }
            }
            for event in &o.events {
                let create = event.event_type == EventType::Create;
                if create {
                    speed = OwnSpeed::Inherits(false);
                }
                for action in &event.actions {
                    for arg in &action.args {
                        if arg.kind == assets::ArgKind::Sprite
                            && let Ok(id) = arg.value.trim().parse::<i32>()
                            && gmk::get(&bundle.sprites, id).is_some()
                        {
                            used.insert(id as usize);
                        }
                    }
                    if create && action.function == "action_inherited" {
                        speed.inherit();
                    }
                    let Some(code) = action.code() else { continue };
                    let program = gml::parse_program(code).map_err(|e| {
                        format!(
                            "{}: {:?} event: {}",
                            o.name,
                            event.event_type,
                            e.display(code)
                        )
                    })?;
                    let mut names = SpriteNames {
                        ids: &sprite_ids,
                        found: &mut used,
                    };
                    visit::walk_program(&mut names, &program);
                    // Code that runs on other instances doesn't set this one's speed.
                    if create && (!action.has_applies_to || action.applies_to == -1) {
                        speed.scan(&program);
                    }
                }
            }
        }
        sprites.push(used);
        own_speed.push(speed);
    }

    // Resolve inheritance: an object without a create event runs its parent's, as does one whose create
    // event calls `event_inherited` before setting nothing itself.
    let mut create_speed = Vec::with_capacity(own_speed.len());
    for id in 0..own_speed.len() {
        let mut current = Some(id);
        let mut speed = None;
        // The parent chain is finite in a valid game, but a cycle mustn't hang the tool.
        for _ in 0..=own_speed.len() {
            let Some(c) = current else { break };
            match own_speed[c] {
                OwnSpeed::Set(s) => {
                    speed = Some(s);
                    break;
                }
                OwnSpeed::Inherits(false) => break,
                OwnSpeed::Unset | OwnSpeed::Inherits(true) => {
                    current = bundle.objects[c]
                        .as_ref()
                        .and_then(|o| usize::try_from(o.parent).ok())
                        .filter(|&p| gmk::get(&bundle.objects, p as i32).is_some());
                }
            }
        }
        create_speed.push(speed);
    }
    Ok(Usage {
        sprites,
        create_speed,
    })
}

/// What an object's own create event does with `image_speed`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum OwnSpeed {
    /// No create event: GM runs the parent's.
    Unset,
    /// A create event that sets no plain speed; `true` if it runs the parent's.
    Inherits(bool),
    /// A create event whose last top-level `image_speed = <number>` sets this.
    Set(f64),
}

impl OwnSpeed {
    fn inherit(&mut self) {
        if let OwnSpeed::Inherits(i) = self {
            *i = true;
        }
    }

    /// Takes the top-level statements of one of the create event's code actions, in order. Only
    /// unconditional statements count: a speed set inside an `if` or a loop is a guess.
    fn scan(&mut self, program: &gml::Program) {
        for stmt in &program.body {
            match &stmt.kind {
                StmtKind::Assign {
                    place,
                    op: AssignOp::Set,
                    value,
                } if matches!(&place.kind, ExprKind::Ident(n) if n == "image_speed") => {
                    *self = match value.kind {
                        ExprKind::Real(v) => OwnSpeed::Set(v),
                        // Not a number we can read: unknown, and the parent's no longer applies.
                        _ => OwnSpeed::Inherits(false),
                    };
                }
                StmtKind::Call(call) if matches!(&call.kind, ExprKind::Call { name, .. } if name.name == "event_inherited") =>
                {
                    self.inherit();
                }
                _ => {}
            }
        }
    }
}

/// Collects the sprites a piece of GML names.
struct SpriteNames<'a> {
    ids: &'a BTreeMap<&'a str, usize>,
    found: &'a mut BTreeSet<usize>,
}

impl Visitor for SpriteNames<'_> {
    fn visit_expr(&mut self, expr: &gml::Expr) {
        if let ExprKind::Ident(name) = &expr.kind
            && let Some(&id) = self.ids.get(name.as_str())
        {
            self.found.insert(id);
        }
        visit::walk_expr(self, expr);
    }
}

/// The room speed most rooms use (every room in Spelunky uses 30).
fn room_speed(bundle: &Bundle) -> u32 {
    let mut counts = BTreeMap::new();
    for room in bundle.rooms.iter().flatten() {
        *counts.entry(room.speed.max(1) as u32).or_insert(0usize) += 1;
    }
    // The most common, the slowest on a tie.
    counts
        .iter()
        .max_by_key(|&(speed, n)| (*n, std::cmp::Reverse(*speed)))
        .map_or(30, |(speed, _)| *speed)
}

// ---------------------------------------------------------------------------------------------------------
// Images

/// An RGBA image.
#[derive(Clone, Debug, PartialEq)]
struct Image {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Image {
    fn new(width: u32, height: u32) -> Image {
        Image {
            width,
            height,
            rgba: vec![0; width as usize * height as usize * 4],
        }
    }

    /// Copies `src` in with its top-left corner at (x, y). It must fit.
    fn blit(&mut self, src: &Image, x: u32, y: u32) {
        let row = src.width as usize * 4;
        for (r, line) in src.rgba.chunks_exact(row.max(1)).enumerate() {
            let start = ((y as usize + r) * self.width as usize + x as usize) * 4;
            self.rgba[start..start + row].copy_from_slice(line);
        }
    }

    /// Nearest-neighbour upscaling by a whole factor: each pixel becomes a `scale`-pixel square.
    fn upscale(&self, scale: u32) -> Image {
        let s = scale as usize;
        let mut out = Image::new(self.width * scale, self.height * scale);
        let out_row = out.width as usize * 4;
        for (y, line) in self
            .rgba
            .chunks_exact((self.width as usize * 4).max(1))
            .enumerate()
        {
            let wide: Vec<u8> = line
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|px| px.iter().copied().cycle().take(4 * s))
                .collect();
            for dy in 0..s {
                let start = (y * s + dy) * out_row;
                out.rgba[start..start + out_row].copy_from_slice(&wide);
            }
        }
        out
    }
}

/// Crops a frame out of its atlas page.
fn crop(page: &(u32, u32, Vec<u8>), r: AtlasRect) -> Result<Image> {
    let (pw, ph, pixels) = page;
    if r.x + r.width > *pw || r.y + r.height > *ph {
        return Err(format!("atlas rect {r:?} is outside its {pw}x{ph} page").into());
    }
    let mut out = Image::new(r.width, r.height);
    let row = r.width as usize * 4;
    for y in 0..r.height as usize {
        let start = ((r.y as usize + y) * *pw as usize + r.x as usize) * 4;
        out.rgba[y * row..(y + 1) * row].copy_from_slice(&pixels[start..start + row]);
    }
    Ok(out)
}

/// A PNG of one image or, with several, an APNG that loops them forever at `fps`. Each carries the
/// copyright notice as a `Copyright` text chunk. Settings are fixed, so the bytes only depend on the input.
fn encode(frames: &[Image], fps: f64) -> Result<Vec<u8>> {
    let first = frames.first().ok_or("no frames to encode")?;
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, first.width, first.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    encoder.add_text_chunk("Copyright".to_owned(), NOTICE.to_owned())?;
    if frames.len() > 1 {
        encoder.set_animated(frames.len() as u32, 0)?;
        let (num, den) = frame_delay(fps);
        encoder.set_frame_delay(num, den)?;
        // Every frame replaces the whole canvas, transparent pixels included.
        encoder.set_blend_op(png::BlendOp::Source)?;
        encoder.set_dispose_op(png::DisposeOp::None)?;
    }
    let mut writer = encoder.write_header()?;
    for frame in frames {
        writer.write_image_data(&frame.rgba)?;
    }
    writer.finish()?;
    Ok(out)
}

/// One frame's display time at `fps`, as APNG's seconds fraction: exact for any rate given to a
/// thousandth of a frame a second up to 65 frames a second.
fn frame_delay(fps: f64) -> (u16, u16) {
    let millis = (fps * 1000.0).round().max(1.0) as u64;
    let (mut num, mut den) = (1000u64, millis);
    let g = gcd(num, den);
    (num, den) = (num / g, den / g);
    if den > u16::MAX as u64 {
        // Faster than any sprite plays: settle for the nearest whole rate.
        return (1, (fps.round() as u64).clamp(1, u16::MAX as u64) as u16);
    }
    (num as u16, den as u16)
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// A frame rate as the metadata gives it: rounded to a thousandth, so `0.4 * 30` is 12, not
/// 12.000000000000002.
fn round_fps(fps: f64) -> f64 {
    (fps * 1000.0).round() / 1000.0
}

// ---------------------------------------------------------------------------------------------------------
// Output

/// Builds every output file in memory, keyed by path relative to the output directory.
fn build(
    bundle: &Bundle,
    pages: &[(u32, u32, Vec<u8>)],
    scale: u32,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let usage = analyse(bundle)?;
    let room_speed = room_speed(bundle);
    let default_fps = round_fps(DEFAULT_IMAGE_SPEED * room_speed as f64);
    let name_of = |list: &[Option<Object>], id: usize| list[id].as_ref().map(|o| o.name.clone());

    // Sprite id -> objects whose own sprite it is, and objects that use it at all, in object id order.
    let mut owners: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut users: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (oid, object) in bundle.objects.iter().enumerate() {
        let Some(o) = object else { continue };
        if gmk::get(&bundle.sprites, o.sprite).is_some() {
            owners.entry(o.sprite as usize).or_default().push(oid);
        }
        for &sid in &usage.sprites[oid] {
            users.entry(sid).or_default().push(oid);
        }
    }

    let mut files = BTreeMap::new();
    let mut dirs = tools::UniqueNames::default();
    let mut index_sprites = Vec::new();
    for (id, sprite) in bundle.sprites.iter().enumerate() {
        let Some(s) = sprite else { continue };
        let dir = dirs.get(&s.name, id);
        let owners = owners.get(&id).map_or(&[][..], Vec::as_slice);
        let users = users.get(&id).map_or(&[][..], Vec::as_slice);
        // The first object (by id) whose own sprite this is and whose create event sets a speed, or
        // failing that the first that uses it and sets one: an object's sprites mostly share its speed
        // (oSnake switches to sSnakeWalkL at its create event's 0.4).
        let source_in = |objects: &[usize], via: &'static str| {
            objects.iter().find_map(|&oid| {
                Some(FpsSource {
                    image_speed: usage.create_speed[oid].filter(|&v| v > 0.0)?,
                    object: name_of(&bundle.objects, oid)?,
                    via,
                })
            })
        };
        let source = source_in(owners, "sprite").or_else(|| source_in(users, "uses"));
        let image_speed = source
            .as_ref()
            .map_or(DEFAULT_IMAGE_SPEED, |s| s.image_speed);
        let fps = round_fps(image_speed * room_speed as f64);
        let names = |ids: &[usize]| -> Vec<String> {
            ids.iter()
                .filter_map(|&oid| name_of(&bundle.objects, oid))
                .collect()
        };
        let meta = export_sprite(
            &mut files,
            pages,
            s,
            SpriteInfo {
                id,
                dir: &dir,
                scale,
                fps,
                fps_source: source,
                objects: names(owners),
                used_by: names(users),
            },
        )
        .map_err(|e| format!("sprite {}: {e}", s.name))?;
        index_sprites.push(IndexSprite {
            id,
            name: s.name.clone(),
            meta: format!("{dir}/sprite.json"),
            frames: meta.frames.len(),
            width: meta.width,
            height: meta.height,
            fps,
        });
        files.insert(format!("{dir}/sprite.json"), to_json(&meta)?);
    }

    let sprite_name = |id: i32| gmk::get(&bundle.sprites, id).map(|s| s.name.clone());
    let mut objects = Vec::new();
    for (oid, object) in bundle.objects.iter().enumerate() {
        let Some(o) = object else { continue };
        if usage.sprites[oid].is_empty() {
            continue;
        }
        objects.push(IndexObject {
            id: oid,
            name: o.name.clone(),
            parent: gmk::get(&bundle.objects, o.parent).map(|p| p.name.clone()),
            sprite: sprite_name(o.sprite),
            mask: sprite_name(o.mask).filter(|_| o.mask != o.sprite),
            sprites: usage.sprites[oid]
                .iter()
                .filter_map(|&sid| sprite_name(sid as i32))
                .collect(),
        });
    }

    let index = Index {
        format: FORMAT,
        notice: NOTICE,
        license: LICENSE_URL,
        scale,
        room_speed,
        default_fps,
        sprites: index_sprites,
        objects,
    };
    files.insert(INDEX_FILE.to_owned(), to_json(&index)?);
    Ok(files)
}

fn to_json(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut out = serde_json::to_vec_pretty(value)?;
    out.push(b'\n');
    Ok(out)
}

/// What [`export_sprite`] needs besides the sprite.
struct SpriteInfo<'a> {
    id: usize,
    dir: &'a str,
    scale: u32,
    fps: f64,
    fps_source: Option<FpsSource>,
    objects: Vec<String>,
    used_by: Vec<String>,
}

/// Adds one sprite's images to `files` and returns its metadata.
fn export_sprite(
    files: &mut BTreeMap<String, Vec<u8>>,
    pages: &[(u32, u32, Vec<u8>)],
    s: &Sprite,
    info: SpriteInfo<'_>,
) -> Result<SpriteMeta> {
    let SpriteInfo { dir, scale, .. } = info;
    // GM 8 gives every frame the sprite's size, but a frame of another size would still fit a cell this big.
    let cell_w = s.frames.iter().map(|r| r.width).fold(s.width, u32::max);
    let cell_h = s.frames.iter().map(|r| r.height).fold(s.height, u32::max);
    let mut frames = Vec::new();
    for r in &s.frames {
        let mut cell = Image::new(cell_w, cell_h);
        if r.width > 0 && r.height > 0 {
            let page = pages
                .get(r.page as usize)
                .ok_or_else(|| format!("no atlas page {}", r.page))?;
            cell.blit(&crop(page, *r)?, 0, 0);
        }
        frames.push(cell);
    }
    let scaled: Vec<Image> = frames.iter().map(|f| f.upscale(scale)).collect();
    let suffix = format!("@{scale}x");

    let mut frame_meta = Vec::new();
    for (i, (frame, big)) in frames.iter().zip(&scaled).enumerate() {
        let file = format!("{dir}/frames/{i}.png");
        let file_scaled = format!("{dir}/frames/{i}{suffix}.png");
        files.insert(file.clone(), encode(std::slice::from_ref(frame), info.fps)?);
        files.insert(
            file_scaled.clone(),
            encode(std::slice::from_ref(big), info.fps)?,
        );
        frame_meta.push(FrameMeta {
            file,
            file_scaled,
            bbox: s.mask(i).and_then(|m| bbox(m.bbox)),
        });
    }

    // A sprite without frames (or with empty ones) has no strip: PNG can't hold an empty image.
    let mut strip = Image::new(cell_w * frames.len() as u32, cell_h);
    for (i, frame) in frames.iter().enumerate() {
        strip.blit(frame, i as u32 * cell_w, 0);
    }
    let (strip_file, strip_scaled) = if strip.width > 0 && strip.height > 0 {
        let (a, b) = (
            format!("{dir}/strip.png"),
            format!("{dir}/strip{suffix}.png"),
        );
        files.insert(a.clone(), encode(std::slice::from_ref(&strip), info.fps)?);
        files.insert(b.clone(), encode(&[strip.upscale(scale)], info.fps)?);
        (Some(a), Some(b))
    } else {
        (None, None)
    };

    let (anim, anim_scaled) = if frames.len() > 1 {
        let (a, b) = (format!("{dir}/anim.png"), format!("{dir}/anim{suffix}.png"));
        files.insert(a.clone(), encode(&frames, info.fps)?);
        files.insert(b.clone(), encode(&scaled, info.fps)?);
        (Some(a), Some(b))
    } else {
        (None, None)
    };

    let union = s
        .masks
        .iter()
        .map(|m| m.bbox)
        .filter(|b| !b.is_empty())
        .reduce(|a, b| Bbox {
            left: a.left.min(b.left),
            top: a.top.min(b.top),
            right: a.right.max(b.right),
            bottom: a.bottom.max(b.bottom),
        });
    Ok(SpriteMeta {
        id: info.id,
        name: s.name.clone(),
        width: cell_w,
        height: cell_h,
        origin: Point {
            x: s.origin_x,
            y: s.origin_y,
        },
        bbox: union.and_then(bbox),
        fps: info.fps,
        fps_source: info.fps_source,
        strip: strip_file,
        strip_scaled,
        anim,
        anim_scaled,
        frames: frame_meta,
        objects: info.objects,
        used_by: info.used_by,
    })
}

/// Replaces `out` with `files`. They're written to a sibling `.partial` directory first and swapped in
/// at the end, so a run that fails part-way leaves the previous output as it was. To keep a typo in
/// `--out` from deleting something else, an existing `out` must be empty or hold an `index.json`.
fn write_tree(out: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    let name = out
        .file_name()
        .ok_or_else(|| format!("{}: not a directory name", out.display()))?;
    if out.exists() {
        let empty = fs::read_dir(out)
            .map_err(|e| format!("{}: {e}", out.display()))?
            .next()
            .is_none();
        if !empty && !out.join(INDEX_FILE).is_file() {
            return Err(format!(
                "{}: not empty and has no {INDEX_FILE}; refusing to replace it",
                out.display()
            )
            .into());
        }
    }
    let mut partial_name = name.to_owned();
    partial_name.push(".partial");
    let partial = out.with_file_name(partial_name);
    match fs::remove_dir_all(&partial) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("{}: {e}", partial.display()).into());
        }
        _ => {}
    }
    for (rel, data) in files {
        let path = partial.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&path, data).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    if out.exists() {
        fs::remove_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    fs::rename(&partial, out).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speed_of(code: &str) -> OwnSpeed {
        let mut s = OwnSpeed::Inherits(false);
        s.scan(&gml::parse_program(code).unwrap());
        s
    }

    #[test]
    fn create_speed_is_the_last_plain_top_level_assignment() {
        assert_eq!(speed_of("image_speed = 0.4;"), OwnSpeed::Set(0.4));
        assert_eq!(
            speed_of("image_speed = 1; x = 2; image_speed = 0.25"),
            OwnSpeed::Set(0.25)
        );
        // Conditional or computed speeds aren't known.
        assert_eq!(
            speed_of("if (a) image_speed = 0.4;"),
            OwnSpeed::Inherits(false)
        );
        assert_eq!(
            speed_of("image_speed = 0.4; image_speed = a / 2;"),
            OwnSpeed::Inherits(false)
        );
        assert_eq!(speed_of("event_inherited();"), OwnSpeed::Inherits(true));
        // The object's own setting wins over what it inherits.
        assert_eq!(
            speed_of("event_inherited(); image_speed = 0.5"),
            OwnSpeed::Set(0.5)
        );
    }

    #[test]
    fn frame_delays_are_exact_fractions() {
        assert_eq!(frame_delay(12.0), (1, 12));
        assert_eq!(frame_delay(15.0), (1, 15));
        assert_eq!(frame_delay(5.4), (5, 27));
        assert_eq!(frame_delay(30.0), (1, 30));
        assert_eq!(round_fps(0.4 * 30.0), 12.0);
        assert_eq!(round_fps(0.18 * 30.0), 5.4);
    }

    #[test]
    fn upscaling_is_nearest_neighbour() {
        let img = Image {
            width: 2,
            height: 1,
            rgba: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        let big = img.upscale(2);
        assert_eq!((big.width, big.height), (4, 2));
        let row = [1, 2, 3, 4, 1, 2, 3, 4, 5, 6, 7, 8, 5, 6, 7, 8];
        assert_eq!(big.rgba, [row, row].concat());
        assert_eq!(img.upscale(1), img);
    }

    #[test]
    fn arguments_are_checked() {
        assert!(run(["--out".to_owned(), "x".to_owned()].into_iter()).is_err());
        assert!(run(["--scale".to_owned(), "0".to_owned()].into_iter()).is_err());
        assert!(run(["--bogus".to_owned()].into_iter()).is_err());
    }
}
