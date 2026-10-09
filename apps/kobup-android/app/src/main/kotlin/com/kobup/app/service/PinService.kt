package com.kobup.app.service

import com.kobup.app.BuildConfig
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.concurrent.TimeUnit

class PinService {

    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .build()

    fun createPin(): String {
        val url = "${BuildConfig.PIN_SERVICE_URL}/pin/${BuildConfig.PIN_NAMESPACE}"
        val request = Request.Builder()
            .url(url)
            .post("".toRequestBody(null))
            .build()
        client.newCall(request).execute().use { response ->
            if (!response.isSuccessful) throw PinException("Failed to create PIN: ${response.code}")
            val body = response.body.string()
            val obj = JSONObject(body)
            return obj.getString("pin")
        }
    }

    data class PollResult(
        val pin: String,
        val apiUrl: String?,
        val token: String?,
    )

    fun pollPin(currentPin: String): PollResult {
        val url = "${BuildConfig.PIN_SERVICE_URL}/pin/${BuildConfig.PIN_NAMESPACE}/$currentPin"
        val request = Request.Builder()
            .url(url)
            .post("".toRequestBody(null))
            .build()
        client.newCall(request).execute().use { response ->
            // The service reports an expired or already-consumed PIN as 404 and
            // no longer mints a replacement, so allocate one. Returning it under
            // a changed `pin` keeps the caller's existing rotate-and-redisplay
            // path working.
            if (response.code == 404) {
                return PollResult(pin = createPin(), apiUrl = null, token = null)
            }
            if (!response.isSuccessful) throw PinException("Poll failed: ${response.code}")
            val body = response.body.string()
            val obj = JSONObject(body)
            val pin = obj.getString("pin")
            val result = if (obj.isNull("result")) null else obj.getJSONObject("result")
            return PollResult(
                pin = pin,
                apiUrl = result?.optString("api_url"),
                token = result?.optString("token"),
            )
        }
    }
}

class PinException(message: String) : Exception(message)
