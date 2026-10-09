package com.kobup.app.service

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
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
            repo.refreshCatalog()
            repo.heartbeat()
            Log.i("Kobup", "Sync completed successfully")
            Result.success()
        } catch (e: Exception) {
            Log.e("Kobup", "Sync failed", e)
            Result.retry()
        }
    }
}
