use crate::parser::{Ir, IrLoopType};

#[derive(Debug, Clone)]
pub enum OptimizedIr {
    Ir(Ir),
    ResetToZero,
}
impl From<Ir> for OptimizedIr {
    fn from(ir: Ir) -> Self {
        OptimizedIr::Ir(ir)
    }
}

pub fn noop_optimzer(ir_ops: impl AsRef<[Ir]>) -> Vec<OptimizedIr> {
    ir_ops.as_ref().iter().map(|ir| ir.clone().into()).collect()
}
pub fn optimize(ir_ops: impl AsRef<[Ir]>) -> Vec<OptimizedIr> {
    let ir_ops = ir_ops.as_ref().to_vec();

    // Reset to zero pattern optimization
    let mut instruction_pointer = 0;
    let mut optimized_ops = vec![];
    let mut index_map = vec![0; ir_ops.len()];
    for i in 0..ir_ops.len() {
        index_map[i] = i;
    }
    while instruction_pointer < ir_ops.len() - 2 {
        let current_op = &ir_ops[instruction_pointer];
        let next_op = &ir_ops[instruction_pointer + 1];
        let next_next_op = &ir_ops[instruction_pointer + 2];

        if let Ir::Loop(IrLoopType::Start, _) = current_op
            && let Ir::Data(amount) = next_op
            && let Ir::Loop(IrLoopType::End, _) = next_next_op
            && (*amount == -1 || *amount == 1)
        {
            optimized_ops.push(OptimizedIr::ResetToZero);
            instruction_pointer += 3;

            // Shift all the indices on the right of the current instruction pointer by 2
            for i in instruction_pointer..ir_ops.len() {
                index_map[i] -= 2;
            }
        } else {
            optimized_ops.push(ir_ops[instruction_pointer].clone().into());
            instruction_pointer += 1;
        }
    }
    // Add the remaining instructions if any
    for i in instruction_pointer..ir_ops.len() {
        optimized_ops.push(ir_ops[i].clone().into());
    }
    // Update the loop pointers based on the new indices
    for i in 0..optimized_ops.len() {
        if let OptimizedIr::Ir(Ir::Loop(_, index)) = &mut optimized_ops[i] {
            *index = index_map[*index];
        }
    }

    optimized_ops
}
