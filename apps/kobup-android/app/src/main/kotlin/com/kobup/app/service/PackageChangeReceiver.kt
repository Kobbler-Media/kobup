package com.kobup.app.service

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import androidx.core.content.ContextCompat
import com.kobup.app.data.repository.AppRepository

class PackageChangeReceiver : BroadcastReceiver() {

    companion object {
        // PACKAGE_ADDED/REPLACED/REMOVED aren't delivered to manifest receivers since API 26,
        // so they only reach us while the process is alive; MY_PACKAGE_REPLACED is in the manifest.
        private val RUNTIME_ACTIONS = listOf(
            Intent.ACTION_PACKAGE_ADDED,
            Intent.ACTION_PACKAGE_REPLACED,
            Intent.ACTION_PACKAGE_REMOVED,
        )

        fun register(context: Context) {
            val filter = IntentFilter().apply {
                RUNTIME_ACTIONS.forEach(::addAction)
                addDataScheme("package")
            }
            ContextCompat.registerReceiver(context, PackageChangeReceiver(), filter, ContextCompat.RECEIVER_NOT_EXPORTED)
        }

        fun dismissResolved(context: Context) {
            UpdateNotifier.dismissResolved(context, AppRepository(context)::getInstalledVersionCode)
        }
    }

    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_MY_PACKAGE_REPLACED && intent.action !in RUNTIME_ACTIONS) return
        dismissResolved(context)
    }
}
