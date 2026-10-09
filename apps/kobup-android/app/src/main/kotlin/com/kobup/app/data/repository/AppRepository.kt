package com.kobup.app.data.repository

import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import com.kobup.app.data.api.ApiFactory
import com.kobup.app.data.api.KobupApi
import com.kobup.app.data.local.PrefsManager
import com.kobup.app.data.model.CatalogItem
import com.kobup.app.data.model.DeviceRegisterRequest
import com.kobup.app.data.model.InstallEventBody
import com.kobup.app.data.model.LatestBySlugResponse
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

class AppRepository(private val context: Context) {

    private val prefs = PrefsManager(context)
    private var api: KobupApi? = null

    private val _catalog = MutableStateFlow<List<CatalogItem>>(emptyList())
    val catalog: StateFlow<List<CatalogItem>> = _catalog.asStateFlow()

    suspend fun ensureApi(): KobupApi {
        api?.let { return it }
        val url = prefs.getServerUrl() ?: throw IllegalStateException("No server URL configured")
        val token = prefs.getAuthToken() ?: throw IllegalStateException("No auth token configured")
        return ApiFactory.createApi(url, token).also { api = it }
    }

    fun resetApi() {
        api = null
    }

    suspend fun refreshCatalog(): List<CatalogItem> {
        val items = ensureApi().getCatalog(channel = prefs.getChannel())
        _catalog.value = items
        return items
    }

    suspend fun registerDevice(deviceName: String, isTv: Boolean): String {
        val request = DeviceRegisterRequest(
            deviceName = deviceName,
            sdkInt = Build.VERSION.SDK_INT,
            abis = Build.SUPPORTED_ABIS.toList(),
            isTv = isTv,
            kobupVersion = getAppVersion(),
        )
        val response = ensureApi().registerDevice(request)
        prefs.saveDeviceId(response.deviceId)
        return response.deviceId
    }

    suspend fun registerPushToken(fcmToken: String) {
        val deviceId = prefs.getDeviceId() ?: return
        ensureApi().registerPushToken(deviceId, mapOf("provider" to "fcm", "token" to fcmToken))
    }

    suspend fun heartbeat() {
        val deviceId = prefs.getDeviceId() ?: return
        ensureApi().heartbeat(deviceId, emptyMap())
    }

    suspend fun getLatestBySlug(slug: String): LatestBySlugResponse {
        val sdkInt = Build.VERSION.SDK_INT
        val abis = Build.SUPPORTED_ABIS.joinToString(",")
        return ensureApi().getLatestBySlug(slug, channel = prefs.getChannel(), sdkInt = sdkInt, abis = abis)
    }

    suspend fun downloadArtifact(artifactId: String) = ensureApi().downloadArtifact(artifactId)

    suspend fun downloadDexMetadata(artifactId: String, minApi: Int) =
        ensureApi().downloadDexMetadata(artifactId, minApi)

    suspend fun reportInstallEvent(
        packageName: String,
        fromVersionCode: Long?,
        toVersionCode: Long?,
        status: String,
        message: String? = null,
    ) {
        val deviceId = prefs.getDeviceId() ?: return
        ensureApi().reportInstallEvent(
            deviceId,
            InstallEventBody(
                packageName = packageName,
                fromVersionCode = fromVersionCode,
                toVersionCode = toVersionCode,
                status = status,
                message = message,
            ),
        )
    }

    fun getInstalledVersion(packageName: String): String? {
        return try {
            val info = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                context.packageManager.getPackageInfo(packageName, PackageManager.PackageInfoFlags.of(0))
            } else {
                @Suppress("DEPRECATION")
                context.packageManager.getPackageInfo(packageName, 0)
            }
            info.versionName
        } catch (_: PackageManager.NameNotFoundException) {
            null
        }
    }

    fun getLaunchIntent(packageName: String): Intent? =
        context.packageManager.getLaunchIntentForPackage(packageName)

    fun getInstalledVersionCode(packageName: String): Long? {
        return try {
            val info = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                context.packageManager.getPackageInfo(packageName, PackageManager.PackageInfoFlags.of(0))
            } else {
                @Suppress("DEPRECATION")
                context.packageManager.getPackageInfo(packageName, 0)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                info.longVersionCode
            } else {
                @Suppress("DEPRECATION")
                info.versionCode.toLong()
            }
        } catch (_: PackageManager.NameNotFoundException) {
            null
        }
    }

    /**
     * Check if a newer version of Kobup itself is available.
     * Returns the LatestBySlugResponse if an update exists, null otherwise.
     */
    suspend fun checkSelfUpdate(): LatestBySlugResponse? {
        return try {
            val latest = getLatestBySlug("kobup")
            val myVersionCode = getInstalledVersionCode(context.packageName) ?: return null
            if (latest.versionCode > myVersionCode) latest else null
        } catch (_: Exception) {
            null
        }
    }

    private fun getAppVersion(): String {
        return try {
            val info = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                context.packageManager.getPackageInfo(context.packageName, PackageManager.PackageInfoFlags.of(0))
            } else {
                @Suppress("DEPRECATION")
                context.packageManager.getPackageInfo(context.packageName, 0)
            }
            info.versionName ?: "unknown"
        } catch (_: Exception) {
            "unknown"
        }
    }
}
