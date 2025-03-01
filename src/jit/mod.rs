use cranelift::{codegen::ir::UserFuncName, prelude::*};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};
use io::{io_input, io_input_noop, io_output, io_output_noop};
use std::{
    collections::{HashMap, VecDeque},
    mem,
};

pub mod io;

use crate::{
    parser::{Ir, IrLoopType},
    peephole::PeepholeIr,
};

pub fn jit(ir_ops: impl AsRef<[PeepholeIr]>, ignore_io: bool) {
    let mut flag_builder = settings::builder();
    flag_builder
        .set("use_colocated_libcalls", "false")
        .expect("Invalid flag value");
    flag_builder
        .set("opt_level", "speed")
        .expect("Invalid optimization level");
    flag_builder.set("is_pic", "true").unwrap();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
        panic!("host machine is not supported: {msg}");
    });
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap();
    let mut jit_builder = JITBuilder::with_isa(isa, default_libcall_names());
    jit_builder.symbol(
        "__io_output",
        if ignore_io {
            io_output_noop as *const u8
        } else {
            io_output as *const u8
        },
    );
    jit_builder.symbol(
        "__io_input",
        if ignore_io {
            io_input_noop as *const u8
        } else {
            io_input as *const u8
        },
    );
    let mut module = JITModule::new(jit_builder);

    // IO functions
    let mut io_output_sig = module.make_signature();
    io_output_sig.params.push(AbiParam::new(types::I8));
    let io_output_func = module
        .declare_function("__io_output", Linkage::Import, &io_output_sig)
        .unwrap();
    let mut io_input_sig = module.make_signature();
    io_input_sig.params.push(AbiParam::new(types::I64));
    io_input_sig.returns.push(AbiParam::new(types::I8));
    let io_input_func = module
        .declare_function("__io_input", Linkage::Import, &io_input_sig)
        .unwrap();

    let mut ctx = module.make_context();
    let mut func_ctx = FunctionBuilderContext::new();

    let mut func_sig = module.make_signature();
    func_sig.params.push(AbiParam::new(types::I64));
    func_sig.params.push(AbiParam::new(types::I64));
    func_sig.params.push(AbiParam::new(types::I64));

    let main_func = module
        .declare_function("main_func", Linkage::Local, &func_sig)
        .unwrap();

    ctx.func.signature = func_sig;
    ctx.func.name = UserFuncName::user(0, main_func.as_u32());

    {
        let mut builder: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry_block = builder.create_block();

        builder.switch_to_block(entry_block);
        builder.append_block_params_for_function_params(entry_block);

        let memory_ptr = builder.block_params(entry_block)[0];
        let memory_len = builder.block_params(entry_block)[1];
        let input_buffer_ptr = builder.block_params(entry_block)[2];

        // Data pointer variable
        let data_offset = Variable::new(0);
        let data_ptr = Variable::new(1);
        builder.declare_var(data_offset, types::I64);
        builder.declare_var(data_ptr, types::I64);
        builder.def_var(data_ptr, memory_ptr);

        // IO functions
        let input_callee = module.declare_func_in_func(io_input_func, &mut builder.func);
        let output_callee = module.declare_func_in_func(io_output_func, &mut builder.func);

        // Pre-create an exit block for use when index+1 is out of range.
        let exit_block = builder.create_block();

        let ir_ops = ir_ops.as_ref();
        // First pass to create the blocks
        let mut operation_to_block = HashMap::new();
        for (index, ir) in ir_ops.iter().enumerate() {
            match ir {
                PeepholeIr::Ir(Ir::Loop(_, _)) => {
                    operation_to_block.insert(index, builder.create_block());
                }
                _ => {} // do nothing
            }
        }
        // Also create blocks for the successor of each loop instruction.
        // If index+1 is beyond the end, use exit_block.
        for (index, ir) in ir_ops.iter().enumerate() {
            if let PeepholeIr::Ir(Ir::Loop(_, _)) = ir {
                let next_index = index + 1;
                if next_index < ir_ops.len() {
                    operation_to_block
                        .entry(next_index)
                        .or_insert_with(|| builder.create_block());
                } else {
                    operation_to_block.insert(next_index, exit_block);
                }
            }
        }

        // Second pass for compiling the operations
        let mut _current_block = entry_block;
        let mut current_block_index = -1;
        let mut skip_next_jump = false;
        for (index, ir) in ir_ops.iter().enumerate() {
            let index_block = operation_to_block.get(&index);
            if let Some(block) = index_block {
                if index as i32 != current_block_index {
                    if !skip_next_jump {
                        builder.ins().jump(*block, &[]);
                    } else {
                        skip_next_jump = false;
                    }
                    _current_block = *block;
                    current_block_index = index as i32;
                    builder.switch_to_block(_current_block);
                }
            }

            match ir {
                PeepholeIr::Ir(ir) => match ir {
                    Ir::Data(amount) => {
                        let data_ptr_val = builder.use_var(data_ptr);

                        // Increase the value at the memory pointer by the amount
                        let memory_value =
                            builder
                                .ins()
                                .load(types::I8, MemFlags::new(), data_ptr_val, 0);
                        let constant = builder.ins().iconst(types::I8, *amount as i64);
                        let new_memory_value = builder.ins().iadd(memory_value, constant);
                        builder
                            .ins()
                            .store(MemFlags::new(), new_memory_value, data_ptr_val, 0);
                    }
                    Ir::Move(amount) => {
                        let mut data_offset_var = builder.use_var(data_offset);
                        data_offset_var = builder.ins().iadd_imm(data_offset_var, *amount as i64);
                        let remainder = builder.ins().srem(data_offset_var, memory_len);
                        let less_than_zero =
                            builder.ins().icmp_imm(IntCC::SignedLessThan, remainder, 0);
                        let increased_data_offset = builder.ins().iadd(remainder, memory_len);
                        data_offset_var =
                            builder
                                .ins()
                                .select(less_than_zero, increased_data_offset, remainder);
                        builder.def_var(data_offset, data_offset_var);

                        // update the data_offset_ptr
                        let data_ptr_val = builder.ins().iadd(memory_ptr, data_offset_var);
                        builder.def_var(data_ptr, data_ptr_val);
                    }
                    Ir::IO(true) => {
                        let data_ptr = builder.use_var(data_ptr);
                        let result = builder.ins().call(input_callee, &[input_buffer_ptr]);
                        let result = builder.inst_results(result)[0];
                        builder.ins().store(MemFlags::new(), result, data_ptr, 0);
                    }
                    Ir::IO(false) => {
                        let data_ptr = builder.use_var(data_ptr);
                        let memory_value =
                            builder.ins().load(types::I8, MemFlags::new(), data_ptr, 0);
                        builder.ins().call(output_callee, &[memory_value]);
                    }
                    Ir::Loop(IrLoopType::Start, jump_index) => {
                        let jump_block = operation_to_block
                            .get(&(jump_index + 1))
                            .expect("Block not found");
                        let successor_block = operation_to_block
                            .get(&(index + 1))
                            .expect("Successor block not found");
                        let data_ptr = builder.use_var(data_ptr);
                        let memory_value =
                            builder.ins().load(types::I8, MemFlags::new(), data_ptr, 0);

                        let jump_condition = builder.ins().icmp_imm(IntCC::Equal, memory_value, 0);
                        builder
                            .ins()
                            .brif(jump_condition, *jump_block, &[], *successor_block, &[]);
                        skip_next_jump = true;
                    }
                    Ir::Loop(IrLoopType::End, jump_index) => {
                        let jump_block =
                            operation_to_block.get(jump_index).expect("Block not found");
                        builder.ins().jump(*jump_block, &[]);
                        skip_next_jump = true;
                    }
                },
                PeepholeIr::ResetToZero => {
                    let data_ptr = builder.use_var(data_ptr);
                    let constant = builder.ins().iconst(types::I8, 0 as i64);
                    builder.ins().store(MemFlags::new(), constant, data_ptr, 0);
                }
                PeepholeIr::AddAndZero(target) => {
                    let source_ptr = builder.use_var(data_ptr);
                    let source_value =
                        builder
                            .ins()
                            .load(types::I8, MemFlags::new(), source_ptr, 0);

                    let target_ptr = builder.ins().iadd_imm(source_ptr, *target as i64);
                    let target_value =
                        builder
                            .ins()
                            .load(types::I8, MemFlags::new(), target_ptr, 0);

                    let new_target_value = builder.ins().iadd(target_value, source_value);
                    builder
                        .ins()
                        .store(MemFlags::new(), new_target_value, target_ptr, 0);

                    let constant = builder.ins().iconst(types::I8, 0 as i64);
                    builder
                        .ins()
                        .store(MemFlags::new(), constant, source_ptr, 0);
                }
                _ => {
                    // do nothing
                }
            }
        }

        if !skip_next_jump {
            builder.ins().jump(exit_block, &[]);
        }
        builder.switch_to_block(exit_block);
        builder.ins().return_(&[]);
        builder.seal_all_blocks();
        builder.finalize();
    }

    module.define_function(main_func, &mut ctx).unwrap();
    module.clear_context(&mut ctx);

    // Perform linking.
    module
        .finalize_definitions()
        .expect("Failed to finalize definitions");

    // Get a raw pointer to the generated code.
    let code_b = module.get_finalized_function(main_func);

    // Cast it to a rust function pointer type.
    let ptr_b = unsafe { mem::transmute::<_, extern "C" fn(i64, i64, i64)>(code_b) };

    let mut memory = [0u8; 30000];
    let memory_ptr = { memory.as_mut_ptr() as *mut i64 };
    let mut input_buffer: VecDeque<char> = VecDeque::new();
    let input_buffer_ptr = (&mut input_buffer) as *mut _ as *mut i64;
    ptr_b(
        memory_ptr as i64,
        memory.len() as i64,
        input_buffer_ptr as i64,
    );
}
