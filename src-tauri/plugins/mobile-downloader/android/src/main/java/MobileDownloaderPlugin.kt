package com.ytdlpdeck.mobiledownloader

import android.app.Activity
import android.app.DownloadManager
import android.content.ContentValues
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
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

private class PythonProgress(private val emit: (String) -> Unit) {
    @Suppress("unused")
    fun onProgress(line: String) = emit(line)
}

@TauriPlugin
class MobileDownloaderPlugin(private val activity: Activity) : Plugin(activity) {
    private val ioScope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    @Command
    fun checkTools(invoke: Invoke) {
        invoke.resolve(
            JSObject().apply {
                put("ytDlp", true)
                put("ffmpeg", true)
                put("ytDlpVersion", "2026.7.4 · incorporado")
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
            runCatching {
                val request = args.request
                require(request.url.isNotBlank()) { "Informe uma URL antes de iniciar o download." }
                require(request.cookies == "none") {
                    "Cookies de navegadores não podem ser acessados pelo Android. Use conteúdo público."
                }

                val platform = safeName(request.platformFolder.ifBlank { "Outro" })
                val workDir = File(activity.cacheDir, "downloads/${System.nanoTime()}").apply {
                    mkdirs()
                }
                try {
                    emitOutput("[download] 1.0% Preparando o yt-dlp incorporado…")
                    val resultJson = python()
                        .getModule("ytdlp_mobile")
                        .callAttr(
                            "download",
                            request.url.trim(),
                            workDir.absolutePath,
                            request.format,
                            request.quality ?: "best",
                            PythonProgress(::emitOutput),
                        ).toString()
                    val result = JSONObject(resultJson)
                    val converted = processMedia(result, request.format, workDir)
                    emitOutput("[download] 97.0% Salvando na pasta Downloads…")
                    saveToDownloads(converted, platform)
                    emitOutput("[download] 100.0% Download concluído.")

                    JSObject().apply {
                        put("success", true)
                        put("outputDir", "Downloads/YT-DLP Deck/$platform")
                        put("message", "Download concluído com sucesso.")
                    }
                } finally {
                    workDir.deleteRecursively()
                }
            }.onSuccess(invoke::resolve).onFailure { reject(invoke, it) }
        }
    }

    @Command
    fun openDownloadsFolder(invoke: Invoke) {
        runCatching {
            activity.startActivity(Intent(DownloadManager.ACTION_VIEW_DOWNLOADS))
            JSObject().apply { put("ok", true) }
        }.onSuccess(invoke::resolve).onFailure { reject(invoke, it) }
    }

    @Command
    fun openExternalUrl(invoke: Invoke) {
        val args = invoke.parseArgs(OpenUrlArgs::class.java)
        runCatching {
            val uri = Uri.parse(args.url.trim())
            val host = uri.host.orEmpty().lowercase(Locale.ROOT)
            require(uri.scheme == "https" || uri.scheme == "http") {
                "Somente endereços HTTP ou HTTPS podem ser abertos."
            }
            require(
                host == "youtu.be" ||
                    host == "youtube.com" ||
                    host.endsWith(".youtube.com") ||
                    host == "youtube-nocookie.com" ||
                    host.endsWith(".youtube-nocookie.com"),
            ) { "A prévia só pode abrir endereços oficiais do YouTube." }
            activity.startActivity(Intent(Intent.ACTION_VIEW, uri))
            JSObject().apply { put("ok", true) }
        }.onSuccess(invoke::resolve).onFailure { reject(invoke, it) }
    }

    private fun python(): Python {
        if (!Python.isStarted()) {
            Python.start(AndroidPlatform(activity))
        }
        return Python.getInstance()
    }

    private fun emitOutput(line: String) {
        trigger("download-output", JSObject().apply { put("line", line) })
    }

    private fun processMedia(result: JSONObject, requestedFormat: String, workDir: File): File {
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
            first.copyTo(output, overwrite = true)
            return output
        }

        emitOutput("[download] 93.0% FFmpeg está processando o arquivo…")
        val arguments = when {
            requestedFormat in setOf("mp3", "flac", "wav", "m4a") -> {
                val source = File((audioOnly ?: inputs.first()).getString("path"))
                audioArguments(source, output, requestedFormat)
            }
            video != null && audioOnly != null -> listOf(
                "-y",
                "-i", video.getString("path"),
                "-i", audioOnly.getString("path"),
                "-map", "0:v:0",
                "-map", "1:a:0",
                "-c", "copy",
                "-movflags", "+faststart",
                output.absolutePath,
            )
            else -> listOf(
                "-y",
                "-i", first.absolutePath,
                "-c", "copy",
                "-movflags", "+faststart",
                output.absolutePath,
            )
        }
        val session = FFmpegKit.executeWithArguments(arguments.toTypedArray())
        check(ReturnCode.isSuccess(session.returnCode)) {
            "O FFmpeg não conseguiu processar a mídia: ${session.output.takeLast(1200)}"
        }
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
        return listOf("-y", "-i", input.absolutePath, "-vn") +
            codec +
            listOf(output.absolutePath)
    }

    private fun saveToDownloads(source: File, platform: String) {
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
            } catch (error: Throwable) {
                resolver.delete(uri, null, null)
                throw error
            }
        } else {
            val base = activity.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS)
                ?: error("A pasta de downloads do aplicativo não está disponível.")
            val destination = File(base, "YT-DLP Deck/$platform/${source.name}")
            destination.parentFile?.mkdirs()
            source.copyTo(destination, overwrite = true)
        }
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
}
