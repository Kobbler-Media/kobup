package com.kobup.app.service

import android.content.Context
import android.os.Build
import android.util.Log
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.kobup.app.data.autoUpdatable
import com.kobup.app.data.local.PrefsManager
import com.kobup.app.data.repository.AppRepository

class AutoUpdateWorker(
    context: Context,
    params: WorkerParameters,
) : CoroutineWorker(context, params) {

    companion object {
        private const val WORK_NAME = "kobup_auto_update"

        // Silent updates need USER_ACTION_NOT_REQUIRED, which arrived in Android 12.
        val supported: Boolean get() = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S

        // Device idle is the quiet-time gate: screen off and untouched for a while.
        fun enqueue(context: Context) {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.UNMETERED)
                .setRequiresDeviceIdle(true)
                .build()
            WorkManager.getInstance(context).enqueueUniqueWork(
                WORK_NAME,
                ExistingWorkPolicy.KEEP,
                OneTimeWorkRequestBuilder<AutoUpdateWorker>().setConstraints(constraints).build(),
            )
        }

        fun cancel(context: Context) {
            WorkManager.getInstance(context).cancelUniqueWork(WORK_NAME)
        }
    }

    override suspend fun doWork(): Result {
        if (!supported || !PrefsManager(applicationContext).getAutoUpdate()) return Result.success()

        val repo = AppRepository(applicationContext)
        val installService = InstallService(applicationContext)
        val self = applicationContext.packageName
        val updates = try {
            autoUpdatable(repo.refreshCatalog(), self, repo::getInstalledVersion, repo::getInstallerPackage)
        } catch (e: Exception) {
            Log.w("Kobup", "Auto-update catalog refresh failed", e)
            return Result.retry()
        }

        var failed = false
        // Replacing our own APK kills this process, so kobup updates itself last.
        updates.sortedBy { it.packageName == self }.forEach { item ->
            val from = repo.getInstalledVersionCode(item.packageName)
            var to: Long? = null
            try {
                val latest = repo.getLatestBySlug(item.slug)
                to = latest.versionCode
                if (from != null && latest.versionCode <= from) return@forEach
                installService.abandonStaleSessions(item.packageName)
                installService.downloadAndInstall(
                    repo = repo,
                    packageName = item.packageName,
                    fromVersionCode = from,
                    toVersionCode = latest.versionCode,
                    artifactId = latest.artifactId,
                    expectedSha256 = latest.sha256,
                    dexMetadata = latest.dexMetadata,
                    onProgress = {},
                    background = true,
                )
                Log.i("Kobup", "Auto-update staged: ${item.packageName} -> ${latest.versionName}")
            } catch (e: Exception) {
                failed = true
                Log.e("Kobup", "Auto-update failed: ${item.packageName}", e)
                report(repo, item.packageName, from, to, "error", e.message)
            }
        }
        return if (failed) Result.retry() else Result.success()
    }

    private suspend fun report(repo: AppRepository, pkg: String, from: Long?, to: Long?, status: String, message: String?) {
        try {
            repo.reportInstallEvent(pkg, from, to, status, message)
        } catch (e: Exception) {
            Log.w("Kobup", "Install event report failed", e)
        }
    }
}
