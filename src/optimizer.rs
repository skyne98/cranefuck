use crate::parser::{Ir, IrLoopType};

#[derive(Debug, Clone)]
pub enum OptimizedIr {
    Ir(Ir),
    ResetToZero,
    // [-N>+N<]
    AddAndZero(isize /* n */),
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
    let mut optimized_ops = ir_ops.into_iter().map(OptimizedIr::Ir).collect::<Vec<_>>();

    // Apply optimizations in passes
    optimized_ops = optimize_reset_to_zero(optimized_ops);
    optimized_ops = optimize_add_and_zero(optimized_ops);

    optimized_ops
}

// [-], [+] -> ResetToZero
fn optimize_reset_to_zero(ir_ops: Vec<OptimizedIr>) -> Vec<OptimizedIr> {
    let mut instruction_pointer = 0;
    let mut optimized_ops = Vec::with_capacity(ir_ops.len());
    let mut index_map = (0..ir_ops.len()).collect::<Vec<_>>();

    while instruction_pointer < ir_ops.len() {
        if let Some(
            [OptimizedIr::Ir(Ir::Loop(IrLoopType::Start, _)), OptimizedIr::Ir(Ir::Data(amount)), OptimizedIr::Ir(Ir::Loop(IrLoopType::End, _)), ..],
        ) = ir_ops.get(instruction_pointer..)
        {
            if *amount == -1 || *amount == 1 {
                optimized_ops.push(OptimizedIr::ResetToZero);
                instruction_pointer += 3;
                shift_indices(&mut index_map, instruction_pointer, -2);
                continue; // Skip the default push and increment
            }
        }
        optimized_ops.push(ir_ops[instruction_pointer].clone());
        instruction_pointer += 1;
    }

    update_loop_indices(optimized_ops, &index_map)
}
// [-N>+N<] -> AddAndZero
fn optimize_add_and_zero(ir_ops: Vec<OptimizedIr>) -> Vec<OptimizedIr> {
    let mut instruction_pointer = 0;
    let mut optimized_ops = Vec::with_capacity(ir_ops.len());
    let mut index_map = (0..ir_ops.len()).collect::<Vec<_>>();

    while instruction_pointer < ir_ops.len() {
        if let Some(
            [OptimizedIr::Ir(Ir::Loop(IrLoopType::Start, _)), OptimizedIr::Ir(Ir::Data(amount1)), OptimizedIr::Ir(Ir::Move(move_right_amount)), OptimizedIr::Ir(Ir::Data(amount2)), OptimizedIr::Ir(Ir::Move(move_left_amount)), OptimizedIr::Ir(Ir::Loop(IrLoopType::End, _)), ..],
        ) = ir_ops.get(instruction_pointer..)
        {
            if *amount1 == -1 && *amount2 == 1 && *move_right_amount == -*move_left_amount {
                optimized_ops.push(OptimizedIr::AddAndZero(*move_right_amount));
                instruction_pointer += 6;
                shift_indices(&mut index_map, instruction_pointer, -5);
                continue; // Skip the default push and increment
            }
        }
        optimized_ops.push(ir_ops[instruction_pointer].clone());
        instruction_pointer += 1;
    }

    update_loop_indices(optimized_ops, &index_map)
}

fn shift_indices(index_map: &mut Vec<usize>, start_index: usize, shift: isize) {
    for i in start_index..index_map.len() {
        index_map[i] = (index_map[i] as isize + shift) as usize;
    }
}

fn update_loop_indices(
    mut optimized_ops: Vec<OptimizedIr>,
    index_map: &[usize],
) -> Vec<OptimizedIr> {
    for op in &mut optimized_ops {
        if let OptimizedIr::Ir(Ir::Loop(_, index)) = op {
            *index = index_map[*index];
        }
    }
    optimized_ops
}
