/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use layout_api::wrapper_traits::ThreadSafeLayoutNode;
use rustc_hash::FxHashMap;
use script::layout_dom::ServoThreadSafeLayoutNode;
use serde::{Deserialize, Serialize};
use style::dom::OpaqueNode;

#[derive(Deserialize, Serialize, Debug)]
pub struct AccessibilityTree {
    nodes: FxHashMap<OpaqueNode, AccessibilityNode>,
    accesskit_tree: accesskit::Tree,
}

#[derive(Deserialize, Serialize, Debug)]
struct AccessibilityNode {
    id: accesskit::NodeId,
    accesskit_node: accesskit::Node,
}

struct AccessibilityUpdate {
    accesskit_update: accesskit::TreeUpdate,
}

impl Default for AccessibilityTree {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
            accesskit_tree: accesskit::Tree::new(accesskit::NodeId(0)),
        }
    }
}

impl Default for AccessibilityUpdate {
    fn default() -> Self {
        Self {
            accesskit_update: accesskit::TreeUpdate {
                nodes: Default::default(),
                tree: None,
                focus: accesskit::NodeId(1),
            },
        }
    }
}

impl AccessibilityUpdate {
    fn add(&mut self, node: &AccessibilityNode) {
        self.accesskit_update
            .nodes
            .push((node.id, node.accesskit_node.clone()));
    }
}

impl AccessibilityTree {
    pub(super) fn update_tree(
        &self,
        root_node: ServoThreadSafeLayoutNode<'_>,
    ) -> accesskit::TreeUpdate {
        let mut tree_update: AccessibilityUpdate = Default::default();
        self.update_node(root_node, &mut tree_update);

        tree_update.accesskit_update
    }

    fn update_node(
        &self,
        dom_node: ServoThreadSafeLayoutNode<'_>,
        tree_update: &mut AccessibilityUpdate,
    ) {
        let Some(accessibility_node) = self.nodes.get(&dom_node.opaque()) else {
            // FIXME: if we're passing in a DOM node, we should create the accessibility node if it doesn't exist!
            return;
        };
        // FIXME: this is silly since we may need to also add children
        // pass tree_update or tree_updates.nodes into update method?
        if accessibility_node.update(dom_node) {
            tree_update.add(accessibility_node);
        }

        // TODO: read accessibility damage from dom_node (right now, assume damage is complete)
    }
}

impl AccessibilityNode {
    fn update(&self, _dom_node: ServoThreadSafeLayoutNode<'_>) -> bool {
        true
    }
}

/*
Accessibility damage: needs to be in LayoutDamage since RestyleDamage is already fully subscribed
This seems to be available from node.owner_doc().ensure_pending_restyle(self). Ok.

*/

/*
Accessibility tree update:

- Traverse (flat) DOM tree
- for each node in traversal:
    - check layout damage to see whether there is accessiblity damage
    - if yes, retrieve accessibility node:
        - compute ax node id from node.opaque()
        - fetch from map
        - (it should already exist?)
    - run update based on value of accessibility damage
    - if damage requires computing children, traverse DOM node children and
      add stub accessibility nodes for each, so that the next iteration can
      find them

*/

/*
But what about actions :(

Martin says we can go the other way
Delan found node::from_untrusted_node_address, we can convert an OpaqueNode to an untrusted
address (using OpaqueNode.into()))
If the accessibility tree is _clean_, then we won't have any nodes in the accessibility tree
which don't correlate to live DOM nodes, i.e. if we lookup the accessibility node id we'll
get a miss if the DOM node is gone.

So: when we get an action request, before trying to perform the action we need to first do
a reflow and ensure the accessibility tree is clean, then lookup the accesskit node ID to
get the live accessibility node.
We don't want any script to run during this process, however this should be fine because the
action request will come in _on the script thread_ and will block any other script from
running.
*/
