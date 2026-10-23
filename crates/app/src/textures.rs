//! Pixel data for the renderer, decoded from an asset bundle on the CPU.
//!
//! Images live in two 2D array textures, both bound for every draw so sprites, tiles and backgrounds can
//! share a batch: the bundle's sprite atlas pages, at their own size, and the backgrounds, packed into
//! layers just big enough for them (the largest page is 2048 wide while the backgrounds need far less).
//! Every image keeps at least one transparent pixel of gutter around it, as the extractor's atlas does.
//!
//! Loading takes a `read` callback instead of touching the file system so the web build can hand over
//! bytes it fetched.

use assets::Bundle;
use std::io::Cursor;

/// Largest background layer side: WebGPU's guaranteed minimum for 2D textures.
pub const MAX_LAYER_SIDE: u32 = 2048;

/// Which array texture an image is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bank {
    Sprites,
    Backgrounds,
}

/// Where an image sits in its array texture, in texels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TexRect {
    pub bank: Bank,
    pub layer: u32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// The pixels of one array texture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layers {
    /// Size of every layer.
    pub width: u32,
    pub height: u32,
    /// RGBA8, `width * height * 4` bytes each; at least one.
    pub layers: Vec<Vec<u8>>,
}

impl Layers {
    fn empty() -> Layers {
        Layers {
            width: 1,
            height: 1,
            layers: vec![vec![0; 4]],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteTex {
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub frames: Vec<TexRect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Textures {
    pub sprite_pages: Layers,
    pub background_pages: Layers,
    /// Indexed by sprite id.
    pub sprites: Vec<Option<SpriteTex>>,
    /// Indexed by background id; `None` for deleted or empty backgrounds.
    pub backgrounds: Vec<Option<TexRect>>,
}

/// An RGBA8 image.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn decode_png(bytes: &[u8]) -> Result<Image, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or("PNG too large to decode")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[g, a]| [g, g, g, a])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("unexpanded indexed PNG".into()),
    };
    Ok(Image {
        width: info.width,
        height: info.height,
        rgba,
    })
}

impl Textures {
    /// Every file [`Textures::load`] reads, so the web build can fetch them all up front.
    pub fn files(bundle: &Bundle) -> impl Iterator<Item = &str> {
        let pages = bundle.atlas_pages.iter().map(|page| page.file.as_str());
        let backgrounds = bundle
            .backgrounds
            .iter()
            .filter_map(|bg| bg.as_ref()?.file.as_deref());
        pages.chain(backgrounds)
    }

    /// Decodes the bundle's images. `read` gets paths relative to the bundle directory.
    pub fn load(
        bundle: &Bundle,
        mut read: impl FnMut(&str) -> Result<Vec<u8>, String>,
    ) -> Result<Textures, String> {
        let mut pages = Vec::with_capacity(bundle.atlas_pages.len());
        for page in &bundle.atlas_pages {
            let image =
                decode_png(&read(&page.file)?).map_err(|e| format!("{}: {e}", page.file))?;
            if (image.width, image.height) != (page.width, page.height) {
                return Err(format!("{}: size doesn't match the bundle", page.file));
            }
            pages.push(image);
        }

        let mut backgrounds = Vec::new();
        for (id, bg) in bundle.backgrounds.iter().enumerate() {
            let Some(file) = bg.as_ref().and_then(|bg| bg.file.as_ref()) else {
                continue;
            };
            let image = decode_png(&read(file)?).map_err(|e| format!("{file}: {e}"))?;
            backgrounds.push((id, image));
        }

        // Atlas pages are all one size in practice; a smaller one sits in the corner of its layer.
        let sprite_pages = if pages.is_empty() {
            Layers::empty()
        } else {
            let width = pages.iter().map(|p| p.width).max().unwrap_or(1).max(1);
            let height = pages.iter().map(|p| p.height).max().unwrap_or(1).max(1);
            let layers = pages
                .iter()
                .map(|page| {
                    let mut layer = vec![0; (width * height * 4) as usize];
                    blit(&mut layer, width, page, 0, 0);
                    layer
                })
                .collect();
            Layers {
                width,
                height,
                layers,
            }
        };

        let (background_pages, bg_rects) = pack_backgrounds(bundle.backgrounds.len(), backgrounds)?;

        let sprites = bundle
            .sprites
            .iter()
            .map(|s| {
                s.as_ref().map(|s| SpriteTex {
                    width: s.width,
                    height: s.height,
                    origin_x: s.origin_x,
                    origin_y: s.origin_y,
                    frames: s
                        .frames
                        .iter()
                        .map(|f| TexRect {
                            bank: Bank::Sprites,
                            layer: f.page,
                            x: f.x,
                            y: f.y,
                            w: f.width,
                            h: f.height,
                        })
                        .collect(),
                })
            })
            .collect();

        Ok(Textures {
            sprite_pages,
            background_pages,
            sprites,
            backgrounds: bg_rects,
        })
    }
}

/// Shelf-packs backgrounds (tallest first) into as few layers as needed, each as narrow as the widest
/// background allows and only as tall as the fullest layer needs.
fn pack_backgrounds(
    slots: usize,
    mut images: Vec<(usize, Image)>,
) -> Result<(Layers, Vec<Option<TexRect>>), String> {
    let mut rects = vec![None; slots];
    if images.is_empty() {
        return Ok((Layers::empty(), rects));
    }
    images.sort_by_key(|(id, image)| (std::cmp::Reverse(image.height), *id));
    let widest = images.iter().map(|(_, i)| i.width).max().unwrap_or(0) + 2;
    let tallest = images.iter().map(|(_, i)| i.height).max().unwrap_or(0) + 2;
    if widest > MAX_LAYER_SIDE || tallest > MAX_LAYER_SIDE {
        return Err(format!(
            "a background is larger than {MAX_LAYER_SIDE}x{MAX_LAYER_SIDE} with its gutter"
        ));
    }
    let width = widest.next_power_of_two().clamp(256, MAX_LAYER_SIDE);

    // Place everything first, then size and fill the layers.
    let mut placed = Vec::with_capacity(images.len());
    let mut shelf = Shelf::new(width, MAX_LAYER_SIDE);
    let mut layer = 0;
    let mut height = 1;
    for (id, image) in &images {
        let (x, y) = match shelf.place(image.width, image.height) {
            Some(at) => at,
            None => {
                height = height.max(shelf.used_height());
                layer += 1;
                shelf = Shelf::new(width, MAX_LAYER_SIDE);
                shelf
                    .place(image.width, image.height)
                    .expect("an empty layer fits any background")
            }
        };
        placed.push((*id, layer, x, y));
    }
    height = height.max(shelf.used_height());

    let mut layers = vec![vec![0; (width * height * 4) as usize]; layer as usize + 1];
    for ((id, layer, x, y), (_, image)) in placed.into_iter().zip(&images) {
        blit(&mut layers[layer as usize], width, image, x, y);
        rects[id] = Some(TexRect {
            bank: Bank::Backgrounds,
            layer,
            x,
            y,
            w: image.width,
            h: image.height,
        });
    }
    Ok((
        Layers {
            width,
            height,
            layers,
        },
        rects,
    ))
}

fn blit(layer: &mut [u8], layer_width: u32, image: &Image, x: u32, y: u32) {
    let row = (image.width * 4) as usize;
    for (i, src) in image.rgba.chunks_exact(row).enumerate() {
        let start = (((y + i as u32) * layer_width + x) * 4) as usize;
        layer[start..start + row].copy_from_slice(src);
    }
}

/// Places rectangles left to right in rows, keeping a 1px transparent gutter around each.
struct Shelf {
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    row_height: u32,
}

impl Shelf {
    fn new(width: u32, height: u32) -> Shelf {
        Shelf {
            width,
            height,
            x: 1,
            y: 1,
            row_height: 0,
        }
    }

    fn place(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.x + w + 1 > self.width {
            self.x = 1;
            self.y += self.row_height + 1;
            self.row_height = 0;
        }
        if self.x + w + 1 > self.width || self.y + h + 1 > self.height {
            return None;
        }
        let at = (self.x, self.y);
        self.x += w + 1;
        self.row_height = self.row_height.max(h);
        Some(at)
    }

    /// Rows used so far, including the gutter below the last row.
    fn used_height(&self) -> u32 {
        self.y + self.row_height + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shelf_keeps_gutters_and_wraps() {
        // Two 4-wide images and three gutters exactly fill 11 pixels.
        let mut shelf = Shelf::new(11, 11);
        assert_eq!(shelf.place(4, 3), Some((1, 1)));
        assert_eq!(shelf.place(4, 2), Some((6, 1)));
        // No room left on the row: the next row starts below the tallest plus a gutter.
        assert_eq!(shelf.place(4, 4), Some((1, 5)));
        // Too tall for what's left.
        assert_eq!(shelf.place(4, 6), None);
        assert_eq!(shelf.used_height(), 10);
    }

    #[test]
    fn backgrounds_pack_into_small_layers() {
        let image = |w, h| Image {
            width: w,
            height: h,
            rgba: vec![255; (w * h * 4) as usize],
        };
        let (layers, rects) =
            pack_backgrounds(4, vec![(0, image(32, 32)), (2, image(480, 200))]).unwrap();
        // Narrow (the widest needs 482, so 512) and only as tall as used.
        assert_eq!((layers.width, layers.height), (512, 202 + 33));
        assert_eq!(layers.layers.len(), 1);
        assert_eq!(rects[2].map(|r| (r.x, r.y)), Some((1, 1)));
        assert_eq!(rects[0].map(|r| (r.x, r.y)), Some((1, 202)));
        assert_eq!((rects[1], rects[3]), (None, None));
        assert!(pack_backgrounds(1, vec![(0, image(3000, 1))]).is_err());
    }
}
