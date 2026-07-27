package com.ytdlpdeck.mobiledownloader

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import com.arthenica.ffmpegkit.FFmpegKit

class DownloadForegroundService : Service() {
    private var title = "Preparando download"
    private var message = "Inicializando yt-dlp…"
    private var percent = 0

    override fun onCreate() {
        super.onCreate()
        createChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_PAUSE -> {
                val record = activeRecord()
                if (record?.status in PAUSABLE_STATES && DownloadRuntime.pause()) {
                    message = "Download pausado"
                    persistControl("paused", message, "[controle] Download pausado pela notificação.")
                } else {
                    message = "A etapa atual não pode ser pausada"
                }
            }
            ACTION_RESUME -> {
                if (DownloadRuntime.resume()) {
                    message = "Download retomado"
                    persistControl("running", message, "[controle] Download retomado pela notificação.")
                }
            }
            ACTION_CANCEL -> {
                DownloadRuntime.cancel()
                FFmpegKit.cancel()
                message = "Cancelando download…"
                persistControl("cancelled", message, "[controle] Cancelamento solicitado pela notificação.")
            }
            ACTION_UPDATE -> {
                title = intent.getStringExtra(EXTRA_TITLE).orEmpty().ifBlank { title }
                message = intent.getStringExtra(EXTRA_MESSAGE).orEmpty().ifBlank { message }
                percent = intent.getIntExtra(EXTRA_PERCENT, percent).coerceIn(0, 100)
            }
            ACTION_FINISH -> {
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
                return START_NOT_STICKY
            }
            else -> {
                title = intent?.getStringExtra(EXTRA_TITLE).orEmpty().ifBlank { title }
            }
        }

        val notification = buildNotification()
        startForeground(
            NOTIFICATION_ID,
            notification,
            ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC,
        )
        return START_NOT_STICKY
    }

    override fun onTimeout(startId: Int, fgsType: Int) {
        DownloadRuntime.cancel()
        FFmpegKit.cancel()
        persistControl(
            "cancelled",
            "O Android encerrou o serviço após atingir o limite de execução.",
            "[sistema] Limite de execução em segundo plano atingido.",
        )
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf(startId)
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        stopForeground(STOP_FOREGROUND_REMOVE)
        super.onDestroy()
    }

    private fun buildNotification(): Notification {
        val openIntent = packageManager.getLaunchIntentForPackage(packageName)
        val contentIntent = openIntent?.let {
            PendingIntent.getActivity(
                this,
                10,
                it.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
        }
        val builder = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setContentTitle(title)
            .setContentText(message)
            .setContentIntent(contentIntent)
            .setOnlyAlertOnce(true)
            .setOngoing(!DownloadRuntime.isCancelled())
            .setSilent(true)
            .setCategory(NotificationCompat.CATEGORY_PROGRESS)
            .setProgress(100, percent, percent <= 0)

        if (!DownloadRuntime.isCancelled()) {
            val canPause = activeRecord()?.status in PAUSABLE_STATES
            val pauseAction = if (DownloadRuntime.isPaused()) ACTION_RESUME else ACTION_PAUSE
            val pauseLabel = if (DownloadRuntime.isPaused()) "Retomar" else "Pausar"
            if (DownloadRuntime.isPaused() || canPause) {
                builder.addAction(
                    0,
                    pauseLabel,
                    serviceAction(pauseAction, 11),
                )
            }
            builder.addAction(
                0,
                "Cancelar",
                serviceAction(ACTION_CANCEL, 12),
            )
        }
        return builder.build()
    }

    private fun activeRecord(): MobileDownloadRecord? =
        DownloadRuntime.activeId?.let { DownloadStore.get(this).find(it) }

    private fun persistControl(status: String, newMessage: String, line: String) {
        val id = DownloadRuntime.activeId ?: return
        DownloadStore.get(this).update(id) {
            it.status = status
            it.message = newMessage
            it.console.add(line)
            if (it.console.size > 300) {
                it.console.subList(0, it.console.size - 300).clear()
            }
        }
    }

    private fun serviceAction(action: String, requestCode: Int): PendingIntent =
        PendingIntent.getService(
            this,
            requestCode,
            Intent(this, DownloadForegroundService::class.java).setAction(action),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

    private fun createChannel() {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(
                CHANNEL_ID,
                "Downloads",
                NotificationManager.IMPORTANCE_LOW,
            ).apply {
                description = "Progresso dos downloads do YT-DLP Deck"
                setShowBadge(false)
            },
        )
    }

    companion object {
        private const val CHANNEL_ID = "yt_dlp_deck_downloads"
        private const val NOTIFICATION_ID = 7401
        private const val ACTION_START = "com.ytdlpdeck.action.START"
        private const val ACTION_UPDATE = "com.ytdlpdeck.action.UPDATE"
        private const val ACTION_FINISH = "com.ytdlpdeck.action.FINISH"
        private const val ACTION_PAUSE = "com.ytdlpdeck.action.PAUSE"
        private const val ACTION_RESUME = "com.ytdlpdeck.action.RESUME"
        private const val ACTION_CANCEL = "com.ytdlpdeck.action.CANCEL"
        private const val EXTRA_TITLE = "title"
        private const val EXTRA_MESSAGE = "message"
        private const val EXTRA_PERCENT = "percent"
        private val PAUSABLE_STATES = setOf("queued", "running", "paused")

        fun start(context: Context, title: String) {
            val intent = Intent(context, DownloadForegroundService::class.java)
                .setAction(ACTION_START)
                .putExtra(EXTRA_TITLE, title)
            ContextCompat.startForegroundService(context, intent)
        }

        fun update(context: Context, title: String, message: String, percent: Double) {
            context.startService(
                Intent(context, DownloadForegroundService::class.java)
                    .setAction(ACTION_UPDATE)
                    .putExtra(EXTRA_TITLE, title)
                    .putExtra(EXTRA_MESSAGE, message)
                    .putExtra(EXTRA_PERCENT, percent.toInt()),
            )
        }

        fun finish(context: Context) {
            context.stopService(Intent(context, DownloadForegroundService::class.java))
        }
    }
}
