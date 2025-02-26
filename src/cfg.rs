use std::collections::HashMap;

use crate::tree::TreeId;

pub struct CfgContext {
    pub outputs: HashMap<TreeId, Vec<TreeId>>,
}
impl CfgContext {
    pub fn new() -> Self {
        CfgContext {
            outputs: HashMap::new(),
        }
    }
    pub fn get_outputs(&self, id: TreeId) -> Option<&Vec<TreeId>> {
        self.outputs.get(&id)
    }
}
