package com.kobup.app.service

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

data class InstallResult(
    val packageName: String?,
    val status: Int,
    val message: String?,
    val timestamp: Long = System.currentTimeMillis(),
)

class InstallResultReceiver : BroadcastReceiver() {

    companion object {
        private val _lastResult = MutableStateFlow<InstallResult?>(null)
        val lastResult: StateFlow<InstallResult?> = _lastResult.asStateFlow()

        fun clear() {
            _lastResult.value = null
        }
    }

    override fun onReceive(context: Context, intent: Intent) {
        val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)
        val message = intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE)
        val packageName = intent.getStringExtra(PackageInstaller.EXTRA_PACKAGE_NAME)

        when (status) {
            PackageInstaller.STATUS_PENDING_USER_ACTION -> {
                @Suppress("DEPRECATION")
                val confirmIntent = intent.getParcelableExtra<Intent>(Intent.EXTRA_INTENT)
                if (confirmIntent != null) {
                    confirmIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    context.startActivity(confirmIntent)
                }
            }
            PackageInstaller.STATUS_SUCCESS -> {
                Log.i("Kobup", "Install succeeded: $packageName")
                _lastResult.value = InstallResult(packageName, status, null)
            }
            else -> {
                Log.e("Kobup", "Install failed: status=$status message=$message")
                _lastResult.value = InstallResult(packageName, status, message)
            }
        }
    }
}
