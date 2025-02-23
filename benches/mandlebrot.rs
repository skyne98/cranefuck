use cranefuck::interpreter::interpret;
use cranefuck::jit::jit;
use cranefuck::optimizer::{optimize, OptimizedIr};
use cranefuck::parser::{to_ir, tokenize};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

// A sample Brainfuck program. You can change this to any code you'd like to benchmark.
const BF_CODE: &str = include_str!("../examples/mandelbrot.bf");

fn custom_config() -> Criterion {
    Criterion::default().sample_size(10)
}

fn prepare_ir() -> Vec<OptimizedIr> {
    // Tokenize and convert the Brainfuck code to intermediate representation (IR).
    let tokens = tokenize(BF_CODE);
    let ir = to_ir(tokens).expect("Failed to generate IR");
    optimize(&ir)
}

fn bench_interpreter(c: &mut Criterion) {
    let ir = prepare_ir();
    c.bench_function("Interpreter", |b| {
        b.iter(|| {
            // Clone the IR since our functions may consume it.
            let result =
                interpret(black_box(ir.clone()), true).expect("Interpreter execution failed");
            black_box(result);
        })
    });
}

fn bench_jit(c: &mut Criterion) {
    let ir = prepare_ir();
    c.bench_function("JIT", |b| {
        b.iter(|| {
            jit(black_box(ir.clone()), true);
        })
    });
}

criterion_group! {
    name = benches;
    config = custom_config();
    targets = bench_interpreter, bench_jit
}
criterion_main!(benches);
