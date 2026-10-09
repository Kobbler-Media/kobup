package com.kobup.app.service

import android.util.Log
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import com.kobup.app.data.local.PrefsManager
import com.kobup.app.data.repository.AppRepository
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

class FcmService : FirebaseMessagingService() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onNewToken(token: String) {
        Log.i("Kobup", "New FCM token: ${token.take(10)}...")
        scope.launch {
            try {
                val prefs = PrefsManager(applicationContext)
                if (prefs.getServerUrl().isNullOrBlank()) {
                    Log.d("Kobup", "Skipping FCM token registration — not configured yet")
                    return@launch
                }
                val repo = AppRepository(applicationContext)
                repo.registerPushToken(token)
            } catch (e: Exception) {
                Log.e("Kobup", "Failed to register push token", e)
            }
        }
    }

    override fun onMessageReceived(message: RemoteMessage) {
        Log.i("Kobup", "FCM message received: ${message.data}")

        val type = message.data["type"]
        when (type) {
            "refresh", "sync", "update_available" -> {
                val workRequest = OneTimeWorkRequestBuilder<SyncWorker>().build()
                WorkManager.getInstance(applicationContext).enqueue(workRequest)
            }
            else -> {
                Log.w("Kobup", "Unknown FCM message type: $type")
            }
        }
    }
}
