package com.kobup.app.service

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.util.Log
import com.kobup.app.data.repository.AppRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

data class InstallResult(
    val packageName: String?,
    val status: Int,
    val message: String?,
    val timestamp: Long = System.currentTimeMillis(),
)

fun installEventStatus(status: Int): String? = when (status) {
    PackageInstaller.STATUS_PENDING_USER_ACTION -> null
    PackageInstaller.STATUS_SUCCESS -> "success"
    PackageInstaller.STATUS_FAILURE_ABORTED -> "aborted"
    PackageInstaller.STATUS_FAILURE_TIMEOUT -> "timeout"
    else -> "error"
}

class InstallResultReceiver : BroadcastReceiver() {

    companion object {
        const val EXTRA_BACKGROUND = "com.kobup.app.extra.BACKGROUND"
        const val EXTRA_PACKAGE = "com.kobup.app.extra.PACKAGE"
        const val EXTRA_FROM_VERSION = "com.kobup.app.extra.FROM_VERSION"
        const val EXTRA_TO_VERSION = "com.kobup.app.extra.TO_VERSION"

        private val _lastResult = MutableStateFlow<InstallResult?>(null)
        val lastResult: StateFlow<InstallResult?> = _lastResult.asStateFlow()

        fun clear() {
            _lastResult.value = null
        }
    }

    override fun onReceive(context: Context, intent: Intent) {
        val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)
        val message = intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE)
        val packageName = intent.getStringExtra(EXTRA_PACKAGE)
        report(context, intent, status, message)

        when (status) {
            PackageInstaller.STATUS_PENDING_USER_ACTION -> {
                @Suppress("DEPRECATION")
                val confirmIntent = intent.getParcelableExtra<Intent>(Intent.EXTRA_INTENT)
                if (confirmIntent != null) {
                    confirmIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    // Nobody is looking at kobup during an auto-update, and Android blocks
                    // background activity starts anyway, so ask via notification instead.
                    if (intent.getBooleanExtra(EXTRA_BACKGROUND, false)) {
                        UpdateNotifier.needsConfirmation(context, packageName, confirmIntent)
                    } else {
                        context.startActivity(confirmIntent)
                    }
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

    private fun report(context: Context, intent: Intent, status: Int, message: String?) {
        val reported = installEventStatus(status) ?: return
        val packageName = intent.getStringExtra(EXTRA_PACKAGE) ?: return
        val from = intent.getLongExtra(EXTRA_FROM_VERSION, -1).takeIf { it >= 0 }
        val to = intent.getLongExtra(EXTRA_TO_VERSION, -1).takeIf { it >= 0 }
        val detail = if (reported == "success") null else "status=$status ${message.orEmpty()}".trim()
        val pending = goAsync()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                AppRepository(context).reportInstallEvent(packageName, from, to, reported, detail)
            } catch (e: Exception) {
                Log.w("Kobup", "Install event report failed", e)
            } finally {
                pending.finish()
            }
        }
    }
}
