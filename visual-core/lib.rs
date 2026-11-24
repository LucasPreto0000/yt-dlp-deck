//! Browser-independent parameters for the WebGPU and WebGL2 backdrop.

#[unsafe(no_mangle)]
pub extern "C" fn pulse(time: f32) -> f32 {
    0.5 + 0.5 * (time * 0.00042).sin()
}

#[unsafe(no_mangle)]
pub extern "C" fn driftX(time: f32) -> f32 {
    (time * 0.00011).sin() * 0.18
}

#[unsafe(no_mangle)]
pub extern "C" fn driftY(time: f32) -> f32 {
    (time * 0.00009).cos() * 0.14
}

#[unsafe(no_mangle)]
pub extern "C" fn energy(time: f32, progress: f32) -> f32 {
    let wave = 0.72 + (time * 0.00027).sin() * 0.18;
    (wave + progress * 0.1).min(1.0)
}
