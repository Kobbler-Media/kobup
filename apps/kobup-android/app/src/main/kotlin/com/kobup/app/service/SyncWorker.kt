package com.kobup.app.service

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import com.kobup.app.data.autoUpdatable
import com.kobup.app.data.availableUpdates
import com.kobup.app.data.local.PrefsManager
import com.kobup.app.data.repository.AppRepository

class SyncWorker(
    context: Context,
    params: WorkerParameters,
) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result {
        val prefs = PrefsManager(applicationContext)
        if (prefs.getServerUrl().isNullOrBlank()) {
            Log.d("Kobup", "Sync skipped — not configured yet")
            return Result.success()
        }

        return try {
            val repo = AppRepository(applicationContext)
            val catalog = repo.refreshCatalog()
            val autoUpdates = if (AutoUpdateWorker.supported && prefs.getAutoUpdate()) {
                autoUpdatable(catalog, applicationContext.packageName, repo::getInstalledVersion, repo::getInstallerPackage)
            } else {
                emptyList()
            }
            UpdateNotifier.sync(applicationContext, availableUpdates(catalog, repo::getInstalledVersion) - autoUpdates.toSet())
            if (autoUpdates.isNotEmpty()) AutoUpdateWorker.enqueue(applicationContext)
            repo.heartbeat()
            try {
                repo.syncPushToken()
            } catch (e: Exception) {
                Log.w("Kobup", "Push token sync failed", e)
            }
            Log.i("Kobup", "Sync completed successfully")
            Result.success()
        } catch (e: Exception) {
            Log.e("Kobup", "Sync failed", e)
            Result.retry()
        }
    }
}
