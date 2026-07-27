package com.ytdlpdeck.mobiledownloader

import android.content.Context
import android.net.Uri
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.atomic.AtomicBoolean

internal class DownloadCancelledException : RuntimeException("Download cancelado pelo usuário.")

internal data class MobileDownloadRecord(
    val id: String,
    var title: String,
    val url: String,
    var status: String,
    var percent: Double,
    var message: String,
    var outputDir: String,
    var fileUri: String?,
    var fileName: String?,
    val createdAt: Long,
    var updatedAt: Long,
    val console: MutableList<String>,
) {
    fun toJson(): JSONObject = JSONObject().apply {
        put("id", id)
        put("title", title)
        put("url", url)
        put("status", status)
        put("percent", percent)
        put("message", message)
        put("outputDir", outputDir)
        put("fileUri", fileUri)
        put("fileName", fileName)
        put("createdAt", createdAt)
        put("updatedAt", updatedAt)
        put("console", JSONArray(console))
    }

    companion object {
        fun fromJson(json: JSONObject): MobileDownloadRecord = MobileDownloadRecord(
            id = json.getString("id"),
            title = json.optString("title", "Download"),
            url = json.optString("url"),
            status = json.optString("status", "failed"),
            percent = json.optDouble("percent", 0.0),
            message = json.optString("message"),
            outputDir = json.optString("outputDir"),
            fileUri = json.optString("fileUri").takeIf { it.isNotBlank() && it != "null" },
            fileName = json.optString("fileName").takeIf { it.isNotBlank() && it != "null" },
            createdAt = json.optLong("createdAt", System.currentTimeMillis()),
            updatedAt = json.optLong("updatedAt", System.currentTimeMillis()),
            console = MutableList(json.optJSONArray("console")?.length() ?: 0) { index ->
                json.optJSONArray("console")?.optString(index).orEmpty()
            },
        )
    }
}

internal class DownloadStore private constructor(context: Context) {
    private val preferences = context.getSharedPreferences("mobile_downloads", Context.MODE_PRIVATE)
    private val lock = Any()

    fun list(): List<MobileDownloadRecord> = synchronized(lock) {
        readAll().sortedByDescending { it.createdAt }
    }

    fun find(id: String): MobileDownloadRecord? = synchronized(lock) {
        readAll().firstOrNull { it.id == id }
    }

    fun save(record: MobileDownloadRecord) = synchronized(lock) {
        val records = readAll().filterNot { it.id == record.id }.toMutableList()
        records.add(record)
        writeAll(records.sortedByDescending { it.createdAt }.take(MAX_HISTORY))
    }

    fun update(id: String, change: (MobileDownloadRecord) -> Unit): MobileDownloadRecord? =
        synchronized(lock) {
            val records = readAll().toMutableList()
            val index = records.indexOfFirst { it.id == id }
            if (index < 0) return@synchronized null
            change(records[index])
            records[index].updatedAt = System.currentTimeMillis()
            writeAll(records)
            records[index]
        }

    fun clearFinished() = synchronized(lock) {
        writeAll(readAll().filter { it.status in ACTIVE_STATES })
    }

    fun remove(id: String): MobileDownloadRecord? = synchronized(lock) {
        val records = readAll().toMutableList()
        val removed = records.firstOrNull { it.id == id } ?: return@synchronized null
        records.removeAll { it.id == id }
        writeAll(records)
        removed
    }

    private fun readAll(): List<MobileDownloadRecord> = runCatching {
        val array = JSONArray(preferences.getString(KEY, "[]"))
        List(array.length()) { index -> MobileDownloadRecord.fromJson(array.getJSONObject(index)) }
    }.getOrDefault(emptyList())

    private fun writeAll(records: List<MobileDownloadRecord>) {
        preferences.edit().putString(KEY, JSONArray(records.map { it.toJson() }).toString()).apply()
    }

    companion object {
        private const val KEY = "history"
        private const val MAX_HISTORY = 40
        private val ACTIVE_STATES = setOf("queued", "running", "paused", "processing", "saving")

        @Volatile
        private var instance: DownloadStore? = null

        fun get(context: Context): DownloadStore =
            instance ?: synchronized(this) {
                instance ?: DownloadStore(context.applicationContext).also { instance = it }
            }
    }
}

internal object DownloadRuntime {
    private val paused = AtomicBoolean(false)
    private val cancelled = AtomicBoolean(false)

    @Volatile
    var activeId: String? = null
        private set

    fun begin(id: String) {
        check(activeId == null) { "Já existe um download em andamento." }
        activeId = id
        paused.set(false)
        cancelled.set(false)
    }

    fun pause(): Boolean {
        if (activeId == null || cancelled.get()) return false
        paused.set(true)
        return true
    }

    fun resume(): Boolean {
        if (activeId == null || cancelled.get()) return false
        paused.set(false)
        return true
    }

    fun cancel(): Boolean {
        if (activeId == null) return false
        cancelled.set(true)
        paused.set(false)
        return true
    }

    fun isPaused(): Boolean = paused.get()

    fun isCancelled(): Boolean = cancelled.get()

    fun checkpoint() {
        while (paused.get() && !cancelled.get()) {
            Thread.sleep(180)
        }
        if (cancelled.get()) throw DownloadCancelledException()
    }

    fun finish(id: String) {
        if (activeId == id) {
            activeId = null
            paused.set(false)
            cancelled.set(false)
        }
    }
}

internal fun deleteStoredUri(context: Context, value: String?): Boolean {
    if (value.isNullOrBlank()) return false
    return runCatching {
        val uri = Uri.parse(value)
        when (uri.scheme) {
            "content" -> context.contentResolver.delete(uri, null, null) > 0
            "file" -> java.io.File(requireNotNull(uri.path)).delete()
            else -> false
        }
    }.getOrDefault(false)
}
