use cranelift::{codegen::ir::UserFuncName, prelude::*};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};
use std::{collections::HashMap, mem};

use crate::parser::{Ir, IrLoopType};

pub fn jit(ir_ops: impl AsRef<[Ir]>) {
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
    let mut module = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));

    let mut ctx = module.make_context();
    let mut func_ctx = FunctionBuilderContext::new();

    let mut func_sig = module.make_signature();
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

        // Data pointer variable
        let data_offset = Variable::new(0);
        builder.declare_var(data_offset, types::I64);
        let mut data_offset_var = builder.use_var(data_offset);

        let ir_ops = ir_ops.as_ref();

        // First pass to create the blocks
        let mut operation_to_block = HashMap::new();
        for (index, ir) in ir_ops.iter().enumerate() {
            match ir {
                Ir::Loop(_, _) => {
                    operation_to_block.insert(index, builder.create_block());
                }
                _ => {} // do nothing
            }
        }
        //...then create blocks for every loop start/end successor (next operation onwards)
        for (index, ir) in ir_ops.iter().enumerate() {
            if let Ir::Loop(_, _) = ir {
                let next_index = index + 1;
                if let None = operation_to_block.get(&next_index) {
                    operation_to_block.insert(next_index, builder.create_block());
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
                Ir::Data(amount) => {
                    let data_ptr = builder.ins().iadd(memory_ptr, data_offset_var);

                    // Increase the value at the memory pointer by the amount
                    let memory_value = builder.ins().load(types::I8, MemFlags::new(), data_ptr, 0);
                    let constant = builder.ins().iconst(types::I8, *amount as i64);
                    let (new_memory_value, _) = builder.ins().sadd_overflow(memory_value, constant);
                    builder
                        .ins()
                        .store(MemFlags::new(), new_memory_value, data_ptr, 0);
                }
                Ir::Move(amount) => {
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
                Ir::Loop(open, jump_index) => {
                    let successor_block = operation_to_block
                        .get(&(index + 1))
                        .expect("Successor block not found");
                    let jump_block = operation_to_block.get(jump_index).expect("Block not found");
                    let data_ptr = builder.ins().iadd(memory_ptr, data_offset_var);
                    let memory_value = builder.ins().load(types::I8, MemFlags::new(), data_ptr, 0);

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
                _ => {
                    // do nothing
                }
            }
        }

        let exit_block = builder.create_block();
        if !skip_next_jump {
            builder.ins().jump(exit_block, &[]);
        }
        builder.switch_to_block(exit_block);
        builder.ins().return_(&[]);
        builder.seal_all_blocks();
        builder.finalize();
    }

    println!("{}", ctx.func.display());
    module.define_function(main_func, &mut ctx).unwrap();
    module.clear_context(&mut ctx);

    // Perform linking.
    module.finalize_definitions().unwrap();

    // Get a raw pointer to the generated code.
    let code_b = module.get_finalized_function(main_func);

    // Cast it to a rust function pointer type.
    let ptr_b = unsafe { mem::transmute::<_, extern "C" fn(i64, i64)>(code_b) };

    let mut memory = [0u8; 10];
    let memory_ptr = { memory.as_mut_ptr() as *mut i64 };
    ptr_b(memory_ptr as i64, memory.len() as i64);
    println!("Result: {:?}", memory);
}
