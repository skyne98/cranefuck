use std::cell::RefCell;
use std::collections::HashMap;

use crate::parser::Ir;
use crate::peephole::PeepholeIr;
use crate::tree::{NodeKind, Tree};

pub type Variable = String;
pub type VariableIndex = usize;
pub type BlockIndex = usize;

// SSA Representation
#[derive(Debug, Clone)]
pub enum Terminator {
    Return,           // End of the function
    Continue,         // Continue to the next block
    Jump(BlockIndex), // Unconditional jump
    ConditionalJump {
        condition: Variable,
        true_branch: BlockIndex,
        false_branch: BlockIndex,
    },
}

#[derive(Debug, Clone)]
pub enum InstructionOperation {
    Phi(Vec<(Variable, BlockIndex)>),
    Zero(Variable),
    AddAndZero(isize),
    Data(Variable, i64),
    Move(Variable, isize),
    Input,
    Output(Variable),
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub result: Variable,
    pub operation: InstructionOperation,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub index: BlockIndex,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

impl Block {
    pub fn new(index: BlockIndex) -> Self {
        Block {
            index,
            instructions: Vec::new(),
            terminator: Terminator::Return,
        }
    }
}

pub struct SsaContext {
    next_variable_id: RefCell<VariableIndex>,
    next_block_id: RefCell<BlockIndex>,
    pub blocks: RefCell<HashMap<BlockIndex, Block>>,
    pub entry_block: BlockIndex,
    variable_to_block: RefCell<HashMap<VariableIndex, BlockIndex>>,
    variable_to_alias: RefCell<HashMap<VariableIndex, Variable>>,
    block_predecessors: RefCell<HashMap<BlockIndex, Vec<BlockIndex>>>,
}

impl SsaContext {
    pub fn new() -> Self {
        SsaContext {
            next_variable_id: RefCell::new(0),
            next_block_id: RefCell::new(0),
            blocks: RefCell::new(HashMap::new()),
            entry_block: 0,
            variable_to_block: RefCell::new(HashMap::new()),
            variable_to_alias: RefCell::new(HashMap::new()),
            block_predecessors: RefCell::new(HashMap::new()),
        }
    }
    pub fn clear(&mut self) {
        self.next_variable_id = RefCell::new(0);
        self.next_block_id = RefCell::new(0);
        self.blocks = RefCell::new(HashMap::new());
        self.entry_block = 0;
        self.variable_to_block = RefCell::new(HashMap::new());
        self.variable_to_alias = RefCell::new(HashMap::new());
        self.block_predecessors = RefCell::new(HashMap::new());
    }

    // Utilities
    pub fn next_variable(&self, prefix: &str) -> VariableIndex {
        let mut variable_id = self.next_variable_id.borrow_mut();
        *variable_id += 1;
        let alias = format!("{}{}", prefix, variable_id);
        let mut variable_to_alias = self.variable_to_alias.borrow_mut();
        variable_to_alias.insert(*variable_id, alias.clone());
        *variable_id
    }
    pub fn next_block(&mut self) -> BlockIndex {
        let block_id = *self.next_block_id.borrow();
        *self.next_block_id.borrow_mut() += 1;
        block_id
    }
    pub fn add_variable_to_block(&self, variable: VariableIndex, block: BlockIndex) {
        self.variable_to_block.borrow_mut().insert(variable, block);
    }
    pub fn get_variable_alias(&self, variable: VariableIndex) -> Variable {
        self.variable_to_alias
            .borrow()
            .get(&variable)
            .unwrap()
            .clone()
    }
    pub fn get_block(&self, block: BlockIndex) -> Block {
        self.blocks.borrow().get(&block).unwrap().clone()
    }
    pub fn add_predecessor(&self, block: BlockIndex, predecessor: BlockIndex) {
        self.block_predecessors
            .borrow_mut()
            .entry(block)
            .or_insert_with(Vec::new)
            .push(predecessor);
    }
    pub fn get_predecessors(&self, block: BlockIndex) -> Vec<BlockIndex> {
        self.block_predecessors
            .borrow()
            .get(&block)
            .unwrap()
            .clone()
    }
    pub fn print(&self) {
        println!("SSA IR Representation");
        println!("====================");
        println!("Entry Block: {}", self.entry_block);
        println!();

        // Get all blocks and sort them by index
        let blocks = self.blocks.borrow();
        let mut block_indices: Vec<BlockIndex> = blocks.keys().cloned().collect();
        block_indices.sort();

        for block_index in block_indices {
            let block = blocks.get(&block_index).unwrap();

            // Print block header with entry indicator
            print!("Block {}", block_index);
            if block_index == self.entry_block {
                println!(" (ENTRY)");
            } else {
                println!();
            }

            // Print predecessors
            let predecessors = self
                .block_predecessors
                .borrow()
                .get(&block_index)
                .cloned()
                .unwrap_or_default();

            if !predecessors.is_empty() {
                println!("  Predecessors: {:?}", predecessors);
            }

            // Print instructions
            if !block.instructions.is_empty() {
                println!("  Instructions:");

                for instruction in &block.instructions {
                    let op_str = match &instruction.operation {
                        InstructionOperation::Phi(sources) => {
                            let sources_str: Vec<String> = sources
                                .iter()
                                .map(|(var, block)| format!("({} from {})", var, block))
                                .collect();
                            format!("φ [{}]", sources_str.join(", "))
                        }
                        InstructionOperation::Zero(var) => format!("zero({})", var),
                        InstructionOperation::AddAndZero(amount) => {
                            format!("add_and_zero({})", amount)
                        }
                        InstructionOperation::Data(var, value) => {
                            format!("data({}, {})", var, value)
                        }
                        InstructionOperation::Move(var, offset) => {
                            format!("move({}, {})", var, offset)
                        }
                        InstructionOperation::Input => "input()".to_string(),
                        InstructionOperation::Output(var) => format!("output({})", var),
                    };

                    println!("    {} = {}", instruction.result, op_str);
                }
            } else {
                println!("  No instructions");
            }

            // Print terminator
            match &block.terminator {
                Terminator::Return => println!("  Terminator: return"),
                Terminator::Continue => println!("  Terminator: continue"),
                Terminator::Jump(target) => {
                    println!("  Terminator: jump → Block {}", target);
                }
                Terminator::ConditionalJump {
                    condition,
                    true_branch,
                    false_branch,
                } => {
                    println!(
                        "  Terminator: if {} → Block {}, else → Block {}",
                        condition, true_branch, false_branch
                    );
                }
            }

            println!("----------------------------------------");
        }
    }

    pub fn build_from_tree(&mut self, tree: &Tree) {
        self.clear();

        let mut sequence_id_stack = vec![tree.root()];
        let mut sequence_id_to_block = HashMap::new();
        let mut latest_sequence_id = tree.root();

        while let Some(sequence_id) = sequence_id_stack.pop() {}
    }
}
