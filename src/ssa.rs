use std::cell::RefCell;
use std::collections::HashMap;

use crate::parser::{Ir, IrLoopType};
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

    // Block
    pub fn next_block(&self) -> BlockIndex {
        let block_id = *self.next_block_id.borrow();
        *self.next_block_id.borrow_mut() += 1;
        block_id
    }
    pub fn create_block(&self) -> BlockIndex {
        let block_id = self.next_block();
        self.blocks
            .borrow_mut()
            .insert(block_id, Block::new(block_id));
        block_id
    }
    pub fn get_block(&self, block: BlockIndex) -> std::cell::Ref<'_, Block> {
        std::cell::Ref::map(self.blocks.borrow(), |blocks| blocks.get(&block).unwrap())
    }
    pub fn get_block_mut(&self, block: BlockIndex) -> std::cell::RefMut<'_, Block> {
        std::cell::RefMut::map(self.blocks.borrow_mut(), |blocks| {
            blocks.get_mut(&block).unwrap()
        })
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

    pub fn build_from_ir(&mut self, ir: impl AsRef<[PeepholeIr]>) {
        self.clear();
        let ir = ir.as_ref();

        // Create the entry block
        let entry_block = self.create_block();
        self.entry_block = entry_block;

        // Create the IR index to block map
        // then create the blocks
        // Loops contain a head block and a body block
        let mut ir_index_to_block = HashMap::new();
        let mut blocks = vec![];
        let mut current_block = entry_block;
        for (ir_index, ir_op) in ir.iter().enumerate() {
            match ir_op {
                PeepholeIr::Ir(Ir::Loop(IrLoopType::Start, _)) => {
                    let head_block = self.create_block();
                    let body_block = self.create_block();
                    ir_index_to_block.insert(ir_index, head_block);
                    ir_index_to_block.insert(ir_index + 1, body_block);
                    blocks.push(head_block);
                    blocks.push(body_block);
                    self.add_predecessor(head_block, current_block);
                    self.add_predecessor(body_block, head_block);
                    current_block = body_block;
                }
                PeepholeIr::Ir(Ir::Loop(IrLoopType::End, _)) => {
                    // Make a new block after the loop
                    let next_block = self.create_block();
                    ir_index_to_block.insert(ir_index + 1, next_block);
                    blocks.push(next_block);
                    self.add_predecessor(next_block, current_block);
                    current_block = next_block;
                }
                _ => {
                    ir_index_to_block.insert(ir_index, current_block);
                }
            }
        }

        // Pretty-print the IR index to block map
        println!("IR Index to Block Map");
        println!("=====================");
        for (ir_index, block) in ir_index_to_block {
            let ir = &ir[ir_index];
            println!("IR[{:?}] → Block {}", ir, block);
        }
    }
}
