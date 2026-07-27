package com.ytdlpdeck.mobiledownloader

import android.Manifest
import android.app.Activity
import android.app.ActivityManager
import android.app.DownloadManager
import android.content.ContentValues
import android.content.Intent
import android.content.pm.PackageManager
import android.media.MediaScannerConnection
import android.net.Uri
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.Build
import android.os.Environment
import android.os.PowerManager
import android.provider.MediaStore
import android.provider.OpenableColumns
import android.webkit.WebView
import androidx.activity.result.ActivityResult
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import androidx.documentfile.provider.DocumentFile
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.arthenica.ffmpegkit.FFmpegKit
import com.arthenica.ffmpegkit.ReturnCode
import com.chaquo.python.Python
import com.chaquo.python.android.AndroidPlatform
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.Locale
import java.util.UUID

@InvokeArg
class SearchArgs {
    lateinit var query: String
}

@InvokeArg
class DownloadRequestArgs {
    lateinit var url: String
    lateinit var platformFolder: String
    lateinit var format: String
    var quality: String? = null
    var cookies: String = "none"
    var cookieFile: String? = null
    var wifiOnly: Boolean = false
}

@InvokeArg
class StartDownloadArgs {
    lateinit var request: DownloadRequestArgs
}

@InvokeArg
class OpenDownloadsArgs {
    var platformFolder: String? = null
}

@InvokeArg
class OpenUrlArgs {
    lateinit var url: String
}

@InvokeArg
class DownloadControlArgs {
    lateinit var action: String
}

@InvokeArg
class DownloadItemArgs {
    lateinit var id: String
}

private data class SavedMedia(val uri: Uri, val displayLocation: String)

private class PythonProgress(
    private val activity: Activity,
    private val store: DownloadStore,
    private val record: MobileDownloadRecord,
    private val emit: (String) -> Unit,
    private val emitState: (MobileDownloadRecord) -> Unit,
) {
    private var lastPersistAt = 0L

    @Suppress("unused")
    fun onProgress(line: String) {
        DownloadRuntime.checkpoint()
        append(line)
    }

    @Suppress("unused")
    fun onLog(line: String) {
        DownloadRuntime.checkpoint()
        val normalized = line.trim()
        if (normalized.isBlank() || normalized.contains(KNOWN_JS_WARNING, ignoreCase = true)) return
        append(normalized)
    }

    @Suppress("unused")
    fun checkpoint() = DownloadRuntime.checkpoint()

    private fun append(line: String) {
        val percent = PROGRESS.find(line)?.groupValues?.getOrNull(1)?.toDoubleOrNull()
        if (percent != null) record.percent = percent.coerceIn(0.0, 100.0)
        record.status = if (DownloadRuntime.isPaused()) "paused" else "running"
        record.message = when {
            DownloadRuntime.isPaused() -> "Download pausado"
            line.contains("ETA", ignoreCase = true) -> line.substringAfter("de mídia · ", line)
            else -> line
        }.take(240)
        record.console.add(line)
        if (record.console.size > 300) record.console.subList(0, record.console.size - 300).clear()
        emit(line)

        val now = System.currentTimeMillis()
        if (now - lastPersistAt >= 650 || percent == 100.0) {
            record.updatedAt = now
            store.save(record)
            emitState(record)
            DownloadForegroundService.update(activity, record.title, record.message, record.percent)
            lastPersistAt = now
        }
    }

    companion object {
        private val PROGRESS = Regex("""\[download]\s+([0-9.]+)%""")
        private const val KNOWN_JS_WARNING = "No supported JavaScript runtime could be found"
    }
}

@TauriPlugin(
    permissions = [
        Permission(
            strings = [Manifest.permission.POST_NOTIFICATIONS],
            alias = "notifications",
        ),
        Permission(
            strings = [Manifest.permission.WRITE_EXTERNAL_STORAGE],
            alias = "storage",
        ),
    ],
)
class MobileDownloaderPlugin(private val activity: Activity) : Plugin(activity) {
    private val ioScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private val store by lazy { DownloadStore.get(activity) }
    private var pendingSharedUrl: String? = null

    override fun load(webView: WebView) {
        pendingSharedUrl = extractSharedUrl(activity.intent)
        ioScope.launch { recoverInterruptedJobs() }
    }

    override fun onNewIntent(intent: Intent) {
        extractSharedUrl(intent)?.let { url ->
            pendingSharedUrl = url
            trigger("shared-url", JSObject().apply { put("url", url) })
        }
    }

    @Command
    fun checkTools(invoke: Invoke) {
        invoke.resolve(
            JSObject().apply {
                put("ytDlp", true)
                put("ffmpeg", true)
                put("ytDlpVersion", "2026.7.4 + EJS · incorporado")
                put("ffmpegVersion", "8.1.7 · incorporado")
                put("toolsDir", "Componentes internos do aplicativo")
            },
        )
    }

    @Command
    fun searchVideos(invoke: Invoke) {
        val args = invoke.parseArgs(SearchArgs::class.java)
        ioScope.launch {
            runCatching {
                require(args.query.trim().length >= 2) { "Digite pelo menos dois caracteres." }
                val module = python().getModule("ytdlp_mobile")
                val json = module.callAttr("search", args.query.trim()).toString()
                JSObject().apply { put("items", JSONArray(json)) }
            }.onSuccess(invoke::resolve).onFailure { reject(invoke, it) }
        }
    }

    @Command
    fun startDownload(invoke: Invoke) {
        val args = invoke.parseArgs(StartDownloadArgs::class.java)
        ioScope.launch {
            var record: MobileDownloadRecord? = null
            var completed = false
            try {
                val request = args.request
                validateDownloadRequest(request)
                ensureNetworkAvailable(request.wifiOnly)
                ensureLegacyStoragePermission()

                val id = UUID.randomUUID().toString()
                val platform = safeName(request.platformFolder.ifBlank { "Outro" })
                val workRoot = File(
                    activity.getExternalFilesDir(null) ?: activity.filesDir,
                    "download-jobs",
                ).apply { mkdirs() }
                check(workRoot.usableSpace >= MIN_FREE_SPACE_BYTES) {
                    "Espaço insuficiente. Libere pelo menos 256 MB e tente novamente."
                }
                val workDir = File(workRoot, id).apply { mkdirs() }
                val now = System.currentTimeMillis()
                record = MobileDownloadRecord(
                    id = id,
                    title = "Preparando mídia",
                    url = request.url.trim(),
                    status = "queued",
                    percent = 0.0,
                    message = "Preparando o yt-dlp incorporado…",
                    outputDir = configuredOutputLabel(platform),
                    fileUri = null,
                    fileName = null,
                    createdAt = now,
                    updatedAt = now,
                    console = mutableListOf(),
                )
                DownloadRuntime.begin(id)
                store.save(record)
                DownloadForegroundService.start(activity, record.title)
                emitRecordState(record)

                val progress = PythonProgress(
                    activity = activity,
                    store = store,
                    record = record,
                    emit = ::emitOutput,
                    emitState = ::emitRecordState,
                )
                progress.onProgress("[download] 1.0% Preparando o yt-dlp incorporado…")
                val resultJson = python()
                    .getModule("ytdlp_mobile")
                    .callAttr(
                        "download",
                        request.url.trim(),
                        workDir.absolutePath,
                        request.format,
                        request.quality ?: "best",
                        adaptiveFragmentCount(),
                        request.cookieFile.orEmpty(),
                        progress,
                    ).toString()
                val result = JSONObject(resultJson)
                record.title = safeName(result.optString("title", "download"))
                record.status = "processing"
                record.percent = maxOf(record.percent, 92.0)
                record.message = "FFmpeg está processando a mídia…"
                store.save(record)
                emitRecordState(record)

                val converted = processMedia(result, request.format, workDir, progress)
                progress.onProgress("[download] 97.0% Salvando na pasta Downloads…")
                record.status = "saving"
                val saved = saveToDownloads(converted, platform)
                record.status = "completed"
                record.percent = 100.0
                record.message = "Download concluído com sucesso."
                record.outputDir = saved.displayLocation
                record.fileUri = saved.uri.toString()
                record.fileName = converted.name
                record.console.add("[download] 100.0% Download concluído.")
                store.save(record)
                emitOutput("[download] 100.0% Download concluído.")
                emitRecordState(record)
                completed = true
                workDir.deleteRecursively()

                invoke.resolve(
                    JSObject().apply {
                        put("success", true)
                        put("outputDir", record.outputDir)
                        put("message", record.message)
                        put("jobId", record.id)
                    },
                )
            } catch (error: Throwable) {
                record?.let {
                    val cancelled = error is DownloadCancelledException || DownloadRuntime.isCancelled()
                    it.status = if (cancelled) "cancelled" else "failed"
                    it.message = if (cancelled) "Download cancelado. O arquivo parcial foi preservado para diagnóstico." else
                        (error.message ?: error.javaClass.simpleName)
                    it.console.add("${if (cancelled) "AVISO" else "ERRO"}: ${it.message}")
                    store.save(it)
                    emitOutput(it.console.last())
                    emitRecordState(it)
                }
                reject(invoke, error)
            } finally {
                record?.let {
                    DownloadRuntime.finish(it.id)
                    DownloadForegroundService.finish(activity)
                    if (!completed) {
                        emitOutput("[sistema] Arquivos parciais preservados para diagnóstico.")
                    }
                }
            }
        }
    }

    @Command
    fun controlDownload(invoke: Invoke) {
        val args = invoke.parseArgs(DownloadControlArgs::class.java)
        val changed = when (args.action.lowercase(Locale.ROOT)) {
            "pause" -> DownloadRuntime.pause()
            "resume" -> DownloadRuntime.resume()
            "cancel" -> {
                val result = DownloadRuntime.cancel()
                if (result) FFmpegKit.cancel()
                result
            }
            else -> false
        }
        if (!changed) {
            invoke.reject("Não há download ativo para executar essa ação.")
            return
        }
        DownloadRuntime.activeId?.let { id ->
            store.update(id) {
                it.status = when (args.action.lowercase(Locale.ROOT)) {
                    "pause" -> "paused"
                    "resume" -> "running"
                    else -> "cancelled"
                }
                it.message = when (args.action.lowercase(Locale.ROOT)) {
                    "pause" -> "Download pausado"
                    "resume" -> "Download retomado"
                    else -> "Cancelando download…"
                }
            }?.let {
                emitRecordState(it)
                DownloadForegroundService.update(activity, it.title, it.message, it.percent)
            }
        }
        invoke.resolve(downloadStateJson())
    }

    @Command
    fun getDownloadState(invoke: Invoke) = invoke.resolve(downloadStateJson())

    @Command
    fun getDownloadHistory(invoke: Invoke) {
        invoke.resolve(JSObject().apply { put("items", JSONArray(store.list().map { it.toJson() })) })
    }

    @Command
    fun clearDownloadHistory(invoke: Invoke) {
        store.clearFinished()
        invoke.resolve(JSObject().apply { put("ok", true) })
    }

    @Command
    fun openDownloadItem(invoke: Invoke) {
        withDownloadItem(invoke) { record ->
            val uri = shareableUri(record)
            activity.startActivity(
                Intent(Intent.ACTION_VIEW)
                    .setDataAndType(uri, mimeType(record.fileName.orEmpty().substringAfterLast('.')))
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION),
            )
        }
    }

    @Command
    fun shareDownloadItem(invoke: Invoke) {
        withDownloadItem(invoke) { record ->
            val uri = shareableUri(record)
            activity.startActivity(
                Intent.createChooser(
                    Intent(Intent.ACTION_SEND)
                        .setType(mimeType(record.fileName.orEmpty().substringAfterLast('.')))
                        .putExtra(Intent.EXTRA_STREAM, uri)
                        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION),
                    "Compartilhar mídia",
                ),
            )
        }
    }

    @Command
    fun deleteDownloadItem(invoke: Invoke) {
        val args = invoke.parseArgs(DownloadItemArgs::class.java)
        val record = store.remove(args.id)
        if (record == null) {
            invoke.reject("Download não encontrado no histórico.")
            return
        }
        deleteStoredUri(activity, record.fileUri)
        invoke.resolve(JSObject().apply { put("ok", true) })
    }

    @Command
    fun openDownloadsFolder(invoke: Invoke) {
        runCatching {
            val selectedTree = selectedTreeUri()
            val intent = if (selectedTree != null) {
                Intent(Intent.ACTION_VIEW, selectedTree)
            } else {
                Intent(DownloadManager.ACTION_VIEW_DOWNLOADS)
            }
            activity.startActivity(intent)
            JSObject().apply { put("ok", true) }
        }.onSuccess(invoke::resolve).onFailure { reject(invoke, it) }
    }

    @Command
    fun openExternalUrl(invoke: Invoke) {
        val args = invoke.parseArgs(OpenUrlArgs::class.java)
        runCatching {
            val uri = validatedHttpUri(args.url)
            activity.startActivity(Intent(Intent.ACTION_VIEW, uri))
            JSObject().apply { put("ok", true) }
        }.onSuccess(invoke::resolve).onFailure { reject(invoke, it) }
    }

    @Command
    fun getMobileSettings(invoke: Invoke) {
        invoke.resolve(settingsJson(consumeSharedUrl = true))
    }

    @Command
    fun requestMobilePermissions(invoke: Invoke) {
        val aliases = buildList {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                ContextCompat.checkSelfPermission(activity, Manifest.permission.POST_NOTIFICATIONS) !=
                PackageManager.PERMISSION_GRANTED
            ) {
                add("notifications")
            }
            if (Build.VERSION.SDK_INT <= Build.VERSION_CODES.P &&
                ContextCompat.checkSelfPermission(activity, Manifest.permission.WRITE_EXTERNAL_STORAGE) !=
                PackageManager.PERMISSION_GRANTED
            ) {
                add("storage")
            }
        }
        if (aliases.isEmpty()) {
            invoke.resolve(settingsJson())
        } else {
            requestPermissionForAliases(aliases.toTypedArray(), invoke, "mobilePermissionResult")
        }
    }

    @PermissionCallback
    fun mobilePermissionResult(invoke: Invoke) {
        invoke.resolve(settingsJson())
    }

    @Command
    fun chooseDownloadDirectory(invoke: Invoke) {
        startActivityForResult(
            invoke,
            Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
                addFlags(
                    Intent.FLAG_GRANT_READ_URI_PERMISSION or
                        Intent.FLAG_GRANT_WRITE_URI_PERMISSION or
                        Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION,
                )
            },
            "downloadDirectoryResult",
        )
    }

    @ActivityCallback
    fun downloadDirectoryResult(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) {
            invoke.resolve(settingsJson())
            return
        }
        runCatching {
            activity.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
            )
            activity.getSharedPreferences(PREFERENCES, Activity.MODE_PRIVATE)
                .edit()
                .putString(DOWNLOAD_TREE_KEY, uri.toString())
                .apply()
            settingsJson()
        }.onSuccess(invoke::resolve).onFailure { reject(invoke, it) }
    }

    @Command
    fun chooseCookieFile(invoke: Invoke) {
        startActivityForResult(
            invoke,
            Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "text/plain"
            },
            "cookieFileResult",
        )
    }

    @ActivityCallback
    fun cookieFileResult(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) {
            invoke.reject("Nenhum arquivo cookies.txt foi selecionado.")
            return
        }
        runCatching {
            val directory = File(activity.filesDir, "cookies").apply { mkdirs() }
            val target = File(directory, "imported-cookies.txt")
            activity.contentResolver.openInputStream(uri).use { input ->
                checkNotNull(input) { "Não foi possível abrir o cookies.txt selecionado." }
                target.outputStream().use { output -> input.copyTo(output) }
            }
            if (target.length() !in 1..MAX_COOKIE_FILE_BYTES) {
                target.delete()
                error("O cookies.txt está vazio ou excede o limite de 10 MB.")
            }
            val displayName = activity.contentResolver.query(
                uri,
                arrayOf(OpenableColumns.DISPLAY_NAME),
                null,
                null,
                null,
            )?.use { cursor ->
                if (cursor.moveToFirst()) cursor.getString(0) else null
            } ?: "cookies.txt"
            JSObject().apply {
                put("path", target.absolutePath)
                put("name", displayName)
            }
        }.onSuccess(invoke::resolve).onFailure { reject(invoke, it) }
    }

    private fun validateDownloadRequest(request: DownloadRequestArgs) {
        require(request.url.isNotBlank()) { "Informe uma URL antes de iniciar o download." }
        validatedHttpUri(request.url)
        require(request.cookies == "none" || request.cookies == "file") {
            "No Android, use conteúdo público ou importe um arquivo cookies.txt."
        }
        if (request.cookies == "file") {
            require(!request.cookieFile.isNullOrBlank() && File(request.cookieFile!!).isFile) {
                "Selecione um arquivo cookies.txt válido."
            }
        }
    }

    private fun ensureLegacyStoragePermission() {
        if (Build.VERSION.SDK_INT <= Build.VERSION_CODES.P) {
            check(
                ContextCompat.checkSelfPermission(
                    activity,
                    Manifest.permission.WRITE_EXTERNAL_STORAGE,
                ) == PackageManager.PERMISSION_GRANTED,
            ) { "Permita o acesso ao armazenamento para salvar em Downloads." }
        }
    }

    private fun ensureNetworkAvailable(wifiOnly: Boolean) {
        val manager = activity.getSystemService(ConnectivityManager::class.java)
        val network = checkNotNull(manager.activeNetwork) {
            "Nenhuma conexão com a internet está disponível."
        }
        val capabilities = checkNotNull(manager.getNetworkCapabilities(network)) {
            "Não foi possível verificar a conexão atual."
        }
        check(capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)) {
            "A rede atual não possui acesso à internet."
        }
        if (wifiOnly) {
            check(capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) {
                "A opção Somente Wi-Fi está ativa. Conecte-se a uma rede Wi-Fi."
            }
        }
    }

    private fun python(): Python {
        if (!Python.isStarted()) Python.start(AndroidPlatform(activity))
        return Python.getInstance()
    }

    private fun emitOutput(line: String) {
        trigger("download-output", JSObject().apply { put("line", line) })
    }

    private fun emitRecordState(record: MobileDownloadRecord) {
        trigger("download-state", JSObject.fromJSONObject(record.toJson()))
    }

    private fun downloadStateJson(): JSObject {
        val activeId = DownloadRuntime.activeId
        val current = activeId?.let(store::find)
        return JSObject().apply {
            put("active", activeId != null)
            put("paused", DownloadRuntime.isPaused())
            put("cancelled", DownloadRuntime.isCancelled())
            put("current", current?.toJson())
        }
    }

    private fun processMedia(
        result: JSONObject,
        requestedFormat: String,
        workDir: File,
        progress: PythonProgress,
    ): File {
        DownloadRuntime.checkpoint()
        val files = result.getJSONArray("files")
        val inputs = (0 until files.length()).map { files.getJSONObject(it) }
        val video = inputs.firstOrNull { it.optString("vcodec", "none") != "none" }
        val audioOnly = inputs.firstOrNull {
            it.optString("vcodec", "none") == "none" &&
                it.optString("acodec", "none") != "none"
        }
        val first = File(inputs.first().getString("path"))
        val outputExtension = when (requestedFormat) {
            "mp3", "flac", "wav", "m4a", "mp4", "mkv", "webm" -> requestedFormat
            else -> if (video != null && audioOnly != null) "mkv" else first.extension.ifBlank { "mkv" }
        }
        val title = safeName(result.optString("title", "download")).take(140)
        val id = safeName(result.optString("id", "media"))
        val output = File(workDir, "$title [$id].$outputExtension")

        if (inputs.size == 1 && first.extension.equals(outputExtension, ignoreCase = true)) {
            return first
        }

        progress.onProgress("[download] 93.0% FFmpeg está processando o arquivo…")
        val arguments = when {
            requestedFormat in AUDIO_FORMATS -> {
                val source = File((audioOnly ?: inputs.first()).getString("path"))
                audioArguments(source, output, requestedFormat)
            }
            video != null && audioOnly != null -> mutableListOf(
                "-y",
                "-i", video.getString("path"),
                "-i", audioOnly.getString("path"),
                "-map", "0:v:0",
                "-map", "1:a:0",
                "-c", "copy",
            ).apply {
                if (outputExtension == "mp4") addAll(listOf("-movflags", "+faststart"))
                add(output.absolutePath)
            }
            else -> mutableListOf(
                "-y",
                "-i", first.absolutePath,
                "-c", "copy",
            ).apply {
                if (outputExtension == "mp4") addAll(listOf("-movflags", "+faststart"))
                add(output.absolutePath)
            }
        }
        val session = FFmpegKit.executeWithArguments(arguments.toTypedArray())
        if (!ReturnCode.isSuccess(session.returnCode) && outputExtension == "mp4" && !DownloadRuntime.isCancelled()) {
            progress.onLog("[ffmpeg] Remux incompatível; aplicando conversão de compatibilidade para MP4.")
            val fallback = mutableListOf("-y")
            if (video != null && audioOnly != null) {
                fallback.addAll(
                    listOf(
                        "-i", video.getString("path"),
                        "-i", audioOnly.getString("path"),
                        "-map", "0:v:0",
                        "-map", "1:a:0",
                    ),
                )
            } else {
                fallback.addAll(listOf("-i", first.absolutePath))
            }
            fallback.addAll(
                listOf(
                    "-c:v", "mpeg4",
                    "-q:v", "2",
                    "-c:a", "aac",
                    "-b:a", "192k",
                    "-movflags", "+faststart",
                    output.absolutePath,
                ),
            )
            val fallbackSession = FFmpegKit.executeWithArguments(fallback.toTypedArray())
            check(ReturnCode.isSuccess(fallbackSession.returnCode)) {
                "O FFmpeg não conseguiu converter a mídia: ${fallbackSession.output.takeLast(1200)}"
            }
        } else {
            check(ReturnCode.isSuccess(session.returnCode)) {
                if (DownloadRuntime.isCancelled()) "Download cancelado pelo usuário."
                else "O FFmpeg não conseguiu processar a mídia: ${session.output.takeLast(1200)}"
            }
        }
        DownloadRuntime.checkpoint()
        check(output.isFile) { "O FFmpeg terminou sem gerar o arquivo final." }
        return output
    }

    private fun audioArguments(input: File, output: File, format: String): List<String> {
        val codec = when (format) {
            "mp3" -> listOf("-c:a", "libmp3lame", "-q:a", "0")
            "flac" -> listOf("-c:a", "flac")
            "wav" -> listOf("-c:a", "pcm_s16le")
            else -> listOf("-c:a", "aac", "-b:a", "256k")
        }
        return listOf("-y", "-i", input.absolutePath, "-vn") + codec + listOf(output.absolutePath)
    }

    private fun saveToDownloads(source: File, platform: String): SavedMedia {
        selectedTreeUri()?.let { treeUri ->
            val root = checkNotNull(DocumentFile.fromTreeUri(activity, treeUri)) {
                "A pasta escolhida não está mais disponível."
            }
            val appDirectory = root.findFile("YT-DLP Deck") ?: root.createDirectory("YT-DLP Deck")
            val platformDirectory = appDirectory?.findFile(platform) ?: appDirectory?.createDirectory(platform)
            val targetDirectory = checkNotNull(platformDirectory) { "Não foi possível criar a pasta de destino." }
            targetDirectory.findFile(source.name)?.delete()
            val document = checkNotNull(
                targetDirectory.createFile(mimeType(source.extension), source.name),
            ) { "Não foi possível criar o arquivo na pasta escolhida." }
            activity.contentResolver.openOutputStream(document.uri, "w").use { output ->
                checkNotNull(output) { "Não foi possível abrir o arquivo de destino." }
                source.inputStream().use { input -> input.copyTo(output) }
            }
            return SavedMedia(document.uri, "Pasta escolhida/YT-DLP Deck/$platform")
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val resolver = activity.contentResolver
            val values = ContentValues().apply {
                put(MediaStore.MediaColumns.DISPLAY_NAME, source.name)
                put(MediaStore.MediaColumns.MIME_TYPE, mimeType(source.extension))
                put(
                    MediaStore.MediaColumns.RELATIVE_PATH,
                    "${Environment.DIRECTORY_DOWNLOADS}/YT-DLP Deck/$platform",
                )
                put(MediaStore.MediaColumns.IS_PENDING, 1)
            }
            val uri = checkNotNull(
                resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values),
            ) { "O Android recusou a criação do arquivo em Downloads." }
            try {
                resolver.openOutputStream(uri).use { output ->
                    checkNotNull(output) { "Não foi possível abrir o arquivo de destino." }
                    source.inputStream().use { input -> input.copyTo(output) }
                }
                resolver.update(
                    uri,
                    ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) },
                    null,
                    null,
                )
                return SavedMedia(uri, "Downloads/YT-DLP Deck/$platform")
            } catch (error: Throwable) {
                resolver.delete(uri, null, null)
                throw error
            }
        }

        @Suppress("DEPRECATION")
        val destination = File(
            Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS),
            "YT-DLP Deck/$platform/${source.name}",
        )
        destination.parentFile?.mkdirs()
        source.copyTo(destination, overwrite = true)
        MediaScannerConnection.scanFile(
            activity,
            arrayOf(destination.absolutePath),
            arrayOf(mimeType(destination.extension)),
            null,
        )
        return SavedMedia(Uri.fromFile(destination), "Downloads/YT-DLP Deck/$platform")
    }

    private fun withDownloadItem(invoke: Invoke, operation: (MobileDownloadRecord) -> Unit) {
        val args = invoke.parseArgs(DownloadItemArgs::class.java)
        runCatching {
            val record = checkNotNull(store.find(args.id)) { "Download não encontrado." }
            check(!record.fileUri.isNullOrBlank()) { "Esse item não possui um arquivo concluído." }
            operation(record)
            JSObject().apply { put("ok", true) }
        }.onSuccess(invoke::resolve).onFailure { reject(invoke, it) }
    }

    private fun shareableUri(record: MobileDownloadRecord): Uri {
        val uri = Uri.parse(record.fileUri)
        return if (uri.scheme == "file") {
            FileProvider.getUriForFile(
                activity,
                "${activity.packageName}.fileprovider",
                File(requireNotNull(uri.path)),
            )
        } else {
            uri
        }
    }

    private fun settingsJson(consumeSharedUrl: Boolean = false): JSObject {
        if (pendingSharedUrl == null) pendingSharedUrl = extractSharedUrl(activity.intent)
        val shared = pendingSharedUrl
        if (consumeSharedUrl) {
            pendingSharedUrl = null
            activity.intent?.action = null
        }
        return JSObject().apply {
            put("sharedUrl", shared)
            put("downloadDirectory", configuredOutputLabel(null))
            put(
                "notificationsGranted",
                Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
                    ContextCompat.checkSelfPermission(
                        activity,
                        Manifest.permission.POST_NOTIFICATIONS,
                    ) == PackageManager.PERMISSION_GRANTED,
            )
            put(
                "storageGranted",
                Build.VERSION.SDK_INT > Build.VERSION_CODES.P ||
                    ContextCompat.checkSelfPermission(
                        activity,
                        Manifest.permission.WRITE_EXTERNAL_STORAGE,
                    ) == PackageManager.PERMISSION_GRANTED,
            )
        }
    }

    private fun configuredOutputLabel(platform: String?): String {
        val suffix = platform?.let { "/$it" }.orEmpty()
        return if (selectedTreeUri() != null) {
            "Pasta escolhida/YT-DLP Deck$suffix"
        } else {
            "Downloads/YT-DLP Deck$suffix"
        }
    }

    private fun selectedTreeUri(): Uri? =
        activity.getSharedPreferences(PREFERENCES, Activity.MODE_PRIVATE)
            .getString(DOWNLOAD_TREE_KEY, null)
            ?.let(Uri::parse)

    private fun adaptiveFragmentCount(): Int {
        val activityManager = activity.getSystemService(ActivityManager::class.java)
        val powerManager = activity.getSystemService(PowerManager::class.java)
        return if (activityManager.isLowRamDevice || powerManager.isPowerSaveMode) 2 else 4
    }

    private fun recoverInterruptedJobs() {
        store.list()
            .filter { it.status in ACTIVE_STATUSES }
            .forEach { record ->
                store.update(record.id) {
                    it.status = "failed"
                    it.message = "O Android encerrou o processo anterior. Inicie o download novamente."
                    it.console.add("[sistema] Processo anterior interrompido pelo Android.")
                }
            }
        val jobsRoot = File(activity.getExternalFilesDir(null) ?: activity.filesDir, "download-jobs")
        val expiration = System.currentTimeMillis() - PARTIAL_RETENTION_MILLIS
        jobsRoot.listFiles()
            ?.filter { it.isDirectory && it.lastModified() < expiration }
            ?.forEach { it.deleteRecursively() }
    }

    private fun validatedHttpUri(value: String): Uri {
        val uri = Uri.parse(value.trim())
        require(uri.scheme == "https" || uri.scheme == "http") {
            "Somente endereços HTTP ou HTTPS são permitidos."
        }
        require(!uri.host.isNullOrBlank()) { "O endereço informado não possui um domínio válido." }
        return uri
    }

    private fun extractSharedUrl(intent: Intent?): String? {
        if (intent?.action != Intent.ACTION_SEND || intent.type != "text/plain") return null
        val text = intent.getStringExtra(Intent.EXTRA_TEXT).orEmpty()
        return URL_PATTERN.find(text)?.value?.trimEnd('.', ',', ')', ']', '}')
    }

    private fun mimeType(extension: String): String = when (extension.lowercase(Locale.ROOT)) {
        "mp4" -> "video/mp4"
        "mkv" -> "video/x-matroska"
        "webm" -> "video/webm"
        "mp3" -> "audio/mpeg"
        "flac" -> "audio/flac"
        "wav" -> "audio/wav"
        "m4a" -> "audio/mp4"
        else -> "application/octet-stream"
    }

    private fun safeName(value: String): String {
        val cleaned = value.trim().map { character ->
            if (character in "<>:\"/\\|?*" || character.isISOControl()) '_' else character
        }.joinToString("").trim().trimEnd('.', ' ')
        return cleaned.ifBlank { "download" }
    }

    private fun reject(invoke: Invoke, error: Throwable) {
        invoke.reject(error.message ?: error.javaClass.simpleName)
    }

    companion object {
        private const val PREFERENCES = "mobile_downloader_settings"
        private const val DOWNLOAD_TREE_KEY = "download_tree_uri"
        private const val MIN_FREE_SPACE_BYTES = 256L * 1024L * 1024L
        private const val MAX_COOKIE_FILE_BYTES = 10L * 1024L * 1024L
        private const val PARTIAL_RETENTION_MILLIS = 7L * 24L * 60L * 60L * 1000L
        private val AUDIO_FORMATS = setOf("mp3", "flac", "wav", "m4a")
        private val ACTIVE_STATUSES = setOf("queued", "running", "paused", "processing", "saving")
        private val URL_PATTERN = Regex("""https?://[^\s]+""", RegexOption.IGNORE_CASE)
    }
}
