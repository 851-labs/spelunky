//! Exports every sprite in an extracted asset bundle as the wiki's media: frames, strips and animated
//! previews, with JSON metadata and an object-to-sprites index.
//!
//! ```sh
//! wiki-sprites --assets assets/ --out wiki/media/sprites [--scale 4]
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
//! (or inherits one from its parent's), and nothing conditional changes it. An object that sets 0 holds
//! its sprite still and picks frames in code. Failing both, an object that uses the sprite and sets a
//! speed that way is a weaker hint. Anything else, and still sprites, play at [`DEFAULT_IMAGE_SPEED`],
//! and the metadata says which it was.
//!
//! The previews are APNG rather than animated WebP: the `png` crate the workspace already uses writes
//! APNG, every current browser plays it, and it keeps full alpha losslessly. Animated WebP would need
//! libwebp (a C library) or a hand-written WebP muxer, for files a few hundred bytes smaller.
//!
//! # Determinism
//!
//! The same bundle gives byte-identical output: everything is built in memory first, from id-ordered
//! lists and sorted maps, with fixed PNG settings. The output directory is replaced as a whole, written
//! to a sibling `.partial` directory and swapped in by renames, so a failed run leaves the previous output
//! intact. It only ever deletes directories that are empty or hold an `index.json`.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use assets::{AtlasRect, Bbox, Bundle, EventType, Object, Sprite};
use gml::ast::{AssignOp, ExprKind, StmtKind};
use gml::visit::{self, Visitor};
use serde::Serialize;
use tools::{Arg, Args};

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

// The copyright notice the license asks for, in the index and in every PNG.
use tools::{WIKI_LICENSE_URL as LICENSE_URL, WIKI_NOTICE as NOTICE};

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

fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let mut args = Args::new(args);
    let (mut assets_dir, mut out, mut scale) = (None, None, DEFAULT_SCALE);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Flag(flag) => flag,
            Arg::Positional(p) => return Err(format!("unexpected argument {p:?}\n{USAGE}").into()),
        };
        match flag.name.as_str() {
            "-h" | "--help" => {
                args.switch(&flag)?;
                println!("{USAGE}");
                return Ok(());
            }
            "--assets" => assets_dir = Some(PathBuf::from(args.value(&flag)?)),
            "--out" => out = Some(PathBuf::from(args.value(&flag)?)),
            "--scale" => {
                scale = args.number(&flag)?;
                if !(1..=MAX_SCALE).contains(&scale) {
                    return Err(format!("bad --scale {scale}: 1 to {MAX_SCALE}").into());
                }
            }
            name => return Err(format!("unexpected argument {name:?}\n{USAGE}").into()),
        }
    }
    let (Some(assets_dir), Some(out)) = (assets_dir, out) else {
        return Err(USAGE.into());
    };
    // Before the export's few seconds of work, so a mistyped --out fails at once.
    let work = work_dirs(&out)?;
    for dir in [&out, &work.0, &work.1] {
        check_replaceable(dir)?;
    }

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
    write_tree(&out, &work, &files)?;
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
    /// (`gml::lexer::decimal_value`). 0 for a sprite its object holds still, picking frames in code; its
    /// preview plays at the default rate.
    image_speed: f64,
}

#[derive(Serialize)]
struct FrameMeta {
    /// `None` for an empty (0-pixel) frame, which PNG can't hold.
    file: Option<String>,
    file_scaled: Option<String>,
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
    /// Per object id, what its create event (with its parents') does to `image_speed`: `None` if it
    /// leaves it alone, `Some(None)` if it sets something this can't read for sure.
    create_speed: Vec<Option<Option<f64>>>,
}

/// One step of a create event, as far as `image_speed` goes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SpeedOp {
    /// Sets it: to a plain number, or `None` for a value that isn't one, or set only conditionally.
    Set(Option<f64>),
    /// Runs the parent's create event (`event_inherited`, or the inherited action).
    Inherit,
}

fn analyse(bundle: &Bundle) -> Result<Usage> {
    let sprite_ids: BTreeMap<&str, usize> = bundle
        .sprites
        .iter()
        .enumerate()
        .filter_map(|(id, s)| Some((s.as_ref()?.name.as_str(), id)))
        .collect();
    let mut sprites = Vec::new();
    // Per object id, its create event's ops, or `None` without a create event.
    let mut create_ops: Vec<Option<Vec<SpeedOp>>> = Vec::new();
    for object in &bundle.objects {
        let mut used = BTreeSet::new();
        let mut ops = None;
        if let Some(o) = object {
            for id in [o.sprite, o.mask] {
                if gmk::get(&bundle.sprites, id).is_some() {
                    used.insert(id as usize);
                }
            }
            for event in &o.events {
                let create = event.event_type == EventType::Create;
                let mut event_ops = Vec::new();
                // After a drag-and-drop question, the actions run only if it says so.
                let mut conditional = false;
                for action in &event.actions {
                    for arg in &action.args {
                        if arg.kind == assets::ArgKind::Sprite
                            && let Ok(id) = arg.value.trim().parse::<i32>()
                            && gmk::get(&bundle.sprites, id).is_some()
                        {
                            used.insert(id as usize);
                        }
                    }
                    conditional |= action.is_question;
                    if action.function == "action_inherited" {
                        event_ops.push(if conditional {
                            SpeedOp::Set(None)
                        } else {
                            SpeedOp::Inherit
                        });
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
                        speed_ops(&program, conditional, &mut event_ops);
                    }
                }
                if create {
                    ops = Some(event_ops);
                }
            }
        }
        sprites.push(used);
        create_ops.push(ops);
    }

    let create_speed = (0..create_ops.len())
        .map(|id| create_effect(bundle, &create_ops, id, 0))
        .collect();
    Ok(Usage {
        sprites,
        create_speed,
    })
}

/// What object `id`'s create event does to `image_speed` (see [`Usage::create_speed`]). Without a create
/// event an object runs its parent's.
fn create_effect(
    bundle: &Bundle,
    ops: &[Option<Vec<SpeedOp>>],
    id: usize,
    depth: usize,
) -> Option<Option<f64>> {
    // A parent chain is finite in a valid game, but a cycle mustn't hang the tool.
    if depth > ops.len() {
        return Some(None);
    }
    let parent = || {
        let o = bundle.objects[id].as_ref()?;
        gmk::get(&bundle.objects, o.parent)?;
        create_effect(bundle, ops, o.parent as usize, depth + 1)
    };
    match &ops[id] {
        Some(own) => fold_ops(own, parent),
        None => parent(),
    }
}

/// Runs a create event's ops, with `parent` giving what the parent's create event does.
fn fold_ops(ops: &[SpeedOp], parent: impl Fn() -> Option<Option<f64>>) -> Option<Option<f64>> {
    ops.iter().fold(None, |speed, op| match *op {
        SpeedOp::Set(v) => Some(v),
        // A parent that leaves the speed alone leaves it as it was.
        SpeedOp::Inherit => parent().or(speed),
    })
}

/// Appends what one code action does to `image_speed`, statement by statement. Only a top-level
/// `image_speed = <number>` is a plain setting: one inside an `if` or a loop, or after a drag-and-drop
/// question (`conditional`), may or may not run, so it makes the speed unknown.
fn speed_ops(program: &gml::Program, conditional: bool, ops: &mut Vec<SpeedOp>) {
    for stmt in &program.body {
        let op = match &stmt.kind {
            StmtKind::Assign {
                place,
                op: AssignOp::Set,
                value,
            } if is_ident(place, "image_speed") => Some(SpeedOp::Set(match value.kind {
                ExprKind::Real(v) if !conditional => Some(v),
                _ => None,
            })),
            StmtKind::Call(call) if is_call(call, "event_inherited") && !conditional => {
                Some(SpeedOp::Inherit)
            }
            _ => {
                let mut find = TouchesSpeed(false);
                find.visit_stmt(stmt);
                find.0.then_some(SpeedOp::Set(None))
            }
        };
        ops.extend(op);
    }
}

fn is_ident(expr: &gml::Expr, name: &str) -> bool {
    matches!(&expr.kind, ExprKind::Ident(n) if n == name)
}

fn is_call(expr: &gml::Expr, function: &str) -> bool {
    matches!(&expr.kind, ExprKind::Call { name, .. } if name.name == function)
}

/// Whether a statement assigns `image_speed` or runs the parent's event anywhere inside it.
struct TouchesSpeed(bool);

impl Visitor for TouchesSpeed {
    fn visit_stmt(&mut self, stmt: &gml::Stmt) {
        if let StmtKind::Assign { place, .. } = &stmt.kind
            && is_ident(place, "image_speed")
        {
            self.0 = true;
        }
        visit::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &gml::Expr) {
        self.0 |= is_call(expr, "event_inherited");
        visit::walk_expr(self, expr);
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

/// Frames of one size side by side, left to right.
fn strip(frames: &[Image]) -> Image {
    let (w, h) = frames.first().map_or((0, 0), |f| (f.width, f.height));
    let mut out = Image::new(w * frames.len() as u32, h);
    for (i, frame) in frames.iter().enumerate() {
        out.blit(frame, i as u32 * w, 0);
    }
    out
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

/// A PNG of one image.
fn still(image: &Image) -> Result<Vec<u8>> {
    encode(std::slice::from_ref(image), 0.0)
}

/// A PNG of one image or, with several, an APNG that loops them forever at `fps`. Each carries the
/// copyright notice as a `Copyright` text chunk. Settings are `tools::png_encoder`'s, so the bytes only
/// depend on the input.
fn encode(frames: &[Image], fps: f64) -> Result<Vec<u8>> {
    let first = frames.first().ok_or("no frames to encode")?;
    let mut out = Vec::new();
    let mut encoder = tools::png_encoder(&mut out, first.width, first.height);
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
    let room_speed = tools::room_speed(bundle);
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
        // In order: the first object (by id) whose own sprite this is and whose create event sets a
        // speed; one that holds it still (oBow's sBowRight: its frames are states the code picks); the
        // first that only uses it and sets a speed, since an object's sprites mostly share its speed
        // (oSnake switches to sSnakeWalkL at its create event's 0.4).
        let source_in = |objects: &[usize], via: &'static str, moving: bool| {
            objects.iter().find_map(|&oid| {
                Some(FpsSource {
                    image_speed: usage.create_speed[oid]
                        .flatten()
                        .filter(|&v| (v > 0.0) == moving && v >= 0.0)?,
                    object: name_of(&bundle.objects, oid)?,
                    via,
                })
            })
        };
        let source = source_in(owners, "sprite", true)
            .or_else(|| source_in(owners, "sprite", false))
            .or_else(|| source_in(users, "uses", true));
        // A still sprite's preview plays at the default rate: it shows the frames, not a speed.
        let image_speed = source
            .as_ref()
            .map(|s| s.image_speed)
            .filter(|&v| v > 0.0)
            .unwrap_or(DEFAULT_IMAGE_SPEED);
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
    // PNG can't hold an empty image, so a sprite of 0-pixel frames (or none) has no files, only metadata.
    let has_pixels = cell_w > 0 && cell_h > 0 && !frames.is_empty();
    // Adds a pair of files, native and upscaled, returning their paths.
    let mut add = |name: &str, native: Vec<u8>, big: Vec<u8>| {
        let (a, b) = (
            format!("{dir}/{name}.png"),
            format!("{dir}/{name}{suffix}.png"),
        );
        files.insert(a.clone(), native);
        files.insert(b.clone(), big);
        (Some(a), Some(b))
    };

    let mut frame_meta = Vec::new();
    for (i, (frame, big)) in frames.iter().zip(&scaled).enumerate() {
        let (file, file_scaled) = if has_pixels {
            add(&format!("frames/{i}"), still(frame)?, still(big)?)
        } else {
            (None, None)
        };
        frame_meta.push(FrameMeta {
            file,
            file_scaled,
            bbox: s.mask(i).and_then(|m| bbox(m.bbox)),
        });
    }

    let (strip_file, strip_scaled) = if has_pixels {
        add("strip", still(&strip(&frames))?, still(&strip(&scaled))?)
    } else {
        (None, None)
    };

    let (anim, anim_scaled) = if has_pixels && frames.len() > 1 {
        add(
            "anim",
            encode(&frames, info.fps)?,
            encode(&scaled, info.fps)?,
        )
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

/// The sibling directories a run writes into before swapping it in for `out`, and moves the previous
/// output aside to while it does.
fn work_dirs(out: &Path) -> Result<(PathBuf, PathBuf)> {
    let name = out
        .file_name()
        .ok_or_else(|| format!("{}: not a directory name", out.display()))?;
    let sibling = |suffix: &str| {
        let mut n = name.to_owned();
        n.push(suffix);
        out.with_file_name(n)
    };
    Ok((sibling(".partial"), sibling(".old")))
}

/// Errors unless `dir` is missing, empty, or holds an `index.json`: the only directories the exporter
/// deletes, so a typo in `--out` can't delete something else.
fn check_replaceable(dir: &Path) -> Result<()> {
    let mut entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("{}: {e}", dir.display()).into()),
    };
    if entries.next().is_some() && !dir.join(INDEX_FILE).is_file() {
        return Err(format!(
            "{}: not empty and has no {INDEX_FILE}; refusing to replace it",
            dir.display()
        )
        .into());
    }
    Ok(())
}

fn remove_dir(dir: &Path) -> Result<()> {
    match fs::remove_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("{}: {e}", dir.display()).into())
        }
        _ => Ok(()),
    }
}

/// Replaces `out` with `files`, using the [`work_dirs`] (all three checked with [`check_replaceable`]).
/// The files are written to `partial` first, `index.json` before the rest so a run that dies part-way
/// leaves a directory the next run may delete, and swapped in by renames at the end: a failed run leaves
/// the previous output as it was.
fn write_tree(
    out: &Path,
    (partial, old): &(PathBuf, PathBuf),
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    remove_dir(partial)?;
    let index = files
        .get_key_value(INDEX_FILE)
        .ok_or("no index.json was built")?;
    let rest = files.iter().filter(|(rel, _)| *rel != INDEX_FILE);
    for (rel, data) in std::iter::once(index).chain(rest) {
        let path = partial.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&path, data).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    if !out.exists() {
        return Ok(fs::rename(partial, out).map_err(|e| format!("{}: {e}", out.display()))?);
    }
    remove_dir(old)?;
    fs::rename(out, old).map_err(|e| format!("{}: {e}", out.display()))?;
    if let Err(e) = fs::rename(partial, out) {
        // Put the previous output back.
        let _ = fs::rename(old, out);
        return Err(format!("{}: {e}", out.display()).into());
    }
    remove_dir(old)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a create event of `code` leaves `image_speed` at, under a parent's that leaves it at `parent`.
    fn speed_of(code: &str, conditional: bool, parent: Option<Option<f64>>) -> Option<Option<f64>> {
        let mut ops = Vec::new();
        speed_ops(&gml::parse_program(code).unwrap(), conditional, &mut ops);
        fold_ops(&ops, || parent)
    }

    #[test]
    fn create_speed_is_what_the_event_leaves_plainly() {
        let parent = Some(Some(0.2));
        assert_eq!(speed_of("x = 1;", false, parent), None);
        assert_eq!(speed_of("image_speed = 0.4;", false, None), Some(Some(0.4)));
        assert_eq!(
            speed_of("image_speed = 1; x = 2; image_speed = 0.25", false, None),
            Some(Some(0.25))
        );
        assert_eq!(speed_of("image_speed = 0", false, None), Some(Some(0.0)));
        // Conditional or computed speeds aren't known.
        assert_eq!(
            speed_of("if (a) image_speed = 0.4;", false, None),
            Some(None)
        );
        assert_eq!(speed_of("image_speed = 0.4;", true, None), Some(None));
        assert_eq!(
            speed_of("image_speed = 0.4; image_speed = a / 2;", false, None),
            Some(None)
        );
        // The parent's event runs where `event_inherited` is, so whichever sets the speed last wins.
        assert_eq!(speed_of("event_inherited();", false, parent), parent);
        assert_eq!(
            speed_of("event_inherited(); image_speed = 0.5", false, parent),
            Some(Some(0.5))
        );
        assert_eq!(
            speed_of("image_speed = 0.5; event_inherited()", false, parent),
            parent
        );
        assert_eq!(
            speed_of("image_speed = 0.5; event_inherited()", false, None),
            Some(Some(0.5))
        );
        assert_eq!(
            speed_of("if (a) event_inherited()", false, parent),
            Some(None)
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
