use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};

use crate::parser::{Ir, IrLoopType};
use crate::peephole::PeepholeIr;
use crate::tree::{NodeKind, Tree};

pub type VariableIndex = usize;
pub type BlockIndex = usize;

// SSA Representation
#[derive(Debug, Clone)]
pub enum Terminator {
    Return,           // End of the function
    Continue,         // Continue to the next block
    Jump(BlockIndex), // Unconditional jump
    ConditionalJump {
        condition: VariableIndex,
        true_branch: BlockIndex,
        false_branch: BlockIndex,
    },
}

#[derive(Debug, Clone)]
pub enum InstructionOperation {
    Load(isize),                       // Load value at a ptr
    Set(isize, VariableIndex),         // Set value at a ptr
    Zero(VariableIndex),               // Zero out a variable
    AddConstant(VariableIndex, i64),   // Add a constant to a variable
    Add(VariableIndex, VariableIndex), // Add two variables
    Input,                             // Read a value from input
    Output(VariableIndex),             // Write a value to output
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub result: Option<VariableIndex>,
    pub operation: InstructionOperation,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub index: BlockIndex,
    pub alias: Option<String>,
    pub input_offsets: Vec<isize>,
    pub output_offsets: Vec<isize>,
    pub instructions: Vec<Instruction>,
    pub termination_offset: isize,
    pub terminator: Terminator,
}

impl Block {
    pub fn new(index: BlockIndex) -> Self {
        Block {
            index,
            alias: None,
            input_offsets: Vec::new(),
            output_offsets: Vec::new(),
            instructions: Vec::new(),
            termination_offset: 0,
            terminator: Terminator::Continue,
        }
    }
}

pub struct SsaContext {
    next_variable_id: RefCell<VariableIndex>,
    next_block_id: RefCell<BlockIndex>,
    pub blocks: RefCell<HashMap<BlockIndex, Block>>,
    pub entry_block: BlockIndex,
    pub exit_block: BlockIndex,
    variable_to_block: RefCell<HashMap<VariableIndex, BlockIndex>>,
    variable_to_alias: RefCell<HashMap<VariableIndex, String>>,
    block_predecessors: RefCell<HashMap<BlockIndex, Vec<BlockIndex>>>,
}

impl SsaContext {
    pub fn new() -> Self {
        SsaContext {
            next_variable_id: RefCell::new(0),
            next_block_id: RefCell::new(0),
            blocks: RefCell::new(HashMap::new()),
            entry_block: 0,
            exit_block: 0,
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
        self.exit_block = 0;
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
    pub fn create_block_aliased(&self, alias: String) -> BlockIndex {
        let block_id = self.next_block();
        let mut block = Block::new(block_id);
        block.alias = Some(alias.clone());
        self.blocks.borrow_mut().insert(block_id, block);
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
        let this_variable_id = *variable_id;
        *variable_id += 1;
        let alias = format!("{}_{}", prefix, this_variable_id);
        let mut variable_to_alias = self.variable_to_alias.borrow_mut();
        variable_to_alias.insert(this_variable_id, alias.clone());
        this_variable_id
    }
    pub fn add_variable_to_block(&self, variable: VariableIndex, block: BlockIndex) {
        if self.variable_to_block.borrow().contains_key(&variable) == false {
            self.variable_to_block.borrow_mut().insert(variable, block);
        }
    }
    pub fn get_variable_alias(&self, variable: VariableIndex) -> String {
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
    pub fn set_terminator(&self, block: BlockIndex, terminator: Terminator) {
        self.get_block_mut(block).terminator = terminator;
    }
    pub fn block_ids(&self) -> Vec<BlockIndex> {
        let mut block_ids = self.blocks.borrow().keys().cloned().collect::<Vec<_>>();
        block_ids.sort();
        block_ids
    }
    pub fn print(&self) {
        println!("SSA IR Representation");
        println!("====================");
        println!("Entry Block: {}", self.entry_block);

        // Get all blocks and sort them by index
        let blocks = self.blocks.borrow();
        let block_indices = self.block_ids();

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
            let alias_option = block.alias.as_ref();

            if !alias_option.is_none() {
                println!("  Alias: {}", alias_option.unwrap());
            }
            if !predecessors.is_empty() {
                println!("  Predecessors: {:?}", predecessors);
            }
            if !block.input_offsets.is_empty() {
                println!("  Input Offsets: {:?}", block.input_offsets);
            }
            if !block.output_offsets.is_empty() {
                println!("  Output Offsets: {:?}", block.output_offsets);
            }

            // Print instructions
            if !block.instructions.is_empty() {
                println!("  Instructions:");

                for instruction in &block.instructions {
                    let op_str = match &instruction.operation {
                        InstructionOperation::Load(offset) => {
                            format!("load({})", offset)
                        }
                        InstructionOperation::Set(offset, var) => {
                            let alias = self.get_variable_alias(*var);
                            format!("set({}, {})", offset, alias)
                        }
                        InstructionOperation::Zero(var) => {
                            let alias = self.get_variable_alias(*var);
                            format!("zero({})", alias)
                        }
                        InstructionOperation::AddConstant(var, constant) => {
                            let alias = self.get_variable_alias(*var);
                            format!("add({}, {})", alias, constant)
                        }
                        InstructionOperation::Add(var1, var2) => {
                            let alias1 = self.get_variable_alias(*var1);
                            let alias2 = self.get_variable_alias(*var2);
                            format!("add({}, {})", alias1, alias2)
                        }
                        InstructionOperation::Input => "input".to_string(),
                        InstructionOperation::Output(var) => {
                            let alias = self.get_variable_alias(*var);
                            format!("output({})", alias)
                        }
                    };

                    if let Some(result) = &instruction.result {
                        let alias = self.get_variable_alias(*result);
                        println!("    {} = {}", alias, op_str);
                    } else {
                        println!("    {}", op_str);
                    }
                }
            } else {
                println!("  No instructions");
            }

            // Print terminator
            println!("  Termination offset: {}", block.termination_offset);
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
                    let alias = self.get_variable_alias(*condition);
                    println!(
                        "  Terminator: if {} → Block {}, else → Block {}",
                        alias, true_branch, false_branch
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
        let mut blocks = vec![];
        let entry_block = self.create_block();
        self.entry_block = entry_block;
        blocks.push(entry_block);

        // Create the IR index to block map
        // then create the blocks
        // Loops contain a head block and a body block
        let mut ir_index_to_block = HashMap::new();
        let mut block_to_ir_indices = HashMap::new();
        let mut current_block = entry_block;
        for (ir_index, ir_op) in ir.iter().enumerate() {
            match ir_op {
                PeepholeIr::Ir(Ir::Loop(IrLoopType::Start, _)) => {
                    let head_alias = format!("head_{}", ir_index);
                    let head_block = self.create_block_aliased(head_alias);
                    let body_block = self.create_block();
                    ir_index_to_block.insert(ir_index, head_block);
                    block_to_ir_indices
                        .entry(head_block)
                        .or_insert_with(Vec::new)
                        .push(ir_index);
                    blocks.push(head_block);
                    blocks.push(body_block);
                    current_block = body_block;
                }
                PeepholeIr::Ir(Ir::Loop(IrLoopType::End, _)) => {
                    // Make a new block after the loop
                    let next_block = self.create_block();
                    ir_index_to_block.insert(ir_index, current_block);
                    ir_index_to_block.insert(ir_index + 1, next_block);
                    block_to_ir_indices
                        .entry(current_block)
                        .or_insert_with(Vec::new)
                        .push(ir_index);
                    block_to_ir_indices
                        .entry(next_block)
                        .or_insert_with(Vec::new)
                        .push(ir_index + 1);
                    blocks.push(next_block);
                    current_block = next_block;
                }
                _ => {
                    ir_index_to_block.insert(ir_index, current_block);
                    block_to_ir_indices
                        .entry(current_block)
                        .or_insert_with(Vec::new)
                        .push(ir_index);
                }
            }
        }

        // Add the exit block
        let exit_block = self.create_block();
        self.exit_block = exit_block;
        ir_index_to_block.insert(ir.len(), exit_block);
        self.set_terminator(exit_block, Terminator::Return);

        // Pretty-print the IR index to block map
        println!("IR Index to Block Map");
        println!("=====================");
        for (ir_index, block) in ir_index_to_block.iter() {
            if ir_index == &ir.len() {
                println!("IR Index {}: Exit Block", ir_index);
                continue;
            }
            let ir = &ir[*ir_index];
            println!("IR Index {}: {:?} -> Block {}", ir_index, ir, block);
        }

        // Gather a list of offsets which
        // will be read in this block
        let mut block_input_offsets = HashMap::new();
        for block_id in blocks.iter() {
            let mut current_offset = 0;
            let block_ir = &block_to_ir_indices[block_id];
            let mut block = self.get_block_mut(*block_id);
            // add the zero offset
            block_input_offsets
                .entry(*block_id)
                .or_insert_with(HashSet::new)
                .insert(current_offset);
            for ir_index in block_ir {
                let ir_op = &ir[*ir_index];
                match ir_op {
                    PeepholeIr::Ir(Ir::Move(offset)) => {
                        current_offset += offset;
                        println!("Block {}: Move {}", block_id, offset);
                        block_input_offsets
                            .entry(*block_id)
                            .or_insert_with(HashSet::new)
                            .insert(current_offset);
                    }
                    _ => (),
                }
            }

            println!("Block {}: Termination offset {}", block_id, current_offset);
            block.termination_offset = current_offset;
        }

        // Pretty-print the block input offsets
        println!();
        println!("Block Input Offsets");
        println!("==================");
        for (block_id, offsets) in block_input_offsets.iter() {
            println!("Block {}: {:?}", block_id, offsets);
        }

        // Add a load operation for each input offset
        // and record the latest variable for each offset
        // in each block
        let mut latest_var_per_cell_offset = HashMap::new();
        for block_id in blocks.iter() {
            let mut block = self.get_block_mut(*block_id);
            for input_offset in block_input_offsets[block_id].iter() {
                let var = self.next_variable(&format!("cell_{}", input_offset));
                self.add_variable_to_block(var, *block_id);
                block.instructions.push(Instruction {
                    result: Some(var),
                    operation: InstructionOperation::Load(*input_offset),
                });
                latest_var_per_cell_offset
                    .entry(*block_id)
                    .or_insert_with(HashMap::new)
                    .insert(*input_offset, var);
            }
        }

        // Process each IR instruction
        let mut modified_offsets_per_block = HashMap::new();
        // Convert the IR to SSA
        let mut current_block_relative_ptr: isize = 0;
        let mut current_block_id = entry_block;
        for (ir_index, ir_op) in ir.iter().enumerate() {
            let latest_var_per_cell_offset = latest_var_per_cell_offset
                .get_mut(&current_block_id)
                .expect("No hashmap with latest variables found for current block id");
            if ir_index_to_block.contains_key(&ir_index) == false {
                panic!("IR index {} not found in the block map", ir_index);
            }
            let block_id = ir_index_to_block[&ir_index];
            if current_block_id != block_id {
                current_block_id = block_id;
                current_block_relative_ptr = 0;
            }

            let mut block = self.get_block_mut(block_id);
            modified_offsets_per_block.insert(block_id, HashSet::new());

            match ir_op {
                PeepholeIr::Ir(Ir::Move(offset)) => {
                    current_block_relative_ptr += offset;
                }
                PeepholeIr::Ir(Ir::Data(value)) => {
                    // Look up if there was a previous version of this cell
                    let var = latest_var_per_cell_offset
                        .get_mut(&current_block_relative_ptr)
                        .expect("No variable found for current block relative ptr");
                    let new_var =
                        self.next_variable(&format!("cell_{}", current_block_relative_ptr));
                    self.add_variable_to_block(new_var, block_id);
                    block.instructions.push(Instruction {
                        result: Some(new_var),
                        operation: InstructionOperation::AddConstant(*var, *value),
                    });
                    *var = new_var;

                    // Mark offset as modified
                    modified_offsets_per_block
                        .entry(block_id)
                        .or_insert_with(HashSet::new)
                        .insert(current_block_relative_ptr);
                }
                PeepholeIr::ResetToZero => {
                    let var = latest_var_per_cell_offset
                        .entry(current_block_relative_ptr)
                        .or_insert_with(|| {
                            self.next_variable(&format!("cell_{}", current_block_relative_ptr))
                        });
                    self.add_variable_to_block(*var, block_id);
                    let new_var =
                        self.next_variable(&format!("cell_{}", current_block_relative_ptr));
                    self.add_variable_to_block(new_var, block_id);
                    block.instructions.push(Instruction {
                        result: Some(new_var),
                        operation: InstructionOperation::Zero(*var),
                    });

                    // Mark offset as modified
                    modified_offsets_per_block
                        .entry(block_id)
                        .or_insert_with(HashSet::new)
                        .insert(current_block_relative_ptr);
                }
                PeepholeIr::AddAndZero(offset) => {
                    // Handle source variable first and finish with it
                    let source_offset = current_block_relative_ptr;
                    let source_var_entry = latest_var_per_cell_offset
                        .entry(source_offset)
                        .or_insert_with(|| self.next_variable(&format!("cell_{}", source_offset)));
                    let source_var = *source_var_entry;
                    self.add_variable_to_block(source_var, block_id);
                    let source_var_new = self.next_variable(&format!("cell_{}", source_offset));
                    self.add_variable_to_block(source_var_new, block_id);

                    // Now handle target variable
                    let target_offset = current_block_relative_ptr + *offset;
                    let target_var_entry = latest_var_per_cell_offset
                        .entry(target_offset)
                        .or_insert_with(|| self.next_variable(&format!("cell_{}", target_offset)));
                    let target_var = *target_var_entry;
                    self.add_variable_to_block(target_var, block_id);
                    let target_var_new = self.next_variable(&format!("cell_{}", target_offset));
                    self.add_variable_to_block(target_var_new, block_id);

                    block.instructions.push(Instruction {
                        result: Some(target_var_new),
                        operation: InstructionOperation::Add(source_var, target_var),
                    });
                    block.instructions.push(Instruction {
                        result: Some(source_var_new),
                        operation: InstructionOperation::Zero(source_var),
                    });

                    // Update the latest variables
                    latest_var_per_cell_offset.insert(source_offset, source_var_new);
                    latest_var_per_cell_offset.insert(target_offset, target_var_new);

                    // Mark offsets as modified
                    modified_offsets_per_block
                        .entry(block_id)
                        .or_insert_with(HashSet::new)
                        .insert(source_offset);
                    modified_offsets_per_block
                        .entry(block_id)
                        .or_insert_with(HashSet::new)
                        .insert(target_offset);
                }
                PeepholeIr::Ir(Ir::IO(true)) => {
                    let var = self.next_variable("input");
                    self.add_variable_to_block(var, block_id);
                    block.instructions.push(Instruction {
                        result: Some(var),
                        operation: InstructionOperation::Input,
                    });

                    // Update latest variable
                    latest_var_per_cell_offset.insert(current_block_relative_ptr, var);

                    // Mark offset as modified
                    modified_offsets_per_block
                        .entry(block_id)
                        .or_insert_with(HashSet::new)
                        .insert(current_block_relative_ptr);
                }
                PeepholeIr::Ir(Ir::IO(false)) => {
                    let var = latest_var_per_cell_offset
                        .entry(current_block_relative_ptr)
                        .or_insert_with(|| {
                            self.next_variable(&format!("cell_{}", current_block_relative_ptr))
                        });
                    self.add_variable_to_block(*var, block_id);
                    block.instructions.push(Instruction {
                        result: None,
                        operation: InstructionOperation::Output(*var),
                    });
                }
                PeepholeIr::Ir(Ir::Loop(IrLoopType::Start, target_ir_index)) => {
                    let body_block = ir_index_to_block[&(ir_index + 1)];
                    let exit_block = ir_index_to_block[&(*target_ir_index + 1)];

                    // Load the current cell value
                    // or find existing cell value
                    let var = latest_var_per_cell_offset
                        .entry(current_block_relative_ptr)
                        .or_insert_with(|| {
                            self.next_variable(&format!("cell_{}", current_block_relative_ptr))
                        });
                    self.add_variable_to_block(*var, block_id);

                    block.terminator = Terminator::ConditionalJump {
                        condition: *var,
                        true_branch: body_block,
                        false_branch: exit_block,
                    };
                }
                PeepholeIr::Ir(Ir::Loop(IrLoopType::End, target_ir_index)) => {
                    let head_block = ir_index_to_block[target_ir_index];
                    block.terminator = Terminator::Jump(head_block);
                }
            }

            block.output_offsets = modified_offsets_per_block[&block_id]
                .iter()
                .cloned()
                .collect();
        }

        // Properly setup block predecessors
        let mut block_ids = blocks.iter().cloned().collect::<Vec<_>>();
        block_ids.sort();
        for block_id in block_ids {
            let block = self.get_block(block_id);
            match &block.terminator {
                Terminator::Continue => {
                    let next_block_id = block_id + 1;
                    self.add_predecessor(next_block_id, block_id);
                }
                Terminator::Jump(target_block_id) => {
                    self.add_predecessor(*target_block_id, block_id);
                }
                Terminator::ConditionalJump {
                    true_branch,
                    false_branch,
                    ..
                } => {
                    self.add_predecessor(*true_branch, block_id);
                    self.add_predecessor(*false_branch, block_id);
                }
                _ => (),
            }
        }

        println!("Block ids: {:?}", self.block_ids());
    }

    pub fn run(&mut self) {
        println!("Running SSA interpreter");
    }
}
