use cranelift::{codegen::ir::UserFuncName, prelude::*};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};
use std::mem;

use crate::parser::Ir;

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
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();

        bcx.switch_to_block(block);
        bcx.append_block_params_for_function_params(block);

        let memory_ptr = bcx.block_params(block)[0];
        let memory_len = bcx.block_params(block)[1];

        // Data pointer variable
        let data_offset = Variable::new(0);
        bcx.declare_var(data_offset, types::I64);
        let mut data_offset_var = bcx.use_var(data_offset);

        let ir_ops = ir_ops.as_ref();
        for (index, ir) in ir_ops.iter().enumerate() {
            match ir {
                Ir::Data(amount) => {
                    let data_ptr = bcx.ins().iadd(memory_ptr, data_offset_var);

                    // Increase the value at the memory pointer by the amount
                    let memory_value = bcx.ins().load(types::I64, MemFlags::new(), data_ptr, 0);
                    let new_memory_value = bcx.ins().iadd_imm(memory_value, *amount as i64);
                    bcx.ins()
                        .store(MemFlags::new(), new_memory_value, data_ptr, 0);
                }
                Ir::Move(amount) => {
                    data_offset_var = bcx.ins().iadd_imm(data_offset_var, *amount as i64);
                    bcx.def_var(data_offset, data_offset_var);
                }
                _ => {
                    // do nothing
                }
            }
        }

        bcx.ins().return_(&[]);
        bcx.seal_all_blocks();
        bcx.finalize();
    }

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
