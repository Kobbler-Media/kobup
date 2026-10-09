package com.kobup.app

import android.content.ContentProvider
import android.content.ContentValues
import android.content.UriMatcher
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import com.kobup.app.data.local.PrefsManager
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONArray
import java.util.concurrent.TimeUnit

class UpdateStatusProvider : ContentProvider() {

    companion object {
        const val AUTHORITY = "com.kobup.app.provider"
        private const val UPDATE_STATUS = 1
        private val uriMatcher = UriMatcher(UriMatcher.NO_MATCH).apply {
            addURI(AUTHORITY, "update_status/*", UPDATE_STATUS)
        }
    }

    private val httpClient by lazy {
        OkHttpClient.Builder()
            .connectTimeout(10, TimeUnit.SECONDS)
            .readTimeout(15, TimeUnit.SECONDS)
            .build()
    }

    override fun onCreate(): Boolean = true

    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?,
    ): Cursor? {
        if (uriMatcher.match(uri) != UPDATE_STATUS) return null

        val packageName = uri.lastPathSegment ?: return null
        val ctx = context ?: return null

        val cursor = MatrixCursor(arrayOf("status", "version_name", "version_code"))

        try {
            val prefs = PrefsManager(ctx)
            val serverUrl = prefs.getServerUrlSync() ?: return cursor
            val token = prefs.getAuthTokenSync() ?: return cursor

            val response = httpClient.newCall(
                Request.Builder()
                    .url(serverUrl.trimEnd('/') + "/api/v1/catalog")
                    .header("Authorization", "Bearer $token")
                    .build()
            ).execute()

            if (response.isSuccessful) {
                val body = response.body.string()
                val catalog = JSONArray(body)

                for (i in 0 until catalog.length()) {
                    val item = catalog.getJSONObject(i)
                    if (item.optString("packageName") != packageName) continue

                    val latestVersion = item.optJSONObject("latestVersion") ?: continue
                    val serverVersionCode = latestVersion.optLong("versionCode", -1)
                    val serverVersionName = latestVersion.optString("versionName", "")

                    if (serverVersionCode < 0) break

                    val installedVersionCode = try {
                        val info = ctx.packageManager.getPackageInfo(packageName, 0)
                        if (android.os.Build.VERSION.SDK_INT >= 28) {
                            info.longVersionCode
                        } else {
                            @Suppress("DEPRECATION")
                            info.versionCode.toLong()
                        }
                    } catch (_: Exception) {
                        -1L
                    }

                    if (serverVersionCode > installedVersionCode) {
                        cursor.addRow(arrayOf("available", serverVersionName, serverVersionCode))
                    } else {
                        cursor.addRow(arrayOf("up_to_date", serverVersionName, serverVersionCode))
                    }
                    break
                }
            }
        } catch (_: Exception) {
            // Return empty cursor on any error
        }

        return cursor
    }

    override fun getType(uri: Uri): String? = null
    override fun insert(uri: Uri, values: ContentValues?): Uri? = null
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int = 0
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?): Int = 0
}
