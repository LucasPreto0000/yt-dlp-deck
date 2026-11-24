use std::{
    ffi::{c_char, c_void, CString},
    ptr::{null_mut, NonNull},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use glow::HasContext;
use jni::{objects::GlobalRef, JNIEnv, JavaVM};
use ndk_sys::{ANativeWindow, ANativeWindow_fromSurface, ANativeWindow_release};

#[path = "../../visual-core/lib.rs"]
mod core;

const VERTEX: &str = include_str!(concat!(env!("OUT_DIR"), "/deck.vert"));
const FRAGMENT: &str = include_str!(concat!(env!("OUT_DIR"), "/deck.frag"));

type EglHandle = *mut c_void;
type EglBoolean = u32;
const EGL_NONE: i32 = 0x3038;
const EGL_WIDTH: i32 = 0x3057;
const EGL_HEIGHT: i32 = 0x3056;

#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(native_display: EglHandle) -> EglHandle;
    fn eglInitialize(display: EglHandle, major: *mut i32, minor: *mut i32) -> EglBoolean;
    fn eglChooseConfig(
        display: EglHandle,
        attrs: *const i32,
        configs: *mut EglHandle,
        capacity: i32,
        count: *mut i32,
    ) -> EglBoolean;
    fn eglGetConfigAttrib(
        display: EglHandle,
        config: EglHandle,
        attribute: i32,
        value: *mut i32,
    ) -> EglBoolean;
    fn eglCreateContext(
        display: EglHandle,
        config: EglHandle,
        shared: EglHandle,
        attrs: *const i32,
    ) -> EglHandle;
    fn eglCreateWindowSurface(
        display: EglHandle,
        config: EglHandle,
        window: *mut ANativeWindow,
        attrs: *const i32,
    ) -> EglHandle;
    fn eglMakeCurrent(
        display: EglHandle,
        draw: EglHandle,
        read: EglHandle,
        context: EglHandle,
    ) -> EglBoolean;
    fn eglGetProcAddress(name: *const c_char) -> *const c_void;
    fn eglQuerySurface(
        display: EglHandle,
        surface: EglHandle,
        what: i32,
        value: *mut i32,
    ) -> EglBoolean;
    fn eglSwapInterval(display: EglHandle, interval: i32) -> EglBoolean;
    fn eglSwapBuffers(display: EglHandle, surface: EglHandle) -> EglBoolean;
    fn eglDestroySurface(display: EglHandle, surface: EglHandle) -> EglBoolean;
    fn eglDestroyContext(display: EglHandle, context: EglHandle) -> EglBoolean;
    fn eglTerminate(display: EglHandle) -> EglBoolean;
}

struct NativeWindow(NonNull<ANativeWindow>);

impl Drop for NativeWindow {
    fn drop(&mut self) {
        // ANativeWindow_fromSurface acquires one reference.
        unsafe { ANativeWindow_release(self.0.as_ptr()) };
    }
}

struct Egl {
    display: EglHandle,
    surface: EglHandle,
    context: EglHandle,
}

impl Egl {
    fn new(window: &NativeWindow) -> Result<Self, &'static str> {
        // All EGL objects are confined to this renderer thread.
        unsafe {
            let display = eglGetDisplay(null_mut());
            if display.is_null() || eglInitialize(display, null_mut(), null_mut()) == 0 {
                return Err("EGL indisponível");
            }
            let mut egl = Self {
                display,
                surface: null_mut(),
                context: null_mut(),
            };
            let attrs = [
                0x3033, 0x0004, // EGL_SURFACE_TYPE, EGL_WINDOW_BIT
                0x3040, 0x0040, // EGL_RENDERABLE_TYPE, EGL_OPENGL_ES3_BIT
                0x3024, 8, // EGL_RED_SIZE
                0x3023, 8, // EGL_GREEN_SIZE
                0x3022, 8, // EGL_BLUE_SIZE
                EGL_NONE,
            ];
            let mut config = null_mut();
            let mut count = 0;
            if eglChooseConfig(display, attrs.as_ptr(), &mut config, 1, &mut count) == 0
                || count == 0
            {
                return Err("Configuração OpenGL ES 3 indisponível");
            }
            let mut format = 0;
            if eglGetConfigAttrib(display, config, 0x302e, &mut format) == 0
                || ndk_sys::ANativeWindow_setBuffersGeometry(window.0.as_ptr(), 0, 0, format) != 0
            {
                return Err("Formato da janela indisponível");
            }
            let context = [0x3098, 3, EGL_NONE]; // EGL_CONTEXT_CLIENT_VERSION
            egl.context = eglCreateContext(display, config, null_mut(), context.as_ptr());
            egl.surface =
                eglCreateWindowSurface(display, config, window.0.as_ptr(), [EGL_NONE].as_ptr());
            if egl.context.is_null()
                || egl.surface.is_null()
                || eglMakeCurrent(display, egl.surface, egl.surface, egl.context) == 0
            {
                return Err("Não foi possível iniciar o fundo OpenGL ES");
            }
            eglSwapInterval(display, 1);
            Ok(egl)
        }
    }

    fn size(&self) -> Option<(i32, i32)> {
        let (mut width, mut height) = (0, 0);
        unsafe {
            eglQuerySurface(self.display, self.surface, EGL_WIDTH, &mut width);
            eglQuerySurface(self.display, self.surface, EGL_HEIGHT, &mut height);
        }
        (width > 0 && height > 0).then_some((width, height))
    }
}

impl Drop for Egl {
    fn drop(&mut self) {
        unsafe {
            eglMakeCurrent(self.display, null_mut(), null_mut(), null_mut());
            if !self.surface.is_null() {
                eglDestroySurface(self.display, self.surface);
            }
            if !self.context.is_null() {
                eglDestroyContext(self.display, self.context);
            }
            eglTerminate(self.display);
        }
    }
}

fn native_window(
    env: &mut JNIEnv,
    holder: &GlobalRef,
) -> jni::errors::Result<Option<NativeWindow>> {
    let surface = env
        .call_method(
            holder.as_obj(),
            "getSurface",
            "()Landroid/view/Surface;",
            &[],
        )?
        .l()?;
    if surface.is_null() || !env.call_method(&surface, "isValid", "()Z", &[])?.z()? {
        return Ok(None);
    }
    let window = unsafe { ANativeWindow_fromSurface(env.get_raw(), surface.as_raw()) };
    Ok(NonNull::new(window).map(NativeWindow))
}

fn draw(
    window: &NativeWindow,
    started: Instant,
    stopped: &AtomicBool,
    paused: &AtomicBool,
) -> Result<(), String> {
    let egl = Egl::new(window)?;
    unsafe {
        let gl = glow::Context::from_loader_function(|name| {
            CString::new(name).map_or(std::ptr::null(), |name| eglGetProcAddress(name.as_ptr()))
        });
        let program = gl.create_program()?;
        for (kind, source) in [
            (glow::VERTEX_SHADER, VERTEX),
            (glow::FRAGMENT_SHADER, FRAGMENT),
        ] {
            let shader = gl.create_shader(kind)?;
            gl.shader_source(shader, source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                return Err(gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
            gl.delete_shader(shader);
        }
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(gl.get_program_info_log(program));
        }
        gl.use_program(Some(program));

        let position = gl
            .get_attrib_location(program, "position")
            .ok_or("Atributo position ausente")?;
        let vertices: [f32; 6] = [-1., -1., 3., -1., -1., 3.];
        let bytes =
            std::slice::from_raw_parts(vertices.as_ptr().cast(), std::mem::size_of_val(&vertices));
        let vao = gl.create_vertex_array()?;
        gl.bind_vertex_array(Some(vao));
        let buffer = gl.create_buffer()?;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
        gl.vertex_attrib_pointer_f32(position, 2, glow::FLOAT, false, 0, 0);
        gl.enable_vertex_attrib_array(position);

        let resolution = gl.get_uniform_location(program, "resolution");
        let time = gl.get_uniform_location(program, "time");
        let pulse = gl.get_uniform_location(program, "pulse");
        let drift = gl.get_uniform_location(program, "drift");
        let energy = gl.get_uniform_location(program, "energy");
        let mut sampled = f32::NEG_INFINITY;
        while !stopped.load(Ordering::Acquire) {
            if paused.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            let Some((width, height)) = egl.size() else {
                break;
            };
            let frame_started = Instant::now();
            let now = started.elapsed().as_secs_f32() * 1000.;
            if now - sampled >= 50. {
                sampled = now;
                gl.uniform_1_f32(pulse.as_ref(), core::pulse(now));
                gl.uniform_2_f32(drift.as_ref(), core::driftX(now), core::driftY(now));
                gl.uniform_1_f32(energy.as_ref(), core::energy(now, 0.));
            }
            gl.viewport(0, 0, width, height);
            gl.uniform_2_f32(resolution.as_ref(), width as f32, height as f32);
            gl.uniform_1_f32(time.as_ref(), now / 1000.);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            if eglSwapBuffers(egl.display, egl.surface) == 0 {
                break;
            }
            thread::sleep(Duration::from_millis(16).saturating_sub(frame_started.elapsed()));
        }
    }
    Ok(())
}

pub(crate) fn run(vm: &JavaVM, holder: &GlobalRef, stopped: &AtomicBool, paused: &AtomicBool) {
    let started = Instant::now();
    while !stopped.load(Ordering::Acquire) {
        let window = vm.attach_current_thread().ok().and_then(|mut env| {
            match env.with_local_frame(8, |env| native_window(env, holder)) {
                Ok(window) => window,
                Err(_) => {
                    let _ = env.exception_clear();
                    None
                }
            }
        });
        if let Some(window) = window {
            // An invalidated Surface is recreated by the platform. The next iteration picks it up.
            let _ = draw(&window, started, stopped, paused);
        }
        thread::sleep(Duration::from_millis(150));
    }
}
