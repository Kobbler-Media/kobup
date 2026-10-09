package com.kobup.app.data.api

import com.kobup.app.data.model.CatalogItem
import com.kobup.app.data.model.DeviceRegisterRequest
import com.kobup.app.data.model.DeviceRegisterResponse
import com.kobup.app.data.model.InstallEventBody
import com.kobup.app.data.model.LatestBySlugResponse
import okhttp3.ResponseBody
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.Path
import retrofit2.http.Query
import retrofit2.http.Streaming

interface KobupApi {

    @GET("api/v1/catalog")
    suspend fun getCatalog(
        @Query("channel") channel: String? = null,
    ): List<CatalogItem>

    @GET("api/v1/projects/{slug}/latest")
    suspend fun getLatestBySlug(
        @Path("slug") slug: String,
        @Query("channel") channel: String? = null,
        @Query("sdkInt") sdkInt: Int? = null,
        @Query("abis") abis: String? = null,
    ): LatestBySlugResponse

    @POST("api/v1/devices/register")
    suspend fun registerDevice(@Body request: DeviceRegisterRequest): DeviceRegisterResponse

    @POST("api/v1/devices/{deviceId}/push-token")
    suspend fun registerPushToken(
        @Path("deviceId") deviceId: String,
        @Body body: Map<String, String>,
    )

    @POST("api/v1/devices/{deviceId}/heartbeat")
    suspend fun heartbeat(
        @Path("deviceId") deviceId: String,
        @Body body: Map<String, String?>,
    )

    @Streaming
    @GET("api/v1/artifacts/{artifactId}/download")
    suspend fun downloadArtifact(@Path("artifactId") artifactId: String): ResponseBody

    @POST("api/v1/devices/{deviceId}/install-event")
    suspend fun reportInstallEvent(
        @Path("deviceId") deviceId: String,
        @Body body: InstallEventBody,
    )
}
