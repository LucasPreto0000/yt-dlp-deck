type VisualCore = {
  pulse(time: number): number;
  driftX(time: number): number;
  driftY(time: number): number;
  energy(time: number, progress: number): number;
};

export type BackdropController = {
  setProgress(value: number): void;
  destroy(): void;
  renderer: "webgpu-wgsl" | "webgl2-glsl";
};

async function loadVisualCore(): Promise<VisualCore> {
  const response = await fetch("/visual-core.wasm");
  const bytes = await response.arrayBuffer();
  const instance = await WebAssembly.instantiate(bytes, {});
  return instance.instance.exports as unknown as VisualCore;
}

const wgslShader = /* wgsl */ `
struct Uniforms {
  resolution: vec2f,
  time: f32,
  pulse: f32,
  drift: vec2f,
  energy: f32,
  progress: f32,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

fn hash(p: vec2f) -> f32 {
  return fract(sin(dot(p, vec2f(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2f) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let a = hash(i);
  let b = hash(i + vec2f(1.0, 0.0));
  let c = hash(i + vec2f(0.0, 1.0));
  let d = hash(i + vec2f(1.0, 1.0));
  let s = f * f * (3.0 - 2.0 * f);
  return mix(a, b, s.x) + (c - a) * s.y * (1.0 - s.x) + (d - b) * s.x * s.y;
}

@vertex
fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
  let positions = array<vec2f, 3>(
    vec2f(-1.0, -1.0),
    vec2f(3.0, -1.0),
    vec2f(-1.0, 3.0)
  );
  return vec4f(positions[index], 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) position: vec4f) -> @location(0) vec4f {
  let uv = position.xy / u.resolution;
  let aspect = u.resolution.x / max(u.resolution.y, 1.0);
  var p = (uv - 0.5) * vec2f(aspect, 1.0);
  p += u.drift;

  let orangeCenter = vec2f(-0.38 + u.drift.x, -0.22);
  let purpleCenter = vec2f(0.48, 0.35 + u.drift.y);
  let orangeGlow = exp(-dot(p - orangeCenter, p - orangeCenter) * (3.4 - u.pulse * 0.5));
  let purpleGlow = exp(-dot(p - purpleCenter, p - purpleCenter) * 4.0);
  let grain = noise(p * 4.2 + u.time * 0.025) * 0.018;
  let gridX = smoothstep(0.985, 1.0, cos(p.x * 92.0));
  let gridY = smoothstep(0.985, 1.0, cos(p.y * 92.0));
  let grid = (gridX + gridY) * 0.007 * u.energy;

  var color = vec3f(0.021, 0.024, 0.039);
  color += vec3f(1.0, 0.22, 0.08) * orangeGlow * 0.05 * u.energy;
  color += vec3f(0.34, 0.16, 0.95) * purpleGlow * 0.038;
  color += vec3f(grid + grain);
  return vec4f(color, 1.0);
}
`;

const glslVertex = `#version 300 es
in vec2 position;
void main() { gl_Position = vec4(position, 0.0, 1.0); }`;

const glslFragment = `#version 300 es
precision highp float;
uniform vec2 resolution;
uniform float time;
uniform float pulse;
uniform vec2 drift;
uniform float energy;
out vec4 outColor;

float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float noise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  float a = hash(i), b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0)), d = hash(i + vec2(1.0, 1.0));
  vec2 s = f * f * (3.0 - 2.0 * f);
  return mix(a, b, s.x) + (c - a) * s.y * (1.0 - s.x) + (d - b) * s.x * s.y;
}
void main() {
  vec2 uv = gl_FragCoord.xy / resolution;
  vec2 p = (uv - .5) * vec2(resolution.x / max(resolution.y, 1.0), 1.0) + drift;
  float a = exp(-dot(p - vec2(-.38 + drift.x, -.22), p - vec2(-.38 + drift.x, -.22)) * (3.4 - pulse * .5));
  float b = exp(-dot(p - vec2(.48, .35 + drift.y), p - vec2(.48, .35 + drift.y)) * 4.0);
  float grain = noise(p * 4.2 + time * .025) * .018;
  vec3 color = vec3(.021, .024, .039);
  color += vec3(1.0, .22, .08) * a * .05 * energy;
  color += vec3(.34, .16, .95) * b * .038;
  color += grain;
  outColor = vec4(color, 1.0);
}`;

function resizeCanvas(canvas: HTMLCanvasElement) {
  const scale = Math.min(window.devicePixelRatio || 1, 1.25);
  const width = Math.max(1, Math.floor(canvas.clientWidth * scale));
  const height = Math.max(1, Math.floor(canvas.clientHeight * scale));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
}

async function startWebGpu(
  canvas: HTMLCanvasElement,
  core: VisualCore,
): Promise<BackdropController | null> {
  if (!navigator.gpu) return null;
  const adapter = await navigator.gpu.requestAdapter({ powerPreference: "low-power" });
  if (!adapter) return null;
  const device = await adapter.requestDevice();
  const context = canvas.getContext("webgpu");
  if (!context) return null;
  const format = navigator.gpu.getPreferredCanvasFormat();
  context.configure({ device, format, alphaMode: "opaque" });
  const module = device.createShaderModule({ code: wgslShader });
  const pipeline = device.createRenderPipeline({
    layout: "auto",
    vertex: { module, entryPoint: "vs" },
    fragment: { module, entryPoint: "fs", targets: [{ format }] },
    primitive: { topology: "triangle-list" },
  });
  const uniformBuffer = device.createBuffer({
    size: 32,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });
  const bindGroup = device.createBindGroup({
    layout: pipeline.getBindGroupLayout(0),
    entries: [{ binding: 0, resource: { buffer: uniformBuffer } }],
  });
  let frame = 0;
  let progress = 0;
  let stopped = false;
  const render = (now: number) => {
    if (stopped) return;
    if (document.hidden) {
      frame = requestAnimationFrame(render);
      return;
    }
    resizeCanvas(canvas);
    const values = new Float32Array([
      canvas.width,
      canvas.height,
      now / 1000,
      core.pulse(now),
      core.driftX(now),
      core.driftY(now),
      core.energy(now, progress),
      progress,
    ]);
    device.queue.writeBuffer(uniformBuffer, 0, values);
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0.02, g: 0.022, b: 0.035, a: 1 },
        loadOp: "clear",
        storeOp: "store",
      }],
    });
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.draw(3);
    pass.end();
    device.queue.submit([encoder.finish()]);
    frame = requestAnimationFrame(render);
  };
  frame = requestAnimationFrame(render);
  return {
    renderer: "webgpu-wgsl",
    setProgress: (value) => { progress = value; },
    destroy: () => {
      stopped = true;
      cancelAnimationFrame(frame);
      uniformBuffer.destroy();
      device.destroy();
    },
  };
}

function compileShader(gl: WebGL2RenderingContext, kind: number, source: string) {
  const shader = gl.createShader(kind);
  if (!shader) throw new Error("Não foi possível criar o shader.");
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader) || "Falha ao compilar shader GLSL.");
  }
  return shader;
}

function startWebGl(
  canvas: HTMLCanvasElement,
  core: VisualCore,
): BackdropController {
  const gl = canvas.getContext("webgl2", { alpha: false, antialias: false });
  if (!gl) throw new Error("WebGL2 indisponível.");
  const program = gl.createProgram()!;
  gl.attachShader(program, compileShader(gl, gl.VERTEX_SHADER, glslVertex));
  gl.attachShader(program, compileShader(gl, gl.FRAGMENT_SHADER, glslFragment));
  gl.linkProgram(program);
  const buffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
  const position = gl.getAttribLocation(program, "position");
  gl.enableVertexAttribArray(position);
  gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
  const resolution = gl.getUniformLocation(program, "resolution");
  const time = gl.getUniformLocation(program, "time");
  const pulse = gl.getUniformLocation(program, "pulse");
  const drift = gl.getUniformLocation(program, "drift");
  const energy = gl.getUniformLocation(program, "energy");
  let frame = 0;
  let progress = 0;
  let stopped = false;
  const render = (now: number) => {
    if (stopped) return;
    if (document.hidden) {
      frame = requestAnimationFrame(render);
      return;
    }
    resizeCanvas(canvas);
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.useProgram(program);
    gl.uniform2f(resolution, canvas.width, canvas.height);
    gl.uniform1f(time, now / 1000);
    gl.uniform1f(pulse, core.pulse(now));
    gl.uniform2f(drift, core.driftX(now), core.driftY(now));
    gl.uniform1f(energy, core.energy(now, progress));
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    frame = requestAnimationFrame(render);
  };
  frame = requestAnimationFrame(render);
  return {
    renderer: "webgl2-glsl",
    setProgress: (value) => { progress = value; },
    destroy: () => {
      stopped = true;
      cancelAnimationFrame(frame);
      gl.deleteProgram(program);
      gl.deleteBuffer(buffer);
    },
  };
}

export async function startGpuBackdrop(canvas: HTMLCanvasElement) {
  const core = await loadVisualCore();
  return (await startWebGpu(canvas, core)) ?? startWebGl(canvas, core);
}
