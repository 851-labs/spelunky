use crate::MAX_TREE_DEPTH;
use crate::error::ErrorKind;
use crate::reader::{Reader, Result};

/// A node of the editor's resource tree.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeNode {
    /// 1 primary (a top-level section), 2 group (a folder), 3 secondary (a resource).
    pub status: i32,
    /// Resource kind: 1 object, 2 sprite, 3 sound, 4 room, 6 background, 7 script, 8 path, 9 font,
    /// 10 game information, 11 game settings, 12 timeline, 13 extension packages.
    pub kind: i32,
    /// Resource id for a secondary node.
    pub index: i32,
    pub name: String,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub const PRIMARY: i32 = 1;
    pub const GROUP: i32 = 2;
    pub const SECONDARY: i32 = 3;
}

/// GM 8.0 always writes these twelve top-level sections.
const ROOTS: usize = 12;

pub(crate) fn read_tree(r: &mut Reader<'_>) -> Result<Vec<TreeNode>> {
    (0..ROOTS).map(|_| read_node(r, 0)).collect()
}

fn read_node(r: &mut Reader<'_>, depth: usize) -> Result<TreeNode> {
    if depth > MAX_TREE_DEPTH {
        return Err(r.err(ErrorKind::TreeTooDeep));
    }
    let status = r.i32()?;
    let kind = r.i32()?;
    let index = r.i32()?;
    let name = r.string()?;
    let n = r.count(20)?;
    let mut children = Vec::new();
    for _ in 0..n {
        children.push(read_node(r, depth + 1)?);
    }
    Ok(TreeNode {
        status,
        kind,
        index,
        name,
        children,
    })
}
