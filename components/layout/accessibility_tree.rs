/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cell::RefCell;

use accesskit::Role;
use bitflags::bitflags;
use layout_api::wrapper_traits::{LayoutNode, ThreadSafeLayoutNode};
use log::trace;
use rustc_hash::{FxHashMap, FxHashSet};
use script::layout_dom::{ServoLayoutNode, ServoThreadSafeLayoutNode};
use style::dom::{NodeInfo, OpaqueNode};

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct AccessibilityDamage: u16 {
        const SELF = 0b0001;
        const CHILDREN = 0b0010;
        const SUBTREE = 0b0110;
        const SELF_AND_SUBTREE = 0b0111;
    }
}

impl Default for AccessibilityDamage {
    fn default() -> Self {
        Self::empty()
    }
}

struct AccessibilityUpdate {
    accesskit_update: accesskit::TreeUpdate,
}

#[derive(Debug)]
struct AccessibilityNode {
    id: accesskit::NodeId,
    accesskit_node: RefCell<accesskit::Node>,
    damage: AccessibilityDamage,
    opaque_node: Option<OpaqueNode>,
}

#[derive(Debug)]
pub struct AccessibilityTree {
    nodes: FxHashMap<accesskit::NodeId, AccessibilityNode>,
    accesskit_tree: accesskit::Tree,
    tree_id: accesskit::TreeId,
    new_nodes: RefCell<FxHashSet<accesskit::NodeId>>,
}

impl AccessibilityUpdate {
    fn new(tree: accesskit::Tree, tree_id: accesskit::TreeId) -> Self {
        Self {
            accesskit_update: accesskit::TreeUpdate {
                nodes: vec![],
                tree: Some(tree),
                focus: accesskit::NodeId(1),
                tree_id,
            },
        }
    }

    fn add(&mut self, node: &AccessibilityNode) {
        self.accesskit_update
            .nodes
            .push((node.id, node.accesskit_node.borrow().clone()));
    }
}

impl AccessibilityTree {
    const ROOT_NODE_ID: accesskit::NodeId = accesskit::NodeId(0);

    pub(super) fn new(tree_id: accesskit::TreeId) -> Self {
        // The root node doesn't correspond to a DOM node, but contains the root DOM node.
        let root_node = AccessibilityNode::new(AccessibilityTree::ROOT_NODE_ID);
        root_node
            .accesskit_node
            .borrow_mut()
            .set_role(accesskit::Role::RootWebArea);
        root_node
            .accesskit_node
            .borrow_mut()
            .add_action(accesskit::Action::Focus);

        let mut tree = Self {
            nodes: FxHashMap::default(),
            accesskit_tree: accesskit::Tree::new(root_node.id),
            tree_id,
            new_nodes: RefCell::default(),
        };
        tree.nodes.insert(root_node.id, root_node);

        tree
    }

    pub(super) fn update_tree(
        &mut self,
        document: ServoThreadSafeLayoutNode<'_>,
    ) -> Option<accesskit::TreeUpdate> {
        let mut tree_update = AccessibilityUpdate::new(self.accesskit_tree.clone(), self.tree_id);
        let root_node = self
            .nodes
            .get_mut(&AccessibilityTree::ROOT_NODE_ID)
            .unwrap();
        root_node.opaque_node = Some(document.opaque());

        // let root_dom_node_id = Self::to_accesskit_id(&root_dom_node.opaque());
        // root_node
        //     .accesskit_node
        //     .set_children(vec![root_dom_node_id]);

        // tree_update.add(root_node);

        self.update_node_and_children(AccessibilityTree::ROOT_NODE_ID, &mut tree_update);
        self.new_nodes.borrow_mut().clear();
        Some(tree_update.accesskit_update)
    }

    fn update_node_and_children(
        &mut self,
        node_id: accesskit::NodeId,
        // dom_node: ServoThreadSafeLayoutNode<'_>,
        tree_update: &mut AccessibilityUpdate,
    ) {
        let Some(node) = self.nodes.get_mut(&node_id) else {
            return;
        };

        if self.update_children(node) {
            tree_update.add(&node);
        }

        tree_update.add(&node);

        for child_id in node.accesskit_node.borrow().children() {
            self.update_node_and_children(*child_id, tree_update);
        }
    }

    fn update_children(&mut self, node: &mut AccessibilityNode) -> bool {
        if !node.damage.contains(AccessibilityDamage::CHILDREN) {
            return false;
        }
        let accesskit_node = &node.accesskit_node;
        let Some(dom_node) = node.get_dom_node() else {
            return true;
        };
        let mut new_children: Vec<accesskit::NodeId> = vec![];
        for dom_child in dom_node.children() {
            let child_id = Self::to_accesskit_id(&dom_child.opaque());
            new_children.push(child_id);
            self.get_or_create_node_mut(dom_child).opaque_node = Some(dom_child.opaque());
        }
        if new_children == accesskit_node.borrow().children() {
            return false;
        }
        for old_child_id in accesskit_node.borrow().children() {
            if !new_children.contains(old_child_id) &&
                self.new_nodes.borrow().contains(old_child_id)
            {
                // remove stale nodes
                self.remove_subtree(*old_child_id);
            }
        }
        for new_child_id in new_children {
            self.new_nodes.borrow_mut().insert(new_child_id);
        }
        true
    }

    fn update_node(&self, node: &mut AccessibilityNode) -> bool {
        if !node.damage.contains(AccessibilityDamage::SELF) {
            return false;
        }
        let Some(dom_node) = node.get_dom_node() else {
            return true;
        };

        if dom_node.is_text_node() {
            node.accesskit_node.borrow_mut().set_role(Role::TextRun);
            let text_content = dom_node.text_content();
            trace!("node text content = {text_content:?}");
            // FIXME: this should take into account editing selection units (grapheme clusters?)
            node.accesskit_node.borrow_mut().set_value(&*text_content);
        } else if dom_node.as_element().is_some() {
            node.accesskit_node
                .borrow_mut()
                .set_role(Role::GenericContainer);
        }

        true
    }

    fn get_or_create_node_mut(
        &mut self,
        dom_node: ServoThreadSafeLayoutNode<'_>,
    ) -> &mut AccessibilityNode {
        let id = Self::to_accesskit_id(&dom_node.opaque());

        self.nodes
            .entry(id)
            .or_insert_with(|| AccessibilityNode::new(id))
    }

    fn remove_subtree(&mut self, node_id: accesskit::NodeId) {
        let Some(node) = self.nodes.get(&node_id) else {
            return;
        };
        let children: Vec<accesskit::NodeId> = node.accesskit_node.borrow().children().to_vec();
        self.nodes.remove(&node_id);
        for child_id in children {
            self.remove_subtree(child_id);
        }
    }

    fn to_accesskit_id(opaque: &OpaqueNode) -> accesskit::NodeId {
        accesskit::NodeId(opaque.0 as u64)
    }
}

impl AccessibilityNode {
    fn new(id: accesskit::NodeId) -> Self {
        Self {
            id,
            accesskit_node: RefCell::new(accesskit::Node::new(Role::Unknown)),
            damage: AccessibilityDamage::SELF_AND_SUBTREE,
            opaque_node: None,
        }
    }

    #[expect(unsafe_code)]
    /// Safety: this must only be called on accessibility nodes which are known non-stale.
    fn get_dom_node(&self) -> Option<ServoThreadSafeLayoutNode<'_>> {
        let opaque_node = self.opaque_node?;
        let dom_node = unsafe {
            let servo_layout_node = ServoLayoutNode::from_untrusted(&opaque_node.into());
            servo_layout_node.to_threadsafe()
        };
        Some(dom_node)
    }
}
