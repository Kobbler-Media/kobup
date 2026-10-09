package com.kobup.app.data.model

import kotlinx.serialization.Serializable

@Serializable
data class LatestVersion(
    val versionName: String,
    val versionCode: Long,
    val channel: String,
)

@Serializable
data class CatalogItem(
    val id: String,
    val slug: String,
    val name: String,
    val description: String,
    val packageName: String,
    val latestVersion: LatestVersion? = null,
    val artifactId: String? = null,
    val downloadUrl: String? = null,
    val sha256: String? = null,
    val sizeBytes: Long? = null,
    val iconUrl: String? = null,
)

@Serializable
data class DeviceRegisterRequest(
    val deviceName: String,
    val sdkInt: Int,
    val abis: List<String>,
    val isTv: Boolean,
    val kobupVersion: String,
)

@Serializable
data class DeviceRegisterResponse(
    val deviceId: String,
)

@Serializable
data class LatestBySlugResponse(
    val projectSlug: String,
    val versionName: String,
    val versionCode: Long,
    val artifactId: String,
    val downloadUrl: String,
    val sha256: String,
    val sizeBytes: Long,
    val minSdk: Long? = null,
    val dexMetadata: DexMetadataInfo? = null,
    val reason: String? = null,
)

@Serializable
data class DexMetadataInfo(
    val minApi: Int,
    val sha256: String,
    val sizeBytes: Long,
)

@Serializable
data class InstallEventBody(
    val packageName: String,
    val fromVersionCode: Long? = null,
    val toVersionCode: Long? = null,
    val status: String,
    val message: String? = null,
)
