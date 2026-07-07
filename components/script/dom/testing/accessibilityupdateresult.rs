/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
use dom_struct::dom_struct;
use script_bindings::reflector::{Reflector, reflect_dom_object};

use crate::dom::bindings::codegen::Bindings::ServoTestUtilsBinding::AccessibilityUpdateResultMethods;
use crate::dom::bindings::root::DomRoot;
use crate::dom::globalscope::GlobalScope;
use crate::script_runtime::CanGc;

#[dom_struct]
pub(crate) struct AccessibilityUpdateResult {
    reflector_: Reflector,
    accessibility_nodes_updated_from_dom: u32,
    accessibility_nodes_updated_from_tree: u32,
    accessibility_nodes_in_tree_update: u32,
}

impl AccessibilityUpdateResult {
    pub(crate) fn new_inherited(
        accessibility_nodes_updated_from_dom: u32,
        accessibility_nodes_updated_from_tree: u32,
        accessibility_nodes_in_tree_update: u32,
    ) -> Self {
        Self {
            reflector_: Reflector::new(),
            accessibility_nodes_updated_from_dom,
            accessibility_nodes_updated_from_tree,
            accessibility_nodes_in_tree_update,
        }
    }

    pub(crate) fn new(
        global: &GlobalScope,
        accessibility_nodes_updated_from_dom: u32,
        accessibility_nodes_updated_from_tree: u32,
        accessibility_nodes_in_tree_update: u32,
        can_gc: CanGc,
    ) -> DomRoot<Self> {
        reflect_dom_object(
            Box::new(Self::new_inherited(
                accessibility_nodes_updated_from_dom,
                accessibility_nodes_updated_from_tree,
                accessibility_nodes_in_tree_update,
            )),
            global,
            can_gc,
        )
    }
}

impl AccessibilityUpdateResultMethods<crate::DomTypeHolder> for AccessibilityUpdateResult {
    fn AccessibilityNodesUpdatedFromDom(&self) -> u32 {
        self.accessibility_nodes_updated_from_dom
    }

    fn AccessibilityNodesUpdatedFromTree(&self) -> u32 {
        self.accessibility_nodes_updated_from_tree
    }

    fn AccessibilityNodesInTreeUpdate(&self) -> u32 {
        self.accessibility_nodes_in_tree_update
    }
}
