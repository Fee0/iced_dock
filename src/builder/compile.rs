use crate::builder::index::DockIndex;
use crate::builder::spec::{validate_tree, LayoutTree, PanelDef, SplitNode, TabsNode};
use crate::factory::Factory;
use crate::model::{Layout, NodeId, NodeKind};
use crate::widget::DockWidgetState;
use crate::{Error, Result};

/// Result of compiling a [`LayoutTree`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BuiltLayout {
    pub layout: Layout,
    pub index: DockIndex,
}

/// Compile a declarative [`LayoutTree`] into a runtime [`Layout`] and index.
pub fn build_tree(tree: &LayoutTree) -> Result<BuiltLayout> {
    validate_tree(tree)?;
    let factory = Factory;
    let mut layout = Layout::new();
    let mut index = DockIndex::default();
    let root = compile_node(tree, &factory, &mut layout, &mut index)?;
    layout.set_root_child(Some(root));
    Ok(BuiltLayout { layout, index })
}

fn compile_node(
    tree: &LayoutTree,
    factory: &Factory,
    layout: &mut Layout,
    index: &mut DockIndex,
) -> Result<NodeId> {
    match tree {
        LayoutTree::Tabs(node) => compile_tabs(node, factory, layout, index),
        LayoutTree::Split(node) => compile_split(node, factory, layout, index),
    }
}

fn compile_tabs(
    node: &TabsNode,
    factory: &Factory,
    layout: &mut Layout,
    index: &mut DockIndex,
) -> Result<NodeId> {
    let pane_id = factory.create_pane(layout);
    if let Some(NodeKind::Pane(ref mut pane)) = layout.get_mut(pane_id).map(|e| &mut e.kind) {
        pane.name = node.name.clone();
    }
    if let Some(name) = &node.name {
        index.panes.insert(name.clone(), pane_id);
    }

    let mut panel_nodes = Vec::with_capacity(node.panels.len());
    for def in &node.panels {
        let panel_id = insert_panel(factory, layout, index, def)?;
        factory.add_panel_to_pane(layout, pane_id, panel_id)?;
        panel_nodes.push((def.id.clone(), panel_id));
    }

    if let Some(active) = &node.active {
        if let Some((_, panel_id)) = panel_nodes.iter().find(|(id, _)| id == active) {
            factory.set_active_panel(layout, pane_id, *panel_id);
        }
    }

    Ok(pane_id)
}

fn compile_split(
    node: &SplitNode,
    factory: &Factory,
    layout: &mut Layout,
    index: &mut DockIndex,
) -> Result<NodeId> {
    let mut children = Vec::with_capacity(node.children.len());
    for child in &node.children {
        children.push(compile_node(child, factory, layout, index)?);
    }
    let group_id = factory.create_proportional(layout, node.axis, children);
    if let Some(weights) = &node.weights {
        factory.set_proportions(layout, group_id, weights.clone())?;
    }
    Ok(group_id)
}

fn insert_panel(
    factory: &Factory,
    layout: &mut Layout,
    index: &mut DockIndex,
    def: &PanelDef,
) -> Result<NodeId> {
    let panel_id = factory.insert_panel(layout, def.id.clone(), def.title.clone(), def.content);
    if let Some(NodeKind::Panel(ref mut panel)) = layout.get_mut(panel_id).map(|e| &mut e.kind) {
        panel.can_close = def.can_close;
        panel.can_drag = def.can_drag;
        panel.can_drop = def.can_drop;
    }
    index.panels.insert(def.id.clone(), panel_id);
    Ok(panel_id)
}

/// Insert a panel using widget state (avoids overlapping field borrows).
pub(crate) fn insert_panel_into_state(
    factory: &Factory,
    state: &mut DockWidgetState,
    def: &PanelDef,
) -> Result<NodeId> {
    if state.index.panels.contains_key(&def.id) {
        return Err(Error::DuplicatePanelId(def.id.clone()));
    }
    insert_panel(factory, &mut state.layout, &mut state.index, def)
}

/// Resolve the first pane in preorder tree walk.
pub fn first_pane(layout: &Layout) -> Option<NodeId> {
    let root = layout.root_child()?;
    first_pane_walk(layout, root)
}

fn first_pane_walk(layout: &Layout, node: NodeId) -> Option<NodeId> {
    match layout.kind(node)? {
        NodeKind::Pane(_) => Some(node),
        NodeKind::Proportional(pg) => {
            for &child in &pg.children {
                if let Some(found) = first_pane_walk(layout, child) {
                    return Some(found);
                }
            }
            None
        }
        NodeKind::Panel(_) | NodeKind::Root(_) => None,
    }
}

/// Find the pane that owns a panel node.
pub fn owning_pane(layout: &Layout, panel: NodeId) -> Option<NodeId> {
    layout.get(panel).and_then(|e| e.owner)
}

/// Pane that owns a panel identified by string id.
pub fn pane_for_panel(layout: &Layout, index: &DockIndex, panel_id: &str) -> Option<NodeId> {
    let panel = index.panel_node(panel_id)?;
    owning_pane(layout, panel)
}

/// Active panel id string in a specific pane.
pub fn active_panel_in_pane(
    layout: &Layout,
    index: &DockIndex,
    pane: NodeId,
) -> Option<String> {
    let NodeKind::Pane(pane_state) = layout.kind(pane)? else {
        return None;
    };
    let active = pane_state
        .active
        .or_else(|| pane_state.tabs.first().copied())?;
    index
        .panels
        .iter()
        .find_map(|(id, node_id)| (*node_id == active).then(|| id.clone()))
}
