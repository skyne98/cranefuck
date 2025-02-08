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
        let mut data_offset = bcx.ins().iconst(types::I64, 0);

        let ir_ops = ir_ops.as_ref();
        for (index, ir) in ir_ops.iter().enumerate() {
            match ir {
                Ir::Data(amount) => {
                    let data_ptr = bcx.ins().iadd(memory_ptr, data_offset);

                    // Increase the value at the memory pointer by the amount
                    let memory_value = bcx.ins().load(types::I64, MemFlags::new(), data_ptr, 0);
                    let new_memory_value = bcx.ins().iadd_imm(memory_value, *amount as i64);
                    bcx.ins()
                        .store(MemFlags::new(), new_memory_value, data_ptr, 0);
                }
                Ir::Move(amount) => {
                    data_offset = bcx.ins().iadd(data_offset, *amount as i64);
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

    let mut memory = [0; 10];
    let memory_ptr = { memory.as_mut_ptr() as *mut i64 };
    ptr_b(memory_ptr as i64, memory.len() as i64);
    println!("Result: {:?}", memory);
}

pub fn jit_demo(ir_ops: impl AsRef<[Ir]>) {
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

    let mut sig_a = module.make_signature();
    sig_a.params.push(AbiParam::new(types::I32));
    sig_a.returns.push(AbiParam::new(types::I32));

    let mut sig_b = module.make_signature();
    sig_b.returns.push(AbiParam::new(types::I32));

    let func_a = module
        .declare_function("a", Linkage::Local, &sig_a)
        .unwrap();
    let func_b = module
        .declare_function("b", Linkage::Local, &sig_b)
        .unwrap();

    ctx.func.signature = sig_a;
    ctx.func.name = UserFuncName::user(0, func_a.as_u32());

    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();

        bcx.switch_to_block(block);
        bcx.append_block_params_for_function_params(block);
        let param = bcx.block_params(block)[0];
        let cst = bcx.ins().iconst(types::I32, 37);
        let add = bcx.ins().iadd(cst, param);
        bcx.ins().return_(&[add]);
        bcx.seal_all_blocks();
        bcx.finalize();
    }
    module.define_function(func_a, &mut ctx).unwrap();
    module.clear_context(&mut ctx);

    ctx.func.signature = sig_b;
    ctx.func.name = UserFuncName::user(0, func_b.as_u32());

    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();

        bcx.switch_to_block(block);
        let local_func = module.declare_func_in_func(func_a, &mut bcx.func);
        let arg = bcx.ins().iconst(types::I32, 5);
        let call = bcx.ins().call(local_func, &[arg]);
        let value = {
            let results = bcx.inst_results(call);
            assert_eq!(results.len(), 1);
            results[0]
        };
        bcx.ins().return_(&[value]);
        bcx.seal_all_blocks();
        bcx.finalize();
    }
    module.define_function(func_b, &mut ctx).unwrap();
    module.clear_context(&mut ctx);

    // Perform linking.
    module.finalize_definitions().unwrap();

    // Get a raw pointer to the generated code.
    let code_b = module.get_finalized_function(func_b);

    // Cast it to a rust function pointer type.
    let ptr_b = unsafe { mem::transmute::<_, extern "C" fn() -> u32>(code_b) };

    // Call it!
    let res = ptr_b();
    println!("Result: {}", res);
}
