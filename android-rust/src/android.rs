use std::{
    ffi::c_void,
    fs::File,
    io,
    os::fd::FromRawFd,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use jni::{
    objects::{GlobalRef, JObject, JString, JValue},
    JNIEnv, JavaVM,
};
use ndk_sys::ANativeActivity;

mod bars;

struct Instance {
    stopped: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    view: GlobalRef,
    _navigation: Option<bars::Navigation>,
}

// NativeActivity calls this on Android's UI thread.
#[no_mangle]
pub unsafe extern "C" fn ANativeActivity_onCreate(
    activity: *mut ANativeActivity,
    _saved_state: *mut c_void,
    _saved_state_size: usize,
) {
    let activity = &mut *activity;
    let Ok(mut env) = JNIEnv::from_raw(activity.env) else {
        return;
    };
    let context = JObject::from_raw(activity.clazz);
    match launch(&mut env, &context) {
        Ok(instance) => {
            activity.instance = Box::into_raw(Box::new(instance)).cast();
            (*activity.callbacks).onDestroy = Some(destroy);
            (*activity.callbacks).onPause = Some(pause);
            (*activity.callbacks).onResume = Some(resume);
        }
        Err(error) => {
            let _ = env.exception_clear();
            if let Ok(tag) = env.new_string("YtDlpDeck") {
                if let Ok(message) = env.new_string(error) {
                    let _ = env.call_static_method(
                        "android/util/Log",
                        "e",
                        "(Ljava/lang/String;Ljava/lang/String;)I",
                        &[JValue::Object(&tag), JValue::Object(&message)],
                    );
                }
            }
            let _ = env.call_method(&context, "finish", "()V", &[]);
        }
    }
}

unsafe extern "C" fn pause(activity: *mut ANativeActivity) {
    if let Some(instance) = (*activity).instance.cast::<Instance>().as_ref() {
        instance.paused.store(true, Ordering::Release);
    }
}

unsafe extern "C" fn resume(activity: *mut ANativeActivity) {
    if let Some(instance) = (*activity).instance.cast::<Instance>().as_ref() {
        instance.paused.store(false, Ordering::Release);
    }
}

unsafe extern "C" fn destroy(activity: *mut ANativeActivity) {
    let activity = &mut *activity;
    if activity.instance.is_null() {
        return;
    }
    let instance = Box::from_raw(activity.instance.cast::<Instance>());
    activity.instance = std::ptr::null_mut();
    instance.stopped.store(true, Ordering::Release);
    if let Ok(mut env) = JNIEnv::from_raw(activity.env) {
        let _ = env.call_method(instance.view.as_obj(), "stopLoading", "()V", &[]);
    }
}

fn launch(env: &mut JNIEnv, activity: &JObject) -> Result<Instance, String> {
    let (server, route) = crate::web::listen()?;
    let address = server
        .server_addr()
        .to_ip()
        .ok_or("Endereço local indisponível.")?;
    let (vm, application, view, surface_holder) =
        setup_view(env, activity, &format!("http://{address}{route}"))
            .map_err(|error| error.to_string())?;
    let vm = Arc::new(vm);
    let (navigation, navigation_sender) =
        match bars::Navigation::listen(env, activity, Arc::clone(&vm)) {
            Some((receiver, sender)) => (Some(receiver), Some(sender)),
            None => {
                let _ = env.exception_clear();
                (None, None)
            }
        };
    let stopped = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stopped);
    let gpu_stop = Arc::clone(&stopped);
    let gpu_pause = Arc::clone(&paused);
    let gpu_vm = Arc::clone(&vm);
    std::thread::spawn(move || {
        crate::backdrop::run(&gpu_vm, &surface_holder, &gpu_stop, &gpu_pause)
    });
    std::thread::spawn(move || {
        crate::web::run(
            server,
            &route,
            &thread_stop,
            AndroidActions {
                vm,
                app: application,
                navigation: navigation_sender,
            },
        );
    });
    Ok(Instance {
        stopped,
        paused,
        view,
        _navigation: navigation,
    })
}

struct AndroidActions {
    vm: Arc<JavaVM>,
    app: GlobalRef,
    navigation: Option<std::os::unix::net::UnixDatagram>,
}

impl AndroidActions {
    fn with_android<T>(
        &self,
        call: impl FnOnce(&mut JNIEnv, &JObject) -> jni::errors::Result<T>,
    ) -> Result<T, String> {
        let mut env = self.vm.attach_current_thread().map_err(|e| e.to_string())?;
        let value = env.with_local_frame(24, |env| call(env, self.app.as_obj()));
        if value.is_err() {
            let _ = env.exception_clear();
        }
        value.map_err(|e| e.to_string())
    }
}

fn service<'a>(
    env: &mut JNIEnv<'a>,
    app: &JObject,
    name: &str,
) -> jni::errors::Result<JObject<'a>> {
    let name = env.new_string(name)?;
    env.call_method(
        app,
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[JValue::Object(&name)],
    )?
    .l()
}

impl crate::web::Actions for AndroidActions {
    fn set_immersive(&self, enabled: bool) -> Result<(), String> {
        self.navigation
            .as_ref()
            .ok_or("Controle da navegação indisponível.")?
            .send(&[u8::from(enabled)])
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    fn open_video(&self, id: &str) -> Result<(), String> {
        self.with_android(|env, app| {
            let url = env.new_string(format!("https://www.youtube.com/watch?v={id}"))?;
            let uri = env
                .call_static_method(
                    "android/net/Uri",
                    "parse",
                    "(Ljava/lang/String;)Landroid/net/Uri;",
                    &[JValue::Object(&url)],
                )?
                .l()?;
            let action = env.new_string("android.intent.action.VIEW")?;
            let intent = env.new_object(
                "android/content/Intent",
                "(Ljava/lang/String;Landroid/net/Uri;)V",
                &[JValue::Object(&action), JValue::Object(&uri)],
            )?;
            env.call_method(
                &intent,
                "addFlags",
                "(I)Landroid/content/Intent;",
                &[JValue::Int(0x1000_0000)],
            )?;
            env.call_method(
                app,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )?;
            Ok(())
        })
    }

    fn search(&self, query: &str) -> Result<Vec<crate::web::SearchResult>, String> {
        use std::io::Read;

        let mut url = url::Url::parse("https://www.youtube.com/results").unwrap();
        url.query_pairs_mut()
            .append_pair("search_query", query)
            .append_pair("hl", "pt-BR");
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(12))
            .build()
            .map_err(|e| e.without_url().to_string())?;
        let response = client
            .get(url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Falha na pesquisa: {}", e.without_url()))?;
        let mut page = String::new();
        response
            .take(4 * 1024 * 1024)
            .read_to_string(&mut page)
            .map_err(|e| e.to_string())?;
        crate::search::extract_results(&page)
    }

    fn history_path(&self) -> Option<std::path::PathBuf> {
        self.with_android(|env, app| {
            let directory = env
                .call_method(app, "getFilesDir", "()Ljava/io/File;", &[])?
                .l()?;
            let path = JString::from(
                env.call_method(&directory, "getAbsolutePath", "()Ljava/lang/String;", &[])?
                    .l()?,
            );
            let path: String = env.get_string(&path)?.into();
            Ok(std::path::PathBuf::from(path).join("download-history.json"))
        })
        .ok()
    }

    fn date_and_time(&self) -> (String, String) {
        self.with_android(|env, _app| {
            let now = env
                .call_static_method(
                    "java/time/LocalDateTime",
                    "now",
                    "()Ljava/time/LocalDateTime;",
                    &[],
                )?
                .l()?;
            let value = JString::from(
                env.call_method(&now, "toString", "()Ljava/lang/String;", &[])?
                    .l()?,
            );
            let value: String = env.get_string(&value)?.into();
            let (date, time) = value.split_once('T').unwrap_or(("", ""));
            let date = date.split('-').rev().collect::<Vec<_>>().join("/");
            Ok((date, time.chars().take(5).collect()))
        })
        .unwrap_or_default()
    }

    fn download(
        &self,
        url: &str,
        format: &str,
        folder: &str,
        wifi_only: bool,
        transfer: &crate::web::transfer::Transfer,
    ) -> Result<String, String> {
        if wifi_only {
            let wifi = self.with_android(|env, app| {
                let manager = service(env, app, "connectivity")?;
                let network = env
                    .call_method(&manager, "getActiveNetwork", "()Landroid/net/Network;", &[])?
                    .l()?;
                let caps = env
                    .call_method(
                        &manager,
                        "getNetworkCapabilities",
                        "(Landroid/net/Network;)Landroid/net/NetworkCapabilities;",
                        &[JValue::Object(&network)],
                    )?
                    .l()?;
                if caps.is_null() {
                    return Ok(false);
                }
                env.call_method(&caps, "hasTransport", "(I)Z", &[JValue::Int(1)])?
                    .z()
            })?;
            if !wifi {
                return Err("Conecte o aparelho ao Wi-Fi para iniciar o download.".into());
            }
        }
        download(&self.vm, &self.app, url, format, folder, transfer)
    }

    fn clipboard(&self) -> Result<String, String> {
        self.with_android(|env, app| {
            let manager = service(env, app, "clipboard")?;
            let data = env
                .call_method(
                    &manager,
                    "getPrimaryClip",
                    "()Landroid/content/ClipData;",
                    &[],
                )?
                .l()?;
            if data.is_null() || env.call_method(&data, "getItemCount", "()I", &[])?.i()? == 0 {
                return Ok(String::new());
            }
            let item = env
                .call_method(
                    &data,
                    "getItemAt",
                    "(I)Landroid/content/ClipData$Item;",
                    &[JValue::Int(0)],
                )?
                .l()?;
            let value = env
                .call_method(&item, "getText", "()Ljava/lang/CharSequence;", &[])?
                .l()?;
            if value.is_null() {
                return Ok(String::new());
            }
            let string = JString::from(
                env.call_method(&value, "toString", "()Ljava/lang/String;", &[])?
                    .l()?,
            );
            let text = env.get_string(&string)?;
            Ok(text.into())
        })
    }

    fn copy(&self, text: &str) -> Result<(), String> {
        self.with_android(|env, app| {
            let manager = service(env, app, "clipboard")?;
            let label = env.new_string("YT-DLP Deck")?;
            let text = env.new_string(text)?;
            let data = env
                .call_static_method(
                    "android/content/ClipData",
                    "newPlainText",
                    "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                    &[JValue::Object(&label), JValue::Object(&text)],
                )?
                .l()?;
            env.call_method(
                &manager,
                "setPrimaryClip",
                "(Landroid/content/ClipData;)V",
                &[JValue::Object(&data)],
            )?;
            Ok(())
        })
    }

    fn open_folder(&self) -> Result<(), String> {
        self.with_android(|env, app| {
            let action = env.new_string("android.intent.action.VIEW_DOWNLOADS")?;
            let intent = env.new_object(
                "android/content/Intent",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&action)],
            )?;
            env.call_method(
                &intent,
                "addFlags",
                "(I)Landroid/content/Intent;",
                &[JValue::Int(0x1000_0000)],
            )?;
            env.call_method(
                app,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )?;
            Ok(())
        })
        .map_err(|_| "O aparelho não encontrou um aplicativo para abrir Downloads.".into())
    }
}

fn setup_view(
    env: &mut JNIEnv,
    activity: &JObject,
    origin: &str,
) -> jni::errors::Result<(JavaVM, GlobalRef, GlobalRef, GlobalRef)> {
    let window = env
        .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])?
        .l()?;
    // The platform NativeActivity initially reserves the surface and input queue.
    // Release both before attaching a regular Android view.
    for (method, signature) in [
        ("takeSurface", "(Landroid/view/SurfaceHolder$Callback2;)V"),
        ("takeInputQueue", "(Landroid/view/InputQueue$Callback;)V"),
    ] {
        env.call_method(
            &window,
            method,
            signature,
            &[JValue::Object(&JObject::null())],
        )?;
    }
    env.call_method(&window, "setFormat", "(I)V", &[JValue::Int(1)])?;
    env.call_method(&window, "addFlags", "(I)V", &[JValue::Int(0x0100_0000)])?;
    let insets = env
        .call_method(
            &window,
            "getInsetsController",
            "()Landroid/view/WindowInsetsController;",
            &[],
        )?
        .l()?;
    if !insets.is_null() {
        let navigation = env
            .call_static_method(
                "android/view/WindowInsets$Type",
                "navigationBars",
                "()I",
                &[],
            )?
            .i()?;
        env.call_method(&insets, "setSystemBarsBehavior", "(I)V", &[JValue::Int(2)])?;
        env.call_method(&insets, "hide", "(I)V", &[JValue::Int(navigation)])?;
    }
    let app = env
        .call_method(
            activity,
            "getApplicationContext",
            "()Landroid/content/Context;",
            &[],
        )?
        .l()?;
    let vm = env.get_java_vm()?;
    let app = env.new_global_ref(app)?;
    let container = env.new_object(
        "android/widget/FrameLayout",
        "(Landroid/content/Context;)V",
        &[JValue::Object(activity)],
    )?;
    let surface = env.new_object(
        "android/view/SurfaceView",
        "(Landroid/content/Context;)V",
        &[JValue::Object(activity)],
    )?;
    let holder = env
        .call_method(&surface, "getHolder", "()Landroid/view/SurfaceHolder;", &[])?
        .l()?;
    let holder = env.new_global_ref(holder)?;
    let view = env.new_object(
        "android/webkit/WebView",
        "(Landroid/content/Context;)V",
        &[JValue::Object(activity)],
    )?;
    // Keep forwards and form redirects inside the same WebView.
    let navigation = env.new_object("android/webkit/WebViewClient", "()V", &[])?;
    env.call_method(
        &view,
        "setWebViewClient",
        "(Landroid/webkit/WebViewClient;)V",
        &[JValue::Object(&navigation)],
    )?;
    let settings = env
        .call_method(&view, "getSettings", "()Landroid/webkit/WebSettings;", &[])?
        .l()?;
    for method in [
        "setAllowFileAccess",
        "setAllowContentAccess",
        "setMediaPlaybackRequiresUserGesture",
    ] {
        env.call_method(&settings, method, "(Z)V", &[JValue::Bool(0)])?;
    }
    // The local Rust pages block scripts via CSP. The official YouTube frame
    // is separately sandboxed and allowed to execute its own player scripts.
    env.call_method(
        &settings,
        "setJavaScriptEnabled",
        "(Z)V",
        &[JValue::Bool(1)],
    )?;
    env.call_method(
        &settings,
        "setDomStorageEnabled",
        "(Z)V",
        &[JValue::Bool(1)],
    )?;
    env.call_method(&view, "setBackgroundColor", "(I)V", &[JValue::Int(0)])?;
    env.call_method(
        &container,
        "setBackgroundColor",
        "(I)V",
        &[JValue::Int(0xff08090f_u32 as i32)],
    )?;
    for child in [&surface, &view] {
        env.call_method(
            &container,
            "addView",
            "(Landroid/view/View;II)V",
            &[JValue::Object(child), JValue::Int(-1), JValue::Int(-1)],
        )?;
    }
    env.call_method(
        activity,
        "setContentView",
        "(Landroid/view/View;)V",
        &[JValue::Object(&container)],
    )?;
    let url = env.new_string(origin)?;
    env.call_method(
        &view,
        "loadUrl",
        "(Ljava/lang/String;)V",
        &[JValue::Object(&url)],
    )?;
    let view = env.new_global_ref(view)?;
    Ok((vm, app, view, holder))
}

fn download(
    vm: &JavaVM,
    app: &GlobalRef,
    input: &str,
    format: &str,
    platform: &str,
    transfer: &crate::web::transfer::Transfer,
) -> Result<String, String> {
    let url = url::Url::parse(input).map_err(|error| error.to_string())?;
    let cover = format == "image";
    let (mut response, source_name) = if crate::tiktok::is_page(&url) {
        if !matches!(format, "mp4" | "best" | "image") {
            return Err("Use MP4 ou Melhor para o vídeo do TikTok, ou Imagem para baixar a capa.".into());
        }
        let (response, name) = crate::tiktok::open(url, cover, transfer)?;
        (response, Some(name))
    } else {
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|error| error.to_string())?;
        let response = client
            .get(url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| e.without_url().to_string())?;
        (response, None)
    };
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.split(';').next())
        .unwrap_or_default()
        .to_owned();
    let extension = match mime.as_str() {
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/x-matroska" => "mkv",
        "audio/mpeg" => "mp3",
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/mp4" | "audio/x-m4a" => "m4a",
        "image/jpeg" if cover => "jpg",
        "image/png" if cover => "png",
        "image/webp" if cover => "webp",
        "image/avif" if cover => "avif",
        _ if cover => return Err("O link não retornou uma imagem compatível.".into()),
        _ => return Err("O servidor não retornou um arquivo de mídia compatível. Use um link HTTPS direto do arquivo.".into()),
    };
    if (cover && !mime.starts_with("image/")) || (!cover && format != "best" && format != extension) {
        return Err(format!("A fonte fornece {extension}, mas você selecionou {format}. Escolha o formato original ou Melhor."));
    }
    let name = response
        .url()
        .path_segments()
        .and_then(|mut parts| parts.next_back())
        .and_then(|file| file.rsplit_once('.'))
        .map(|(stem, _)| stem)
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '_' | '-'))
        .take(120)
        .collect::<String>();
    let name = format!(
        "{}.{extension}",
        if let Some(source_name) = source_name.as_deref() {
            source_name
        } else if name.trim().is_empty() {
            "Download"
        } else {
            name.trim()
        }
    );
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| error.to_string())?;
    transfer.total(response.content_length());
    let mut reader = ProgressReader {
        response: &mut response,
        transfer,
    };
    let outcome = save(&mut env, app.as_obj(), &name, &mime, platform, &mut reader);
    if let Err(error) = &outcome {
        let _ = env.exception_clear();
        return Err(format!("Não foi possível salvar o arquivo: {error}"));
    }
    Ok(format!("Salvo em Downloads/YT-DLP Deck/{platform}/{name}"))
}

struct ProgressReader<'a> {
    response: &'a mut reqwest::blocking::Response,
    transfer: &'a crate::web::transfer::Transfer,
}

impl io::Read for ProgressReader<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.transfer.checkpoint()?;
        let count = self.response.read(bytes)?;
        self.transfer.received(count);
        Ok(count)
    }
}

type SaveError = Box<dyn std::error::Error>;

fn content_value(
    env: &mut JNIEnv,
    values: &JObject,
    key: &str,
    value: &str,
) -> jni::errors::Result<()> {
    let key = env.new_string(key)?;
    let value = env.new_string(value)?;
    env.call_method(
        values,
        "put",
        "(Ljava/lang/String;Ljava/lang/String;)V",
        &[JValue::Object(&key), JValue::Object(&value)],
    )?;
    Ok(())
}

fn pending(env: &mut JNIEnv, values: &JObject, value: i32) -> jni::errors::Result<()> {
    let key = env.new_string("is_pending")?;
    let value = env.new_object("java/lang/Integer", "(I)V", &[JValue::Int(value)])?;
    env.call_method(
        values,
        "put",
        "(Ljava/lang/String;Ljava/lang/Integer;)V",
        &[JValue::Object(&key), JValue::Object(&value)],
    )?;
    Ok(())
}

fn save(
    env: &mut JNIEnv,
    app: &JObject,
    name: &str,
    mime: &str,
    platform: &str,
    input: &mut impl io::Read,
) -> Result<(), SaveError> {
    let resolver = env
        .call_method(
            app,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )?
        .l()?;
    let collection = env
        .get_static_field(
            "android/provider/MediaStore$Downloads",
            "EXTERNAL_CONTENT_URI",
            "Landroid/net/Uri;",
        )?
        .l()?;
    let values = env.new_object("android/content/ContentValues", "()V", &[])?;
    content_value(env, &values, "_display_name", name)?;
    content_value(env, &values, "mime_type", mime)?;
    content_value(
        env,
        &values,
        "relative_path",
        &format!("Download/YT-DLP Deck/{platform}"),
    )?;
    pending(env, &values, 1)?;
    let item = env
        .call_method(
            &resolver,
            "insert",
            "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;",
            &[JValue::Object(&collection), JValue::Object(&values)],
        )?
        .l()?;
    if item.is_null() {
        return Err("MediaStore recusou o arquivo.".into());
    }
    let result = (|| -> Result<(), SaveError> {
        let mode = env.new_string("w")?;
        let descriptor = env
            .call_method(
                &resolver,
                "openFileDescriptor",
                "(Landroid/net/Uri;Ljava/lang/String;)Landroid/os/ParcelFileDescriptor;",
                &[JValue::Object(&item), JValue::Object(&mode)],
            )?
            .l()?;
        if descriptor.is_null() {
            return Err("Arquivo indisponível.".into());
        }
        let handle = env.call_method(&descriptor, "detachFd", "()I", &[])?.i()?;
        if handle < 0 {
            return Err("Descritor de arquivo inválido.".into());
        }
        // detachFd transfers sole ownership of the file descriptor.
        let mut output = unsafe { File::from_raw_fd(handle) };
        if io::copy(input, &mut output)? == 0 {
            return Err("O servidor retornou um arquivo vazio.".into());
        }
        drop(output);
        env.call_method(&values, "clear", "()V", &[])?;
        pending(env, &values, 0)?;
        let completed = env.call_method(&resolver, "update", "(Landroid/net/Uri;Landroid/content/ContentValues;Ljava/lang/String;[Ljava/lang/String;)I", &[JValue::Object(&item), JValue::Object(&values), JValue::Object(&JObject::null()), JValue::Object(&JObject::null())])?.i()?;
        if completed == 0 {
            return Err("O Android não publicou o arquivo.".into());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = env.exception_clear();
        let _ = env.call_method(
            &resolver,
            "delete",
            "(Landroid/net/Uri;Ljava/lang/String;[Ljava/lang/String;)I",
            &[
                JValue::Object(&item),
                JValue::Object(&JObject::null()),
                JValue::Object(&JObject::null()),
            ],
        );
    }
    result
}
