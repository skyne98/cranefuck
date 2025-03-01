use std::collections::HashMap;

use crate::peephole::PeepholeIr;
use crate::tree::Tree;

pub type Variable = String;
pub type BlockIndex = usize;
#[derive(Debug, Clone)]
pub enum Terminator {
    Return,         // End of the function
    Jump(Variable), // Unconditional jump
    ConditionalJump {
        condition: Variable,
        true_branch: BlockIndex,
        false_branch: BlockIndex,
    },
}

pub struct Block {
    pub index: BlockIndex,
    pub terminator: Terminator,
}

pub struct SsaContext {
    next_variable_id: usize,
    next_block_id: usize,
    blocks: HashMap<BlockIndex, Block>,
    entry_block: BlockIndex,
}
impl SsaContext {
    pub fn new() -> Self {
        SsaContext {
            next_variable_id: 0,
            next_block_id: 0,
            blocks: HashMap::new(),
            entry_block: 0,
        }
    }
    pub fn clear(&mut self) {
        self.next_variable_id = 0;
    }
    pub fn build_from_tree(&mut self, tree: &Tree) {
        self.clear();

        let mut current_block = self.new_block();
        self.entry_block = current_block.index;
    }

    // Variables

    // Blocks
    pub fn new_block(&mut self) -> Block {
        Block {
            index: self.next_block(),
            terminator: Terminator::Return,
        }
    }

    // Utilities
    pub fn next_variable(&mut self, prefix: &str) -> Variable {
        let variable_id = self.next_variable_id;
        self.next_variable_id += 1;
        format!("{}_{}", prefix, variable_id)
    }
    pub fn next_block(&mut self) -> BlockIndex {
        let block_id = self.next_block_id;
        self.next_block_id += 1;
        block_id
    }
    pub fn print(&self) {
        // TODO: Print the entry block
    }
}
