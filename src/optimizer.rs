use crate::parser::{Ir, IrLoopType};

#[derive(Debug, Clone)]
pub enum OptimizedIr {
    Ir(Ir),
    ResetToZero,
    // [-N>+N<]
    AddAndZero(isize /* n */),
    // [X-N>Y+N<]
    ScaledAddAndZero(isize /* n */, i64 /* x */, i64 /* y */), // => (initial_value / X) * Y
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
    // ==================================
    // [-] or [+] pattern optimization
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

    // Copy pattern [->+<] optimization
    // ================================
    // also applies to any [-N>+N<] pattern
    let mut instruction_pointer = 0;
    let previous_optimized_ops = optimized_ops.clone();
    let mut optimized_ops = vec![];
    let mut index_map = vec![0; previous_optimized_ops.len()];
    for i in 0..previous_optimized_ops.len() {
        index_map[i] = i;
    }
    while instruction_pointer < previous_optimized_ops.len() {
        let current_op = &previous_optimized_ops[instruction_pointer];
        // [
        if let OptimizedIr::Ir(Ir::Loop(IrLoopType::Start, _)) = current_op
            // -
            && let Some(first_op) = previous_optimized_ops.get(instruction_pointer + 1)
            && let OptimizedIr::Ir(Ir::Data(amount)) = first_op
            && *amount == -1
            // >
            && let Some(second_op) = previous_optimized_ops.get(instruction_pointer + 2)
            && let OptimizedIr::Ir(Ir::Move(move_right_amount)) = second_op
            // +
            && let Some(third_op) = previous_optimized_ops.get(instruction_pointer + 3)
            && let OptimizedIr::Ir(Ir::Data(amount)) = third_op
            && *amount == 1
            // <
            && let Some(fourth_op) = previous_optimized_ops.get(instruction_pointer + 4)
            && let OptimizedIr::Ir(Ir::Move(move_left_amount)) = fourth_op
            // ]
            && let Some(fifth_op) = previous_optimized_ops.get(instruction_pointer + 5)
            && let OptimizedIr::Ir(Ir::Loop(IrLoopType::End, _)) = fifth_op
            && *move_right_amount == -*move_left_amount
        {
            optimized_ops.push(OptimizedIr::AddAndZero(*move_right_amount));
            instruction_pointer += 6;

            // Shift all the indices on the right of the current instruction pointer by 5
            for i in instruction_pointer..previous_optimized_ops.len() {
                index_map[i] -= 5;
            }
        } else {
            optimized_ops.push(previous_optimized_ops[instruction_pointer].clone());
            instruction_pointer += 1;
        }
    }
    // Update the loop pointers based on the new indices
    for i in 0..optimized_ops.len() {
        if let OptimizedIr::Ir(Ir::Loop(_, index)) = &mut optimized_ops[i] {
            *index = index_map[*index];
        }
    }

    // Scaled copy pattern [X-N>+Y+N<] optimization
    // ===========================================
    let mut instruction_pointer = 0;
    let previous_optimized_ops = optimized_ops.clone();
    let mut optimized_ops = vec![];
    let mut index_map = vec![0; previous_optimized_ops.len()];
    for i in 0..previous_optimized_ops.len() {
        index_map[i] = i;
    }
    while instruction_pointer < previous_optimized_ops.len() {
        let current_op = &previous_optimized_ops[instruction_pointer];
        // [
        if let OptimizedIr::Ir(Ir::Loop(IrLoopType::Start, _)) = current_op
            // X
            && let Some(first_op) = previous_optimized_ops.get(instruction_pointer + 1)
            && let OptimizedIr::Ir(Ir::Data(x)) = first_op
            // >
            && let Some(second_op) = previous_optimized_ops.get(instruction_pointer + 2)
            && let OptimizedIr::Ir(Ir::Move(move_right_amount)) = second_op
            // Y
            && let Some(third_op) = previous_optimized_ops.get(instruction_pointer + 3)
            && let OptimizedIr::Ir(Ir::Data(y)) = third_op
            // <
            && let Some(fourth_op) = previous_optimized_ops.get(instruction_pointer + 4)
            && let OptimizedIr::Ir(Ir::Move(move_left_amount)) = fourth_op
            // ]
            && let Some(fifth_op) = previous_optimized_ops.get(instruction_pointer + 5)
            && let OptimizedIr::Ir(Ir::Loop(IrLoopType::End, _)) = fifth_op
            && *move_right_amount == -*move_left_amount
        {
            optimized_ops.push(OptimizedIr::ScaledAddAndZero(*move_right_amount, *x, *y));
            instruction_pointer += 6;

            // Shift all the indices on the right of the current instruction pointer by 5
            for i in instruction_pointer..previous_optimized_ops.len() {
                index_map[i] -= 5;
            }
        } else {
            optimized_ops.push(previous_optimized_ops[instruction_pointer].clone());
            instruction_pointer += 1;
        }
    }
    // Update the loop pointers based on the new indices
    for i in 0..optimized_ops.len() {
        if let OptimizedIr::Ir(Ir::Loop(_, index)) = &mut optimized_ops[i] {
            *index = index_map[*index];
        }
    }

    optimized_ops
}
