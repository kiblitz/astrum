use crate::buffer::Cursor;
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_PANE_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Vertical,   // side by side |
    Horizontal, // stacked ─
}

#[derive(Debug)]
pub struct Pane {
    pub id: usize,
    pub buffer_id: Option<usize>,
    pub cursor: Cursor,
    pub scroll_offset: usize,
    cursor_cache: HashMap<usize, (Cursor, usize)>,
    /// Content height in rows (updated by the renderer each frame).
    pub height: Cell<u16>,
}

impl Pane {
    pub fn new(buffer_id: Option<usize>) -> Self {
        Self {
            id: NEXT_PANE_ID.fetch_add(1, Ordering::Relaxed),
            buffer_id,
            cursor: Cursor::default(),
            scroll_offset: 0,
            cursor_cache: HashMap::new(),
            height: Cell::new(0),
        }
    }

    /// Save current cursor for old buffer, restore saved cursor for new buffer (or default).
    pub fn switch_buffer(&mut self, new_id: usize) {
        if let Some(old_id) = self.buffer_id {
            self.cursor_cache
                .insert(old_id, (self.cursor, self.scroll_offset));
        }
        if let Some((cursor, scroll)) = self.cursor_cache.get(&new_id) {
            self.cursor = *cursor;
            self.scroll_offset = *scroll;
        } else {
            self.cursor = Cursor::default();
            self.scroll_offset = 0;
        }
        self.buffer_id = Some(new_id);
    }
}

#[derive(Debug)]
pub enum LayoutNode {
    Leaf(usize), // pane_id
    Split {
        direction: SplitDirection,
        children: Vec<LayoutNode>,
    },
}

impl LayoutNode {
    /// Returns all pane ids in left-to-right / top-to-bottom order.
    pub fn pane_ids(&self) -> Vec<usize> {
        match self {
            LayoutNode::Leaf(id) => vec![*id],
            LayoutNode::Split { children, .. } => {
                let mut ids = Vec::new();
                for child in children {
                    ids.extend(child.pane_ids());
                }
                ids
            }
        }
    }

    /// Split a leaf. If the leaf's immediate parent is a Split with the same direction,
    /// the new pane is added as a sibling. Otherwise, the leaf is replaced with a new
    /// Split containing the original leaf and the new pane.
    pub fn split_leaf(&mut self, target_id: usize, direction: SplitDirection, new_pane_id: usize) -> bool {
        match self {
            LayoutNode::Leaf(id) if *id == target_id => {
                let old = std::mem::replace(self, LayoutNode::Leaf(0));
                *self = LayoutNode::Split {
                    direction,
                    children: vec![old, LayoutNode::Leaf(new_pane_id)],
                };
                true
            }
            LayoutNode::Split { direction: dir, children } => {
                // If same direction, add as sibling instead of nesting
                if *dir == direction {
                    if let Some(pos) = children.iter().position(|c| matches!(c, LayoutNode::Leaf(id) if *id == target_id)) {
                        children.insert(pos + 1, LayoutNode::Leaf(new_pane_id));
                        return true;
                    }
                }
                for child in children.iter_mut() {
                    if child.split_leaf(target_id, direction, new_pane_id) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }
}

/// Remove a pane from the tree and collapse if needed.
pub fn remove_from_tree(node: &mut LayoutNode, target_id: usize) -> bool {
    match node {
        LayoutNode::Leaf(_) => false,
        LayoutNode::Split { children, .. } => {
            if let Some(pos) = children.iter().position(|c| matches!(c, LayoutNode::Leaf(id) if *id == target_id)) {
                children.remove(pos);
                if children.len() == 1 {
                    let remaining = children.remove(0);
                    *node = remaining;
                }
                return true;
            }
            for child in children.iter_mut() {
                if remove_from_tree(child, target_id) {
                    return true;
                }
            }
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Pane movement — tree restructuring
// ---------------------------------------------------------------------------

/// Find (parent_direction, sibling_pane_id, parent_child_count) for a pane.
fn find_parent_info(node: &LayoutNode, pane_id: usize) -> Option<(SplitDirection, usize, usize)> {
    match node {
        LayoutNode::Leaf(_) => None,
        LayoutNode::Split { direction, children } => {
            if let Some(pane_idx) = children.iter().position(|c| matches!(c, LayoutNode::Leaf(id) if *id == pane_id)) {
                let sibling_idx = if pane_idx > 0 { pane_idx - 1 } else { 1.min(children.len() - 1) };
                let sibling_id = children[sibling_idx].pane_ids().into_iter().next().unwrap_or(0);
                return Some((*direction, sibling_id, children.len()));
            }
            for child in children {
                if let Some(result) = find_parent_info(child, pane_id) {
                    return Some(result);
                }
            }
            None
        }
    }
}

/// Find the path of child indices from root to a leaf.
fn find_leaf_path(node: &LayoutNode, pane_id: usize) -> Vec<usize> {
    let mut path = Vec::new();
    find_leaf_path_rec(node, pane_id, &mut path);
    path
}

fn find_leaf_path_rec(node: &LayoutNode, pane_id: usize, path: &mut Vec<usize>) -> bool {
    match node {
        LayoutNode::Leaf(id) => *id == pane_id,
        LayoutNode::Split { children, .. } => {
            for (i, child) in children.iter().enumerate() {
                path.push(i);
                if find_leaf_path_rec(child, pane_id, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
    }
}

/// Get the split direction of the node at the given path.
fn get_direction_at_path(root: &LayoutNode, path: &[usize]) -> Option<SplitDirection> {
    let mut current = root;
    for &idx in path {
        current = match current {
            LayoutNode::Split { children, .. } => match children.get(idx) {
                Some(c) => c,
                None => return None,
            },
            _ => return None,
        };
    }
    match current {
        LayoutNode::Split { direction, .. } => Some(*direction),
        _ => None,
    }
}

/// Replace root with Split(dir, [root, Leaf(pane)]) (or reversed).
fn wrap_root(root: &mut LayoutNode, pane_id: usize, dir: SplitDirection, at_end: bool) {
    let old = std::mem::replace(root, LayoutNode::Leaf(0));
    let children = if at_end {
        vec![old, LayoutNode::Leaf(pane_id)]
    } else {
        vec![LayoutNode::Leaf(pane_id), old]
    };
    *root = LayoutNode::Split { direction: dir, children };
}

/// Wrap the node at `path` in a new Split(dir) with a leaf for pane_id.
fn wrap_node_at_path(root: &mut LayoutNode, path: &[usize], pane_id: usize, dir: SplitDirection, at_end: bool) {
    if path.is_empty() {
        wrap_root(root, pane_id, dir, at_end);
        return;
    }
    // Navigate to the parent of the target node
    let mut current = &mut *root;
    for &idx in &path[..path.len() - 1] {
        current = match current {
            LayoutNode::Split { children, .. } => &mut children[idx],
            _ => return,
        };
    }
    let child_idx = path[path.len() - 1];
    if let LayoutNode::Split { children, .. } = current {
        if child_idx >= children.len() {
            return;
        }
        let child = std::mem::replace(&mut children[child_idx], LayoutNode::Leaf(0));
        let new_children = if at_end {
            vec![child, LayoutNode::Leaf(pane_id)]
        } else {
            vec![LayoutNode::Leaf(pane_id), child]
        };
        children[child_idx] = LayoutNode::Split {
            direction: dir,
            children: new_children,
        };
    }
}

/// Insert a leaf into the Split at `path` at the start or end.
fn insert_leaf_at_path(root: &mut LayoutNode, path: &[usize], pane_id: usize, at_end: bool) {
    let mut current = &mut *root;
    for &idx in path {
        current = match current {
            LayoutNode::Split { children, .. } => &mut children[idx],
            _ => return,
        };
    }
    if let LayoutNode::Split { children, .. } = current {
        if at_end {
            children.push(LayoutNode::Leaf(pane_id));
        } else {
            children.insert(0, LayoutNode::Leaf(pane_id));
        }
    }
}

/// Enforce tree invariants:
/// 1. No single-child splits (collapse to the child).
/// 2. No parent-child with the same split direction (merge children up).
fn normalize_tree(node: &mut LayoutNode) {
    // Recursively normalize children first
    if let LayoutNode::Split { children, .. } = node {
        for child in children.iter_mut() {
            normalize_tree(child);
        }
    }

    // Collapse single-child (or empty) splits
    if let LayoutNode::Split { children, .. } = node {
        if children.len() <= 1 {
            if let Some(child) = children.pop() {
                *node = child;
            }
            return;
        }
    }

    // Merge same-direction parent-child
    if let LayoutNode::Split { direction, children } = node {
        let dir = *direction;
        let mut merged = Vec::new();
        for child in std::mem::take(children) {
            match child {
                LayoutNode::Split { direction: d, children: gc } if d == dir => {
                    merged.extend(gc);
                }
                other => merged.push(other),
            }
        }
        *children = merged;
    }

    // After merging, might be single-child again
    if let LayoutNode::Split { children, .. } = node {
        if children.len() <= 1 {
            if let Some(child) = children.pop() {
                *node = child;
            }
        }
    }
}

/// Move a pane in the given direction by restructuring the layout tree.
///
/// - **Opposite axis** (e.g. move right from a Horizontal stack): wrap the old
///   parent remnant and the pane in a new split at the parent's level.
/// - **Same axis** (e.g. move down from a Horizontal stack): do the same
///   operation one level up (at grandparent). If at root, reorder within it.
///
/// After the operation, invariants are enforced (no single-child splits, no
/// same-direction parent-child).
pub fn move_pane_in_tree(root: &mut LayoutNode, pane_id: usize, direction: FocusDirection) -> bool {
    let target_dir = match direction {
        FocusDirection::Left | FocusDirection::Right => SplitDirection::Vertical,
        FocusDirection::Up | FocusDirection::Down => SplitDirection::Horizontal,
    };
    let at_end = matches!(direction, FocusDirection::Right | FocusDirection::Down);

    // Gather info before removal
    let (parent_dir, sibling_id, child_count) = match find_parent_info(root, pane_id) {
        Some(info) => info,
        None => return false,
    };
    let same_axis = parent_dir == target_dir;
    let will_collapse = child_count == 2;

    // Remove pane from tree (collapses single-child splits)
    if !remove_from_tree(root, pane_id) {
        return false;
    }

    // Find sibling in the modified tree to locate insertion point
    let sibling_path = find_leaf_path(root, sibling_id);

    // Path to the "old parent remnant" — what remains of the pane's original container
    let ref_path = if will_collapse {
        // Parent collapsed: sibling IS the remnant (took the parent's position)
        sibling_path
    } else if !sibling_path.is_empty() {
        // Parent survived: it's the sibling's parent
        sibling_path[..sibling_path.len() - 1].to_vec()
    } else {
        vec![]
    };

    if same_axis {
        // Same axis: operate one level above the old parent
        if ref_path.is_empty() {
            // Old parent remnant is root
            let root_has_target =
                matches!(root, LayoutNode::Split { direction, .. } if *direction == target_dir);
            if root_has_target {
                // Reorder: insert pane at start/end of root
                if let LayoutNode::Split { children, .. } = root {
                    if at_end {
                        children.push(LayoutNode::Leaf(pane_id));
                    } else {
                        children.insert(0, LayoutNode::Leaf(pane_id));
                    }
                }
            } else {
                wrap_root(root, pane_id, target_dir, at_end);
            }
        } else {
            // Go one level up from ref_path
            let parent_path = &ref_path[..ref_path.len() - 1];
            let parent_has_target = get_direction_at_path(root, parent_path) == Some(target_dir);
            if parent_has_target {
                insert_leaf_at_path(root, parent_path, pane_id, at_end);
            } else {
                wrap_node_at_path(root, parent_path, pane_id, target_dir, at_end);
            }
        }
    } else {
        // Opposite axis: wrap at the old parent level
        if ref_path.is_empty() {
            wrap_root(root, pane_id, target_dir, at_end);
        } else {
            wrap_node_at_path(root, &ref_path, pane_id, target_dir, at_end);
        }
    }

    normalize_tree(root);
    true
}

// ---------------------------------------------------------------------------
// PaneLayout
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct PaneLayout {
    pub panes: Vec<Pane>,
    pub root: LayoutNode,
    pub active_id: usize,
}

impl PaneLayout {
    pub fn new() -> Self {
        let pane = Pane::new(None);
        let id = pane.id;
        Self {
            panes: vec![pane],
            root: LayoutNode::Leaf(id),
            active_id: id,
        }
    }

    pub fn active_pane(&self) -> &Pane {
        self.panes
            .iter()
            .find(|p| p.id == self.active_id)
            .expect("active pane must exist")
    }

    pub fn active_pane_mut(&mut self) -> &mut Pane {
        self.panes
            .iter_mut()
            .find(|p| p.id == self.active_id)
            .expect("active pane must exist")
    }

    pub fn pane_by_id(&self, id: usize) -> Option<&Pane> {
        self.panes.iter().find(|p| p.id == id)
    }

    pub fn pane_by_id_mut(&mut self, id: usize) -> Option<&mut Pane> {
        self.panes.iter_mut().find(|p| p.id == id)
    }

    /// Split the active pane. Returns the new pane's id.
    pub fn split(&mut self, direction: SplitDirection) -> usize {
        let active = self.active_pane();
        let mut new_pane = Pane::new(active.buffer_id);
        new_pane.cursor = active.cursor;
        new_pane.scroll_offset = active.scroll_offset;
        let new_id = new_pane.id;

        self.root.split_leaf(self.active_id, direction, new_id);
        self.panes.push(new_pane);
        new_id
    }

    pub fn focus_next(&mut self) {
        let ids = self.root.pane_ids();
        if let Some(pos) = ids.iter().position(|&id| id == self.active_id) {
            let next = (pos + 1) % ids.len();
            self.active_id = ids[next];
        }
    }

    pub fn focus_prev(&mut self) {
        let ids = self.root.pane_ids();
        if let Some(pos) = ids.iter().position(|&id| id == self.active_id) {
            let prev = if pos == 0 { ids.len() - 1 } else { pos - 1 };
            self.active_id = ids[prev];
        }
    }

    pub fn close_active(&mut self) -> bool {
        if self.panes.len() <= 1 {
            return false;
        }
        let old_id = self.active_id;
        let ids = self.root.pane_ids();
        if let Some(pos) = ids.iter().position(|&id| id == old_id) {
            let next = if pos + 1 < ids.len() {
                ids[pos + 1]
            } else if pos > 0 {
                ids[pos - 1]
            } else {
                return false;
            };
            self.active_id = next;
        }
        remove_from_tree(&mut self.root, old_id);
        self.panes.retain(|p| p.id != old_id);
        true
    }

    pub fn is_single(&self) -> bool {
        self.panes.len() == 1
    }

    /// Focus the pane in the given direction relative to the active pane.
    pub fn focus_direction(&mut self, dir: FocusDirection) {
        let rects = self.root.compute_rects(0.0, 0.0, 1.0, 1.0);
        let active_rect = match rects.iter().find(|(id, _)| *id == self.active_id) {
            Some((_, r)) => *r,
            None => return,
        };
        let (ax, ay) = active_rect.center();

        let mut best: Option<(usize, f64)> = None;
        for &(id, ref r) in &rects {
            if id == self.active_id {
                continue;
            }
            if !active_rect.overlaps_for_direction(r, dir) {
                continue;
            }
            let (cx, cy) = r.center();
            let valid = match dir {
                FocusDirection::Left => cx < ax,
                FocusDirection::Right => cx > ax,
                FocusDirection::Up => cy < ay,
                FocusDirection::Down => cy > ay,
            };
            if !valid {
                continue;
            }
            let dist = (cx - ax).powi(2) + (cy - ay).powi(2);
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((id, dist));
            }
        }
        if let Some((id, _)) = best {
            self.active_id = id;
        }
    }

    /// Move the active pane in the given direction by restructuring the layout tree.
    pub fn move_direction(&mut self, dir: FocusDirection) -> bool {
        move_pane_in_tree(&mut self.root, self.active_id, dir)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Normalized rectangle (0.0..1.0 coordinate space).
#[derive(Debug, Clone, Copy)]
struct NormRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl NormRect {
    fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    fn overlaps_for_direction(&self, other: &NormRect, dir: FocusDirection) -> bool {
        match dir {
            FocusDirection::Left | FocusDirection::Right => {
                self.y < other.y + other.h && other.y < self.y + self.h
            }
            FocusDirection::Up | FocusDirection::Down => {
                self.x < other.x + other.w && other.x < self.x + self.w
            }
        }
    }
}

impl LayoutNode {
    /// Assign normalized rectangles to each leaf pane.
    fn compute_rects(&self, x: f64, y: f64, w: f64, h: f64) -> Vec<(usize, NormRect)> {
        match self {
            LayoutNode::Leaf(id) => vec![(*id, NormRect { x, y, w, h })],
            LayoutNode::Split { direction, children } => {
                let mut out = Vec::new();
                let n = children.len() as f64;
                match direction {
                    SplitDirection::Vertical => {
                        let each = w / n;
                        for (i, child) in children.iter().enumerate() {
                            out.extend(child.compute_rects(x + each * i as f64, y, each, h));
                        }
                    }
                    SplitDirection::Horizontal => {
                        let each = h / n;
                        for (i, child) in children.iter().enumerate() {
                            out.extend(child.compute_rects(x, y + each * i as f64, w, each));
                        }
                    }
                }
                out
            }
        }
    }
}
