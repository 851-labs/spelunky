//! Collision masks, computed from a sprite's pixels the way the GM 8.0 IDE does when it builds a game.
//!
//! The `.gmk` stores only the mask *settings*; GM bakes the masks into the executable. These rules were
//! read from OpenGMK (`gm8emulator/src/asset/sprite.rs`: `make_bbox`, `make_colliders_precise`,
//! `make_colliders_shaped`; `gm8exe/src/asset/sprite.rs` for the stored layout) and then checked against
//! the masks baked into the released `Spelunky.exe`: every one of its 2,299 masks matches what this module
//! computes from `spelunky.gmk`, bit for bit, bounding box included. That file only exercises precise and
//! rectangle masks with separate masks on, automatic bounding boxes and tolerance 0, so the rules for the
//! other settings are OpenGMK's reading of GM and haven't been checked against GM itself.
//!
//! - A pixel is solid when its alpha is strictly greater than the tolerance. *(Checked.)*
//! - With separate masks, each frame gets its own mask; otherwise there's one mask, the union of the
//!   frames' solid pixels. *(Separate checked.)*
//! - The automatic bounding box is the tight box around the solid pixels. A frame without solid pixels gets
//!   the "inverted" box left = w-1, right = 0, top = h-1, bottom = 0, and no solid pixels. *(Checked: six
//!   such frames in the exe.)*
//! - A rectangle mask is its bounding box, filled. With an automatic box, that's the tight box of the
//!   frame's alpha. *(Checked.)*
//! - Full-image and manual boxes replace the box. A precise mask keeps its pixels but nothing outside the
//!   box collides, because GM tests the box before the pixels; this module clears those pixels so that the
//!   mask alone is enough. *(Unchecked.)*
//! - Disk and diamond shapes are unsupported: OpenGMK's versions of them have known open questions, and
//!   Spelunky doesn't use them.

use crate::resources::{BboxMode, MaskShape, Sprite};
use std::fmt;

/// An inclusive pixel box. `left > right` or `top > bottom` means the mask is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bbox {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl Bbox {
    /// What GM stores for a frame without solid pixels.
    pub fn empty(width: u32, height: u32) -> Bbox {
        Bbox {
            left: width.saturating_sub(1),
            top: height.saturating_sub(1),
            right: 0,
            bottom: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.left > self.right || self.top > self.bottom
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }
}

/// One collision mask, as GM 8.0 stores it in a compiled game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionMask {
    pub width: u32,
    pub height: u32,
    pub bbox: Bbox,
    /// `width * height` flags, row-major. Every `true` lies inside `bbox`.
    pub solid: Vec<bool>,
}

impl CollisionMask {
    pub fn get(&self, x: u32, y: u32) -> bool {
        x < self.width
            && y < self.height
            && self.solid[y as usize * self.width as usize + x as usize]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaskError {
    /// GM 8.0 requires every frame of a sprite to have the same size.
    FrameSizeMismatch {
        frame: usize,
    },
    UnsupportedShape(MaskShape),
}

impl fmt::Display for MaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MaskError::FrameSizeMismatch { frame } => {
                write!(f, "frame {frame} differs in size from frame 0")
            }
            MaskError::UnsupportedShape(shape) => {
                write!(f, "{shape:?} collision masks are not supported")
            }
        }
    }
}

impl std::error::Error for MaskError {}

/// Tight box around the `true` cells of `solid`, or [`Bbox::empty`].
fn tight_bbox(solid: &[bool], width: u32, height: u32) -> Bbox {
    let mut b = Bbox {
        left: u32::MAX,
        top: u32::MAX,
        right: 0,
        bottom: 0,
    };
    let mut any = false;
    for y in 0..height {
        for x in 0..width {
            if solid[y as usize * width as usize + x as usize] {
                any = true;
                b.left = b.left.min(x);
                b.right = b.right.max(x);
                b.top = b.top.min(y);
                b.bottom = b.bottom.max(y);
            }
        }
    }
    if any { b } else { Bbox::empty(width, height) }
}

impl Sprite {
    /// The collision masks GM 8.0 would build for this sprite: one per frame with separate masks, else a
    /// single mask shared by every frame. Empty for a sprite without frames.
    pub fn collision_masks(&self) -> Result<Vec<CollisionMask>, MaskError> {
        if matches!(self.shape, MaskShape::Disk | MaskShape::Diamond) {
            return Err(MaskError::UnsupportedShape(self.shape));
        }
        let (width, height) = (self.width(), self.height());
        if let Some(frame) = self
            .frames
            .iter()
            .position(|f| f.width != width || f.height != height)
        {
            return Err(MaskError::FrameSizeMismatch { frame });
        }
        if self.frames.is_empty() {
            return Ok(Vec::new());
        }
        let tolerance = self.alpha_tolerance.clamp(0, 255) as u8;
        let alpha_solid = |frames: &[crate::Image]| -> Vec<bool> {
            let mut solid = vec![false; width as usize * height as usize];
            for frame in frames {
                for (cell, px) in solid.iter_mut().zip(frame.bgra.as_chunks::<4>().0) {
                    *cell |= px[3] > tolerance;
                }
            }
            solid
        };
        let groups: Vec<&[crate::Image]> = if self.separate_masks {
            self.frames.chunks(1).collect()
        } else {
            vec![&self.frames[..]]
        };
        let fixed_bbox = match self.bbox_mode {
            BboxMode::Automatic => None,
            BboxMode::FullImage => Some(Bbox {
                left: 0,
                top: 0,
                right: width.saturating_sub(1),
                bottom: height.saturating_sub(1),
            }),
            BboxMode::Manual => Some(self.manual_bbox()),
        };
        Ok(groups
            .into_iter()
            .map(|frames| {
                let mut solid = alpha_solid(frames);
                let bbox = fixed_bbox.unwrap_or_else(|| tight_bbox(&solid, width, height));
                for y in 0..height {
                    for x in 0..width {
                        let cell = &mut solid[y as usize * width as usize + x as usize];
                        *cell =
                            bbox.contains(x, y) && (self.shape == MaskShape::Rectangle || *cell);
                    }
                }
                CollisionMask {
                    width,
                    height,
                    bbox,
                    solid,
                }
            })
            .collect())
    }

    /// The stored box, clamped to the image as the sprite editor's fields are.
    fn manual_bbox(&self) -> Bbox {
        let clamp = |v: i32, size: u32| v.clamp(0, size.saturating_sub(1) as i32) as u32;
        let (w, h) = (self.width(), self.height());
        Bbox {
            left: clamp(self.bbox_left, w),
            top: clamp(self.bbox_top, h),
            right: clamp(self.bbox_right, w),
            bottom: clamp(self.bbox_bottom, h),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Image;

    fn sprite(shape: MaskShape, separate: bool, mode: BboxMode, frames: &[&[u8]]) -> Sprite {
        // 3x2 frames given as alpha values.
        Sprite {
            name: "s".into(),
            origin_x: 0,
            origin_y: 0,
            frames: frames
                .iter()
                .map(|a| Image {
                    width: 3,
                    height: 2,
                    bgra: a.iter().flat_map(|&a| [9, 9, 9, a]).collect(),
                })
                .collect(),
            shape,
            alpha_tolerance: 10,
            separate_masks: separate,
            bbox_mode: mode,
            bbox_left: 1,
            bbox_right: 5,
            bbox_bottom: 0,
            bbox_top: -3,
        }
    }

    const A: &[u8] = &[0, 11, 0, 0, 10, 0];
    const B: &[u8] = &[0, 0, 0, 0, 0, 255];
    const EMPTY: &[u8] = &[10, 10, 10, 10, 10, 10];

    #[test]
    fn precise_uses_strict_tolerance() {
        let m = sprite(MaskShape::Precise, true, BboxMode::Automatic, &[A])
            .collision_masks()
            .unwrap();
        assert_eq!(m[0].solid, [false, true, false, false, false, false]);
        assert_eq!(
            m[0].bbox,
            Bbox {
                left: 1,
                top: 0,
                right: 1,
                bottom: 0
            }
        );
    }

    #[test]
    fn shared_mask_is_the_union() {
        let m = sprite(MaskShape::Precise, false, BboxMode::Automatic, &[A, B])
            .collision_masks()
            .unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].solid, [false, true, false, false, false, true]);
        assert_eq!(
            m[0].bbox,
            Bbox {
                left: 1,
                top: 0,
                right: 2,
                bottom: 1
            }
        );
    }

    #[test]
    fn rectangle_fills_the_box() {
        let m = sprite(MaskShape::Rectangle, false, BboxMode::Automatic, &[A, B])
            .collision_masks()
            .unwrap();
        assert_eq!(m[0].solid, [false, true, true, false, true, true]);
    }

    #[test]
    fn empty_frame_gets_inverted_box() {
        let m = sprite(MaskShape::Rectangle, true, BboxMode::Automatic, &[EMPTY])
            .collision_masks()
            .unwrap();
        assert_eq!(
            m[0].bbox,
            Bbox {
                left: 2,
                top: 1,
                right: 0,
                bottom: 0
            }
        );
        assert!(m[0].bbox.is_empty());
        assert!(m[0].solid.iter().all(|&s| !s));
    }

    #[test]
    fn manual_box_is_clamped_and_clips_precise_pixels() {
        let m = sprite(MaskShape::Precise, true, BboxMode::Manual, &[B])
            .collision_masks()
            .unwrap();
        assert_eq!(
            m[0].bbox,
            Bbox {
                left: 1,
                top: 0,
                right: 2,
                bottom: 0
            }
        );
        assert!(m[0].solid.iter().all(|&s| !s));
        let m = sprite(MaskShape::Rectangle, true, BboxMode::FullImage, &[EMPTY])
            .collision_masks()
            .unwrap();
        assert!(m[0].solid.iter().all(|&s| s));
    }

    #[test]
    fn unsupported_inputs_are_errors() {
        let s = sprite(MaskShape::Disk, true, BboxMode::Automatic, &[A]);
        assert_eq!(
            s.collision_masks(),
            Err(MaskError::UnsupportedShape(MaskShape::Disk))
        );
        let mut s = sprite(MaskShape::Precise, true, BboxMode::Automatic, &[A, B]);
        s.frames[1].width = 1;
        assert_eq!(
            s.collision_masks(),
            Err(MaskError::FrameSizeMismatch { frame: 1 })
        );
    }
}
