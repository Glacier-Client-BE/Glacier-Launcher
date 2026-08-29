package xyz.glacierclient.launcher.service

import android.app.DownloadManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import androidx.core.content.FileProvider
import androidx.webkit.WebViewCompat
import java.io.File

/**
 * Android analogue of GlacierLauncher.Services.AutoUpdateService.
 *
 * Windows can silently replace its own .exe and relaunch (see
 * AutoUpdateService.ApplyUpdateAsync's swap-script). Android has no
 * equivalent — a foreground app can never overwrite or install its own APK
 * without the user confirming the system's package-installer dialog, by
 * design (this is the same wall every self-updating Android app hits, F-Droid
 * and browser updaters included, not something specific to this launcher).
 * So the real, honest parity here is: download the new APK, then hand it to
 * PackageInstaller via a FileProvider content:// Uri, which pops that system
 * dialog. The actual GitHub-releases version check/compare lives in JS
 * (js/updater.js, mirroring AutoUpdateService.CheckLauncherUpdateAsync) since
 * it's a plain public HTTP call like the other integrations in this app —
 * this class only owns the platform-specific download + install-intent part.
 */
object LauncherUpdateService {

    private var receiver: BroadcastReceiver? = null
    private val mainHandler = Handler(Looper.getMainLooper())
    private var pollRunnable: Runnable? = null

    fun downloadAndInstall(
        context: Context,
        url: String,
        tag: String,
        onProgress: (Int) -> Unit,
        onError: (String) -> Unit,
    ) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !context.packageManager.canRequestPackageInstalls()) {
            onError("Allow \"Install unknown apps\" for Glacier Launcher in system settings, then try again.")
            context.startActivity(
                Intent(android.provider.Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${context.packageName}"))
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
            return
        }

        val fileName = "GlacierLauncher-$tag.apk"
        val downloadManager = context.getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager
        val request = DownloadManager.Request(Uri.parse(url))
            .setTitle("Glacier Launcher $tag")
            .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
            .setDestinationInExternalFilesDir(context, android.os.Environment.DIRECTORY_DOWNLOADS, fileName)
            .setMimeType("application/vnd.android.package-archive")
        val downloadId = downloadManager.enqueue(request)

        registerCompletionReceiver(context, downloadManager, downloadId, onError)
        pollProgress(downloadManager, downloadId, onProgress, onError)
    }

    private fun registerCompletionReceiver(context: Context, downloadManager: DownloadManager, downloadId: Long, onError: (String) -> Unit) {
        receiver?.let { runCatching { context.unregisterReceiver(it) } }
        val newReceiver = object : BroadcastReceiver() {
            override fun onReceive(ctx: Context, intent: Intent) {
                if (intent.getLongExtra(DownloadManager.EXTRA_DOWNLOAD_ID, -1) != downloadId) return
                stopPolling()
                runCatching { ctx.unregisterReceiver(this) }
                receiver = null

                val query = DownloadManager.Query().setFilterById(downloadId)
                downloadManager.query(query).use { cursor ->
                    if (!cursor.moveToFirst()) { onError("Update download failed."); return }
                    val statusIdx = cursor.getColumnIndex(DownloadManager.COLUMN_STATUS)
                    if (cursor.getInt(statusIdx) != DownloadManager.STATUS_SUCCESSFUL) {
                        onError("Update download failed."); return
                    }
                    val localUriIdx = cursor.getColumnIndex(DownloadManager.COLUMN_LOCAL_URI)
                    val localUri = cursor.getString(localUriIdx) ?: run { onError("Update download failed."); return }
                    val apkFile = File(Uri.parse(localUri).path ?: return@use)
                    installApk(ctx, apkFile, onError)
                }
            }
        }
        receiver = newReceiver
        val filter = IntentFilter(DownloadManager.ACTION_DOWNLOAD_COMPLETE)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            context.registerReceiver(newReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            @Suppress("UnspecifiedRegisterReceiverFlag")
            context.registerReceiver(newReceiver, filter)
        }
    }

    private fun installApk(context: Context, apkFile: File, onError: (String) -> Unit) {
        if (!apkFile.exists()) { onError("Downloaded update file is missing."); return }
        val apkUri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", apkFile)
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(apkUri, "application/vnd.android.package-archive")
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        runCatching { context.startActivity(intent) }
            .onFailure { onError("Couldn't open the installer: ${it.message}") }
    }

    private fun pollProgress(downloadManager: DownloadManager, downloadId: Long, onProgress: (Int) -> Unit, onError: (String) -> Unit) {
        stopPolling()
        val runnable = object : Runnable {
            override fun run() {
                val query = DownloadManager.Query().setFilterById(downloadId)
                downloadManager.query(query).use { cursor ->
                    if (cursor.moveToFirst()) {
                        val statusIdx = cursor.getColumnIndex(DownloadManager.COLUMN_STATUS)
                        val status = cursor.getInt(statusIdx)
                        if (status == DownloadManager.STATUS_FAILED) {
                            onError("Update download failed.")
                            return
                        }
                        val soFarIdx = cursor.getColumnIndex(DownloadManager.COLUMN_BYTES_DOWNLOADED_SO_FAR)
                        val totalIdx = cursor.getColumnIndex(DownloadManager.COLUMN_TOTAL_SIZE_BYTES)
                        val soFar = cursor.getLong(soFarIdx)
                        val total = cursor.getLong(totalIdx)
                        if (total > 0) onProgress(((soFar * 100) / total).toInt())
                        if (status == DownloadManager.STATUS_SUCCESSFUL) return
                    }
                }
                mainHandler.postDelayed(this, 500)
            }
        }
        pollRunnable = runnable
        mainHandler.post(runnable)
    }

    private fun stopPolling() {
        pollRunnable?.let { mainHandler.removeCallbacks(it) }
        pollRunnable = null
    }
}
