use crate::builder::index::DockIndex;
use crate::builder::spec::{validate_tree, LayoutTree, PanelDef, SplitNode, TabsNode};
use crate::factory::Factory;
use crate::model::{Layout, NodeId, NodeKind};
use crate::{Error, Result};

/// Result of compiling a [`LayoutTree`].
#[derive(Debug, Clone)]
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

/// Insert a panel at runtime (shared by [`DockSession`](crate::builder::DockSession)).
pub(crate) fn insert_panel_runtime(
    factory: &Factory,
    layout: &mut Layout,
    index: &mut DockIndex,
    def: &PanelDef,
) -> Result<NodeId> {
    if index.panels.contains_key(&def.id) {
        return Err(Error::DuplicatePanelId(def.id.clone()));
    }
    insert_panel(factory, layout, index, def)
}

/// Resolve the first pane in preorder tree walk.
pub(crate) fn first_pane(layout: &Layout) -> Option<NodeId> {
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
pub(crate) fn owning_pane(layout: &Layout, panel: NodeId) -> Option<NodeId> {
    layout.get(panel).and_then(|e| e.owner)
}

/// Active panel id string, if any pane has an active tab.
pub(crate) fn active_panel_id(layout: &Layout, index: &DockIndex) -> Option<String> {
    for (id, node_id) in &index.panels {
        let owner = layout.get(*node_id)?.owner?;
        if let Some(NodeKind::Pane(pane)) = layout.kind(owner) {
            if pane.active == Some(*node_id) {
                return Some(id.clone());
            }
        }
    }
    None
}

/// Pane id for the pane whose active tab matches `panel_node`.
pub(crate) fn pane_for_active_panel(layout: &Layout, panel_node: NodeId) -> Option<NodeId> {
    owning_pane(layout, panel_node)
}
