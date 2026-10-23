//! Pixel data for the renderer, decoded from an asset bundle on the CPU.
//!
//! Everything sampled by sprites, tiles and backgrounds goes into one 2D array texture so a whole frame of
//! them can share a bind group: the bundle's sprite atlas pages come first, then the backgrounds packed
//! into extra layers of the same size. Every image keeps at least one transparent pixel of gutter around
//! it, as the extractor's atlas does.
//!
//! Loading takes a `read` callback instead of touching the file system so the web build can hand over
//! bytes it fetched.

use assets::Bundle;
use std::io::Cursor;

/// Where an image sits in the texture array, in texels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TexRect {
    pub layer: u32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
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
    /// Size of every layer.
    pub width: u32,
    pub height: u32,
    /// RGBA8, `width * height * 4` bytes each.
    pub layers: Vec<Vec<u8>>,
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

        // Layers are as big as the largest page, and at least big enough for any background.
        let mut width = pages.iter().map(|p| p.width).max().unwrap_or(0);
        let mut height = pages.iter().map(|p| p.height).max().unwrap_or(0);
        for (_, image) in &backgrounds {
            width = width.max(image.width + 2);
            height = height.max(image.height + 2);
        }
        let (width, height) = (width.max(1), height.max(1));

        let mut layers: Vec<Vec<u8>> = pages
            .iter()
            .map(|page| {
                let mut layer = vec![0; (width * height * 4) as usize];
                blit(&mut layer, width, page, 0, 0);
                layer
            })
            .collect();

        let mut bg_rects = vec![None; bundle.backgrounds.len()];
        // Shelf packing, tallest first; the images are few and small.
        backgrounds.sort_by_key(|(id, image)| (std::cmp::Reverse(image.height), *id));
        let mut shelf = Shelf::new(width, height);
        let mut layer = None;
        for (id, image) in &backgrounds {
            let (x, y) = match shelf.place(image.width, image.height) {
                Some(at) if layer.is_some() => at,
                _ => {
                    layers.push(vec![0; (width * height * 4) as usize]);
                    layer = Some(layers.len() - 1);
                    shelf = Shelf::new(width, height);
                    shelf
                        .place(image.width, image.height)
                        .expect("a layer fits any background")
                }
            };
            let index = layer.expect("a background layer exists");
            blit(&mut layers[index], width, image, x, y);
            bg_rects[*id] = Some(TexRect {
                layer: index as u32,
                x,
                y,
                w: image.width,
                h: image.height,
            });
        }

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
            width,
            height,
            layers,
            sprites,
            backgrounds: bg_rects,
        })
    }
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
    }
}
