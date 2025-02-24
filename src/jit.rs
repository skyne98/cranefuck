use cranelift::{codegen::ir::UserFuncName, prelude::*};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};
use std::{
    collections::{HashMap, VecDeque},
    io::Write,
    mem,
};

use crate::{
    optimizer::OptimizedIr,
    parser::{Ir, IrLoopType},
};

#[no_mangle]
pub extern "C" fn io_input(input_buffer: *const i64) -> u8 {
    let input_buffer =
        unsafe { std::mem::transmute::<*const i64, &mut VecDeque<char>>(input_buffer) };

    if input_buffer.len() == 0 {
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .expect("Failed to read line");
        line = line.replace("\r\n", "\n");
        input_buffer.extend(line.chars());
    }

    let character = input_buffer.pop_front().expect("No more input");

    if character == '\n' {
        10
    } else {
        character as u8
    }
}
#[no_mangle]
pub extern "C" fn io_input_noop(_: i64) -> u8 {
    0
}
#[no_mangle]
pub extern "C" fn io_output(value: u8) {
    let char = if value == 10 {
        if cfg!(windows) {
            "\r\n".to_string()
        } else {
            "\n".to_string()
        }
    } else {
        (value as char).to_string()
    };
    print!("{}", char);
    std::io::stdout().flush().expect("Failed to flush stdout");
}
#[no_mangle]
pub extern "C" fn io_output_noop(_: i8) {}

pub fn jit(ir_ops: impl AsRef<[OptimizedIr]>, ignore_io: bool) {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    // FIXME set back to true once the x64 backend supports it.
    flag_builder.set("is_pic", "false").unwrap();
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
        builder.declare_var(data_offset, types::I64);

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
                OptimizedIr::Ir(Ir::Loop(_, _)) => {
                    operation_to_block.insert(index, builder.create_block());
                }
                _ => {} // do nothing
            }
        }
        // Also create blocks for the successor of each loop instruction.
        // If index+1 is beyond the end, use exit_block.
        for (index, ir) in ir_ops.iter().enumerate() {
            if let OptimizedIr::Ir(Ir::Loop(_, _)) = ir {
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
                OptimizedIr::Ir(ir) => match ir {
                    Ir::Data(amount) => {
                        let data_offset_var = builder.use_var(data_offset);
                        let data_ptr = builder.ins().iadd(memory_ptr, data_offset_var);

                        // Increase the value at the memory pointer by the amount
                        let memory_value =
                            builder.ins().load(types::I8, MemFlags::new(), data_ptr, 0);
                        let constant = builder.ins().iconst(types::I8, *amount as i64);
                        let (new_memory_value, _) =
                            builder.ins().sadd_overflow(memory_value, constant);
                        builder
                            .ins()
                            .store(MemFlags::new(), new_memory_value, data_ptr, 0);
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
                    }
                    Ir::IO(true) => {
                        let data_offset_var = builder.use_var(data_offset);
                        let data_ptr = builder.ins().iadd(memory_ptr, data_offset_var);
                        let result = builder.ins().call(input_callee, &[input_buffer_ptr]);
                        let result = builder.inst_results(result)[0];
                        builder.ins().store(MemFlags::new(), result, data_ptr, 0);
                    }
                    Ir::IO(false) => {
                        let data_offset_var = builder.use_var(data_offset);
                        let data_ptr = builder.ins().iadd(memory_ptr, data_offset_var);
                        let memory_value =
                            builder.ins().load(types::I8, MemFlags::new(), data_ptr, 0);
                        builder.ins().call(output_callee, &[memory_value]);
                    }
                    Ir::Loop(open, jump_index) => {
                        let data_offset_var = builder.use_var(data_offset);
                        let successor_block = operation_to_block
                            .get(&(index + 1))
                            .expect("Successor block not found");
                        let jump_block =
                            operation_to_block.get(jump_index).expect("Block not found");
                        let data_ptr = builder.ins().iadd(memory_ptr, data_offset_var);
                        let memory_value =
                            builder.ins().load(types::I8, MemFlags::new(), data_ptr, 0);

                        match open {
                            IrLoopType::Start => {
                                let jump_condition =
                                    builder.ins().icmp_imm(IntCC::Equal, memory_value, 0);
                                builder.ins().brif(
                                    jump_condition,
                                    *jump_block,
                                    &[],
                                    *successor_block,
                                    &[],
                                );
                            }
                            IrLoopType::End => {
                                let jump_condition =
                                    builder.ins().icmp_imm(IntCC::NotEqual, memory_value, 0);
                                builder.ins().brif(
                                    jump_condition,
                                    *jump_block,
                                    &[],
                                    *successor_block,
                                    &[],
                                );
                            }
                        }
                        skip_next_jump = true;
                    }
                },
                OptimizedIr::ResetToZero => {
                    let data_offset_var = builder.use_var(data_offset);
                    let data_ptr = builder.ins().iadd(memory_ptr, data_offset_var);
                    let constant = builder.ins().iconst(types::I8, 0 as i64);
                    builder.ins().store(MemFlags::new(), constant, data_ptr, 0);
                }
                OptimizedIr::AddAndZero(target) => {
                    let source_offset_var = builder.use_var(data_offset);
                    let source_ptr = builder.ins().iadd(memory_ptr, source_offset_var);
                    let source_value =
                        builder
                            .ins()
                            .load(types::I8, MemFlags::new(), source_ptr, 0);

                    let target_offset_var =
                        builder.ins().iadd_imm(source_offset_var, *target as i64);
                    let target_ptr = builder.ins().iadd(memory_ptr, target_offset_var);
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
