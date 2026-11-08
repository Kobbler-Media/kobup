package com.kobup.app.service

import android.Manifest
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationChannelCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.core.content.edit
import com.kobup.app.MainActivity
import com.kobup.app.R
import com.kobup.app.data.model.CatalogItem
import com.kobup.app.data.resolvedNotifications
import com.kobup.app.data.unnotified

object UpdateNotifier {

    private const val CHANNEL_ID = "updates"
    private const val PREFS_NAME = "kobup_notified_updates"

    fun sync(context: Context, updates: List<CatalogItem>) {
        val manager = NotificationManagerCompat.from(context)
        val prefs = prefs(context)
        val notified = notified(prefs)

        prefs.edit {
            val stillPending = updates.map { it.packageName }.toSet()
            notified.keys.filterNot { it in stillPending }.forEach { pkg ->
                manager.cancel(pkg.hashCode())
                remove(pkg)
            }

            if (!canNotify(context, manager)) return@edit
            createChannel(manager)
            val openApp = PendingIntent.getActivity(
                context,
                0,
                Intent(context, MainActivity::class.java)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP),
                PendingIntent.FLAG_IMMUTABLE,
            )
            unnotified(updates, notified).forEach { item ->
                val latest = item.latestVersion ?: return@forEach
                val notification = NotificationCompat.Builder(context, CHANNEL_ID)
                    .setSmallIcon(R.drawable.ic_stat_update)
                    .setContentTitle("${item.name} update available")
                    .setContentText("Version ${latest.versionName} is ready to install")
                    .setContentIntent(openApp)
                    .setAutoCancel(true)
                    .build()
                try {
                    manager.notify(item.packageName.hashCode(), notification)
                    putLong(item.packageName, latest.versionCode)
                } catch (_: SecurityException) {
                    // Permission revoked between the check and the post.
                }
            }
        }
    }

    fun needsConfirmation(context: Context, packageName: String?, confirmIntent: Intent) {
        val manager = NotificationManagerCompat.from(context)
        if (packageName == null || !canNotify(context, manager)) return
        createChannel(manager)
        val label = runCatching {
            context.packageManager.getApplicationLabel(context.packageManager.getApplicationInfo(packageName, 0))
        }.getOrDefault(packageName)
        val confirm = PendingIntent.getActivity(
            context,
            packageName.hashCode(),
            confirmIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val notification = NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_update)
            .setContentTitle("$label update downloaded")
            .setContentText("Tap to finish installing")
            .setContentIntent(confirm)
            .setAutoCancel(true)
            .build()
        try {
            manager.notify(packageName.hashCode(), notification)
        } catch (_: SecurityException) {
            // Permission revoked between the check and the post.
        }
    }

    // Runs off package-change broadcasts with no network, so it can only retire notifications, never post them.
    fun dismissResolved(context: Context, installedVersionCode: (String) -> Long?) {
        val manager = NotificationManagerCompat.from(context)
        val prefs = prefs(context)
        val resolved = resolvedNotifications(notified(prefs), installedVersionCode)
        if (resolved.isEmpty()) return
        prefs.edit {
            resolved.forEach { pkg ->
                manager.cancel(pkg.hashCode())
                remove(pkg)
            }
        }
    }

    private fun createChannel(manager: NotificationManagerCompat) {
        manager.createNotificationChannel(
            NotificationChannelCompat.Builder(CHANNEL_ID, NotificationManagerCompat.IMPORTANCE_DEFAULT)
                .setName("App updates")
                .build(),
        )
    }

    private fun prefs(context: Context) = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    private fun notified(prefs: SharedPreferences): Map<String, Long> =
        prefs.all.mapNotNull { (pkg, code) -> (code as? Long)?.let { pkg to it } }.toMap()

    private fun canNotify(context: Context, manager: NotificationManagerCompat): Boolean {
        if (!manager.areNotificationsEnabled()) return false
        return Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
    }
}
