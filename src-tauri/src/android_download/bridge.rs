use std::ptr;

use jni::{
    objects::{JObject, JString},
    sys::{jboolean, jstring},
    JNIEnv,
};

use super::runtime::DownloadRuntime;

static DOWNLOAD: DownloadRuntime = DownloadRuntime::new();

#[no_mangle]
pub extern "system" fn Java_com_ytdlpdeck_mobiledownloader_DownloadRuntime_nativeBegin(
    mut env: JNIEnv,
    _receiver: JObject,
    id: JString,
) -> jboolean {
    env.get_string(&id)
        .is_ok_and(|value| DOWNLOAD.begin(value.into())) as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_ytdlpdeck_mobiledownloader_DownloadRuntime_nativeActiveId(
    env: JNIEnv,
    _receiver: JObject,
) -> jstring {
    DOWNLOAD
        .active_id()
        .and_then(|id| env.new_string(id).ok())
        .map_or(ptr::null_mut(), JString::into_raw)
}

#[no_mangle]
pub extern "system" fn Java_com_ytdlpdeck_mobiledownloader_DownloadRuntime_nativeFinish(
    mut env: JNIEnv,
    _receiver: JObject,
    id: JString,
) {
    if let Ok(value) = env.get_string(&id) {
        let id: String = value.into();
        DOWNLOAD.finish(&id);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_ytdlpdeck_mobiledownloader_DownloadRuntime_nativePause(
    _env: JNIEnv,
    _receiver: JObject,
) -> jboolean {
    DOWNLOAD.pause() as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_ytdlpdeck_mobiledownloader_DownloadRuntime_nativeResume(
    _env: JNIEnv,
    _receiver: JObject,
) -> jboolean {
    DOWNLOAD.resume() as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_ytdlpdeck_mobiledownloader_DownloadRuntime_nativeCancel(
    _env: JNIEnv,
    _receiver: JObject,
) -> jboolean {
    DOWNLOAD.cancel() as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_ytdlpdeck_mobiledownloader_DownloadRuntime_nativeIsPaused(
    _env: JNIEnv,
    _receiver: JObject,
) -> jboolean {
    DOWNLOAD.is_paused() as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_ytdlpdeck_mobiledownloader_DownloadRuntime_nativeIsCancelled(
    _env: JNIEnv,
    _receiver: JObject,
) -> jboolean {
    DOWNLOAD.is_cancelled() as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_ytdlpdeck_mobiledownloader_DownloadRuntime_nativeCheckpoint(
    _env: JNIEnv,
    _receiver: JObject,
) -> jboolean {
    DOWNLOAD.checkpoint() as jboolean
}
