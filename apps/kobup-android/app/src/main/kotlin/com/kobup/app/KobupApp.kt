package com.kobup.app

import android.app.Application
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader
import coil3.network.okhttp.OkHttpNetworkFetcherFactory
import com.kobup.app.service.PackageChangeReceiver
import com.kobup.app.service.SyncWorker
import java.util.concurrent.TimeUnit

class KobupApp : Application(), SingletonImageLoader.Factory {

    override fun onCreate() {
        super.onCreate()
        PackageChangeReceiver.register(this)
        PackageChangeReceiver.dismissResolved(this)
        scheduleSync()
    }

    override fun newImageLoader(context: PlatformContext): ImageLoader =
        ImageLoader.Builder(context)
            .components { add(OkHttpNetworkFetcherFactory()) }
            .build()

    private fun scheduleSync() {
        val constraints = Constraints.Builder()
            .setRequiredNetworkType(NetworkType.CONNECTED)
            .build()

        val syncRequest = PeriodicWorkRequestBuilder<SyncWorker>(6, TimeUnit.HOURS)
            .setConstraints(constraints)
            .build()

        WorkManager.getInstance(this).enqueueUniquePeriodicWork(
            "kobup_sync",
            ExistingPeriodicWorkPolicy.KEEP,
            syncRequest,
        )

        // Push tokens only reach the server via sync, so run one per launch rather than
        // waiting up to six hours for the periodic job.
        WorkManager.getInstance(this).enqueueUniqueWork(
            "kobup_sync_launch",
            ExistingWorkPolicy.KEEP,
            OneTimeWorkRequestBuilder<SyncWorker>().setConstraints(constraints).build(),
        )
    }
}
