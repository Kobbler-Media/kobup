package com.kobup.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import com.kobup.app.data.repository.AppRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

/**
 * Handles com.kobup.action.CHECK_UPDATE intents from the Kobup SDK.
 * Triggers an immediate catalog refresh then finishes.
 */
class CheckUpdateActivity : ComponentActivity() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val packageName = intent.getStringExtra("package_name")

        scope.launch {
            try {
                val repo = AppRepository(applicationContext)
                val catalog = repo.refreshCatalog()

                if (packageName != null) {
                    val item = catalog.find { it.packageName == packageName }
                    val installedCode = repo.getInstalledVersionCode(packageName)
                    val latestVersionCode = item?.latestVersion?.versionCode
                    val hasUpdate = item != null &&
                        latestVersionCode != null &&
                        installedCode != null &&
                        latestVersionCode > installedCode

                    setResult(
                        RESULT_OK,
                        android.content.Intent().apply {
                            putExtra("has_update", hasUpdate)
                            putExtra("latest_version", item?.latestVersion?.versionName)
                            putExtra("latest_version_code", latestVersionCode ?: 0L)
                        },
                    )
                } else {
                    setResult(RESULT_OK)
                }
            } catch (_: Exception) {
                setResult(RESULT_CANCELED)
            } finally {
                finish()
            }
        }
    }
}
