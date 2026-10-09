package com.kobup.sdk

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import com.google.android.material.dialog.MaterialAlertDialogBuilder

object KobupSdk {

    private const val KOBUP_PACKAGE = "com.kobup.app"
    private const val ACTION_OPEN_APP = "com.kobup.action.OPEN_APP"
    private const val EXTRA_PACKAGE_NAME = "package_name"

    /**
     * Check if the Kobup updater app is installed.
     */
    fun isInstalled(context: Context): Boolean {
        return try {
            context.packageManager.getPackageInfo(KOBUP_PACKAGE, 0)
            true
        } catch (e: PackageManager.NameNotFoundException) {
            false
        }
    }

    private const val PROVIDER_AUTHORITY = "com.kobup.app.provider"

    /**
     * Query whether an update is available for the given package.
     * Returns KobupUpdateStatus.KobupNotInstalled if Kobup is not present.
     *
     * Queries the Kobup app's ContentProvider for update information.
     * This performs a synchronous IPC call and may block briefly while
     * Kobup checks its server — call from a background thread.
     */
    fun getUpdateStatus(context: Context, packageName: String): KobupUpdateStatus {
        if (!isInstalled(context)) {
            return KobupUpdateStatus.KobupNotInstalled
        }

        return try {
            val uri = Uri.parse("content://$PROVIDER_AUTHORITY/update_status/$packageName")
            val cursor = context.contentResolver.query(uri, null, null, null, null)
            cursor?.use {
                if (it.moveToFirst()) {
                    val status = it.getString(it.getColumnIndexOrThrow("status"))
                    when (status) {
                        "available" -> {
                            val versionName = it.getString(it.getColumnIndexOrThrow("version_name"))
                            val versionCode = it.getLong(it.getColumnIndexOrThrow("version_code"))
                            KobupUpdateStatus.Available(versionName, versionCode)
                        }
                        "up_to_date" -> KobupUpdateStatus.UpToDate
                        else -> KobupUpdateStatus.Unknown
                    }
                } else {
                    KobupUpdateStatus.Unknown
                }
            } ?: KobupUpdateStatus.Unknown
        } catch (_: Exception) {
            KobupUpdateStatus.Unknown
        }
    }

    /**
     * Open the Kobup app, optionally deep-linking to a specific app's page.
     * No-op if Kobup is not installed.
     */
    fun openKobup(context: Context, packageName: String? = null) {
        if (!isInstalled(context)) return

        try {
            val intent = Intent(ACTION_OPEN_APP).apply {
                setPackage(KOBUP_PACKAGE)
                if (packageName != null) {
                    putExtra(EXTRA_PACKAGE_NAME, packageName)
                }
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            context.startActivity(intent)
        } catch (e: Exception) {
            try {
                val launchIntent = context.packageManager.getLaunchIntentForPackage(KOBUP_PACKAGE)
                if (launchIntent != null) {
                    context.startActivity(launchIntent)
                }
            } catch (_: Exception) {
                // Silently fail — Kobup might have been uninstalled
            }
        }
    }

    /**
     * Show a dialog prompting the user to open Kobup for an update.
     * If Kobup is not installed, no-op.
     */
    fun maybePromptForUpdate(activity: Activity, packageName: String) {
        if (!isInstalled(activity)) return

        MaterialAlertDialogBuilder(activity)
            .setTitle("Update Available")
            .setMessage("A newer version is available. Open Kobup to update?")
            .setPositiveButton("Open Kobup") { _, _ ->
                openKobup(activity, packageName)
            }
            .setNegativeButton("Later", null)
            .show()
    }
}
