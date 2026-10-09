package com.kobup.sdk

sealed class KobupUpdateStatus {
    data class Available(val versionName: String, val versionCode: Long) : KobupUpdateStatus()
    data object UpToDate : KobupUpdateStatus()
    data object Unknown : KobupUpdateStatus()
    data object KobupNotInstalled : KobupUpdateStatus()
}
