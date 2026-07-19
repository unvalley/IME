use std::hint::black_box;
use std::time::Instant;

use ime_core::{ImeEngine, InputEvent};

fn main() {
    let iterations = iterations(50_000);

    run("engine/nihon_conversion", iterations, || {
        let mut engine = ImeEngine::bundled();
        for character in black_box("nihon").chars() {
            black_box(engine.handle(InputEvent::Character(character)));
        }
        black_box(engine.handle(InputEvent::Space));
        black_box(engine.handle(InputEvent::Enter));
    });
}

fn iterations(default: u64) -> u64 {
    std::env::var("IME_BENCH_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn run(name: &str, iterations: u64, mut operation: impl FnMut()) {
    for _ in 0..1_000 {
        operation();
    }

    let started = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    let elapsed = started.elapsed();
    let nanos = elapsed.as_nanos() / u128::from(iterations);
    println!("{name}\t{nanos}\tns/op\t{iterations}\titerations");
}
