use std::{
    ffi::c_void,
    io,
    os::{fd::AsRawFd, unix::net::UnixDatagram},
    ptr::NonNull,
    sync::Arc,
};

use jni::{
    objects::{GlobalRef, JObject, JValue},
    JNIEnv, JavaVM,
};
use ndk_sys::{ALooper, ALooper_addFd, ALooper_forThread, ALooper_removeFd, ALOOPER_EVENT_INPUT};

struct Receiver {
    socket: UnixDatagram,
    window: GlobalRef,
    vm: Arc<JavaVM>,
}

pub(super) struct Navigation {
    looper: NonNull<ALooper>,
    receiver: Box<Receiver>,
}

impl Navigation {
    pub fn listen(
        env: &mut JNIEnv,
        activity: &JObject,
        vm: Arc<JavaVM>,
    ) -> Option<(Self, UnixDatagram)> {
        let looper = NonNull::new(unsafe { ALooper_forThread() })?;
        let (rx, tx) = UnixDatagram::pair().ok()?;
        rx.set_nonblocking(true).ok()?;
        tx.set_nonblocking(true).ok()?;
        let window = env
            .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])
            .ok()?
            .l()
            .ok()?;
        let window = env.new_global_ref(window).ok()?;
        let mut receiver = Box::new(Receiver {
            socket: rx,
            window,
            vm,
        });
        let data = std::ptr::from_mut(receiver.as_mut()).cast();
        // This looper invokes the callback on the activity thread. The boxed
        // receiver stays alive until the file descriptor is unregistered.
        if unsafe {
            ALooper_addFd(
                looper.as_ptr(),
                receiver.socket.as_raw_fd(),
                -1,
                ALOOPER_EVENT_INPUT as i32,
                Some(on_message),
                data,
            )
        } != 1
        {
            return None;
        }
        Some((Self { looper, receiver }, tx))
    }
}

impl Drop for Navigation {
    fn drop(&mut self) {
        unsafe {
            ALooper_removeFd(self.looper.as_ptr(), self.receiver.socket.as_raw_fd());
        }
    }
}

unsafe extern "C" fn on_message(_fd: i32, _events: i32, data: *mut c_void) -> i32 {
    let receiver = &*data.cast::<Receiver>();
    let mut bytes = [0];
    let mut message = None;
    loop {
        match receiver.socket.recv(&mut bytes) {
            Ok(1) => message = Some(bytes[0] != 0),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            _ => break,
        }
    }
    if let (Some(enabled), Ok(mut env)) = (message, receiver.vm.attach_current_thread()) {
        if env
            .with_local_frame(8, |env| apply(env, receiver.window.as_obj(), enabled))
            .is_err()
        {
            let _ = env.exception_clear();
        }
    }
    1
}

pub(super) fn apply(env: &mut JNIEnv, window: &JObject, enabled: bool) -> jni::errors::Result<()> {
    let controller = env
        .call_method(
            window,
            "getInsetsController",
            "()Landroid/view/WindowInsetsController;",
            &[],
        )?
        .l()?;
    if controller.is_null() {
        return Ok(());
    }
    let navigation = env
        .call_static_method(
            "android/view/WindowInsets$Type",
            "navigationBars",
            "()I",
            &[],
        )?
        .i()?;
    env.call_method(
        &controller,
        "setSystemBarsBehavior",
        "(I)V",
        &[JValue::Int(2)],
    )?;
    env.call_method(
        &controller,
        if enabled { "hide" } else { "show" },
        "(I)V",
        &[JValue::Int(navigation)],
    )?;
    Ok(())
}
