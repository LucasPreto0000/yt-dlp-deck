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
    fun toJson(includeConsole: Boolean = true): JSONObject = JSONObject().apply {
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
        put("console", if (includeConsole) JSONArray(console) else JSONArray())
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
        migrateLegacyIfNeeded()
        readIds()
            .mapNotNull(::readRecord)
            .sortedByDescending { it.createdAt }
    }

    fun find(id: String): MobileDownloadRecord? = synchronized(lock) {
        migrateLegacyIfNeeded()
        readRecord(id)
    }

    fun save(record: MobileDownloadRecord) = synchronized(lock) {
        migrateLegacyIfNeeded()
        val previousIds = readIds()
        val ids = buildList {
            add(record.id)
            addAll(previousIds.filterNot { it == record.id })
        }.take(MAX_HISTORY)
        preferences.edit().apply {
            putString(recordKey(record.id), record.toJson().toString())
            putString(INDEX_KEY, JSONArray(ids).toString())
            previousIds.filterNot(ids::contains).forEach { remove(recordKey(it)) }
        }.apply()
    }

    fun update(id: String, change: (MobileDownloadRecord) -> Unit): MobileDownloadRecord? =
        synchronized(lock) {
            migrateLegacyIfNeeded()
            val record = readRecord(id) ?: return@synchronized null
            change(record)
            record.updatedAt = System.currentTimeMillis()
            preferences.edit()
                .putString(recordKey(id), record.toJson().toString())
                .apply()
            record
        }

    fun clearFinished() = synchronized(lock) {
        migrateLegacyIfNeeded()
        writeAll(list().filter { it.status in ACTIVE_STATES })
    }

    fun remove(id: String): MobileDownloadRecord? = synchronized(lock) {
        migrateLegacyIfNeeded()
        val removed = readRecord(id) ?: return@synchronized null
        val ids = readIds().filterNot { it == id }
        preferences.edit()
            .remove(recordKey(id))
            .putString(INDEX_KEY, JSONArray(ids).toString())
            .apply()
        removed
    }

    private fun readIds(): List<String> = runCatching {
        val array = JSONArray(preferences.getString(INDEX_KEY, "[]"))
        List(array.length()) { index -> array.getString(index) }
    }.getOrDefault(emptyList())

    private fun readRecord(id: String): MobileDownloadRecord? = runCatching {
        preferences.getString(recordKey(id), null)
            ?.let(::JSONObject)
            ?.let(MobileDownloadRecord::fromJson)
    }.getOrNull()

    private fun writeAll(records: List<MobileDownloadRecord>) {
        val previousIds = readIds()
        val recordsById = records
            .sortedByDescending { it.createdAt }
            .take(MAX_HISTORY)
            .associateBy { it.id }
        preferences.edit().apply {
            previousIds.filterNot(recordsById::containsKey).forEach { remove(recordKey(it)) }
            recordsById.forEach { (id, record) ->
                putString(recordKey(id), record.toJson().toString())
            }
            putString(INDEX_KEY, JSONArray(recordsById.keys.toList()).toString())
        }.apply()
    }

    private fun migrateLegacyIfNeeded() {
        if (preferences.contains(INDEX_KEY)) return
        val records = runCatching {
            val array = JSONArray(preferences.getString(LEGACY_KEY, "[]"))
            List(array.length()) { index -> MobileDownloadRecord.fromJson(array.getJSONObject(index)) }
        }.getOrDefault(emptyList())
        preferences.edit().putString(INDEX_KEY, "[]").remove(LEGACY_KEY).apply()
        if (records.isNotEmpty()) writeAll(records)
    }

    private fun recordKey(id: String) = "$RECORD_PREFIX$id"

    companion object {
        private const val LEGACY_KEY = "history"
        private const val INDEX_KEY = "history_v2_ids"
        private const val RECORD_PREFIX = "record_"
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

    @Synchronized
    fun begin(id: String) {
        check(activeId == null) { "Já existe um download em andamento." }
        activeId = id
        paused.set(false)
        cancelled.set(false)
    }

    @Synchronized
    fun pause(): Boolean {
        if (activeId == null || cancelled.get()) return false
        paused.set(true)
        return true
    }

    @Synchronized
    fun resume(): Boolean {
        if (activeId == null || cancelled.get()) return false
        paused.set(false)
        return true
    }

    @Synchronized
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

    @Synchronized
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
