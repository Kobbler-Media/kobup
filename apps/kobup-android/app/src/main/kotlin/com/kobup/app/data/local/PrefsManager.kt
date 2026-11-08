package com.kobup.app.data.local

import android.content.Context
import android.content.SharedPreferences
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map

private val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "kobup_prefs")

class PrefsManager(private val context: Context) {

    companion object {
        private val KEY_SERVER_URL = stringPreferencesKey("server_url")
        private val KEY_AUTH_TOKEN = stringPreferencesKey("auth_token")
        private val KEY_DEVICE_ID = stringPreferencesKey("device_id")
        private val KEY_CHANNEL = stringPreferencesKey("channel")
        private val KEY_AUTO_UPDATE = booleanPreferencesKey("auto_update")
        private const val SHARED_PREFS_NAME = "kobup_prefs_sync"
        const val DEFAULT_CHANNEL = "stable"
    }

    private val sharedPrefs: SharedPreferences =
        context.getSharedPreferences(SHARED_PREFS_NAME, Context.MODE_PRIVATE)

    val serverUrl: Flow<String?> = context.dataStore.data.map { it[KEY_SERVER_URL] }
    val authToken: Flow<String?> = context.dataStore.data.map { it[KEY_AUTH_TOKEN] }
    val deviceId: Flow<String?> = context.dataStore.data.map { it[KEY_DEVICE_ID] }
    val channel: Flow<String> = context.dataStore.data.map { it[KEY_CHANNEL] ?: DEFAULT_CHANNEL }
    val autoUpdate: Flow<Boolean> = context.dataStore.data.map { it[KEY_AUTO_UPDATE] ?: true }

    val isSetupDone: Flow<Boolean> = context.dataStore.data.map { prefs ->
        !prefs[KEY_SERVER_URL].isNullOrBlank() && !prefs[KEY_AUTH_TOKEN].isNullOrBlank()
    }

    suspend fun saveServerConfig(url: String, token: String) {
        context.dataStore.edit { prefs ->
            prefs[KEY_SERVER_URL] = url
            prefs[KEY_AUTH_TOKEN] = token
        }
        sharedPrefs.edit()
            .putString("server_url", url)
            .putString("auth_token", token)
            .apply()
    }

    suspend fun saveDeviceId(deviceId: String) {
        context.dataStore.edit { prefs ->
            prefs[KEY_DEVICE_ID] = deviceId
        }
    }

    suspend fun getServerUrl(): String? = context.dataStore.data.first()[KEY_SERVER_URL]
    suspend fun getAuthToken(): String? = context.dataStore.data.first()[KEY_AUTH_TOKEN]
    suspend fun getDeviceId(): String? = context.dataStore.data.first()[KEY_DEVICE_ID]
    suspend fun getChannel(): String = context.dataStore.data.first()[KEY_CHANNEL] ?: DEFAULT_CHANNEL

    suspend fun saveChannel(channel: String) {
        context.dataStore.edit { it[KEY_CHANNEL] = channel }
    }

    suspend fun getAutoUpdate(): Boolean = autoUpdate.first()

    suspend fun saveAutoUpdate(enabled: Boolean) {
        context.dataStore.edit { it[KEY_AUTO_UPDATE] = enabled }
    }

    suspend fun clear() {
        context.dataStore.edit { it.clear() }
        sharedPrefs.edit().clear().apply()
    }

    /** Synchronous access for use from ContentProvider binder threads. */
    fun getServerUrlSync(): String? = sharedPrefs.getString("server_url", null)

    /** Synchronous access for use from ContentProvider binder threads. */
    fun getAuthTokenSync(): String? = sharedPrefs.getString("auth_token", null)
}
