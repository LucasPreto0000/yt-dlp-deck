// Pequeno núcleo matemático compilado para WebAssembly.
// Ele dirige os parâmetros dos shaders sem depender do JavaScript para os cálculos.
export function pulse(time: f32): f32 {
  return 0.5 + 0.5 * Mathf.sin(time * 0.00042);
}

export function driftX(time: f32): f32 {
  return Mathf.sin(time * 0.00011) * 0.18;
}

export function driftY(time: f32): f32 {
  return Mathf.cos(time * 0.00009) * 0.14;
}

export function energy(time: f32, progress: f32): f32 {
  const wave: f32 = <f32>0.72 + Mathf.sin(time * <f32>0.00027) * <f32>0.18;
  return Mathf.min(<f32>1.0, wave + progress * <f32>0.1);
}
