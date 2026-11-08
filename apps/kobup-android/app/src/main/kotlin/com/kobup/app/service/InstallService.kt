package com.kobup.app.service

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.os.Build
import android.util.Log
import com.kobup.app.data.model.DexMetadataInfo
import com.kobup.app.data.repository.AppRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.OutputStream
import java.util.concurrent.TimeUnit

private val GENTLE_UPDATE_TIMEOUT_MS = TimeUnit.DAYS.toMillis(1)

class InstallService(private val context: Context) {

    suspend fun downloadAndInstall(
        repo: AppRepository,
        packageName: String,
        fromVersionCode: Long?,
        toVersionCode: Long,
        artifactId: String,
        expectedSha256: String?,
        dexMetadata: DexMetadataInfo?,
        onProgress: (Float) -> Unit,
        background: Boolean = false,
    ) = withContext(Dispatchers.IO) {
        // The profile only spares ART the JIT warm-up, so an install goes ahead without it.
        val profile = dexMetadata?.let { dm ->
            runCatching { DexMetadata(repo.downloadDexMetadata(artifactId, dm.minApi).bytes(), dm.sha256) }
                .onFailure { Log.w("Kobup", "installing without the baseline profile", it) }
                .getOrNull()
        }
        val body = repo.downloadArtifact(artifactId)
        val totalBytes = body.contentLength()

        val installer = context.packageManager.packageInstaller
        val params = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL)
        if (totalBytes > 0) params.setSize(totalBytes)
        params.setAppPackageName(packageName)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            params.setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED)
        }
        val sessionId = installer.createSession(params)
        val statusReceiver = commitIntent(sessionId, packageName, fromVersionCode, toVersionCode, background).intentSender
        val commit: (PackageInstaller.Session) -> Unit =
            if (background && Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                { _ ->
                    installer.commitSessionAfterInstallConstraintsAreMet(
                        sessionId,
                        statusReceiver,
                        PackageInstaller.InstallConstraints.GENTLE_UPDATE,
                        GENTLE_UPDATE_TIMEOUT_MS,
                    )
                }
            } else {
                { it.commit(statusReceiver) }
            }

        body.byteStream().use { input ->
            writeApkToSession(
                session = PlatformInstallSession(installer.openSession(sessionId), commit),
                source = input,
                totalBytes = totalBytes,
                expectedSha256 = expectedSha256,
                dexMetadata = profile,
                onProgress = onProgress,
            )
        }
    }

    // A session left waiting on the user or on install constraints keeps its staged APK on /data,
    // so a retry replaces it rather than piling up another copy.
    fun abandonStaleSessions(packageName: String) {
        val installer = context.packageManager.packageInstaller
        installer.mySessions
            .filter { it.appPackageName == packageName }
            .forEach { runCatching { installer.abandonSession(it.sessionId) } }
    }

    private fun commitIntent(
        sessionId: Int,
        packageName: String,
        fromVersionCode: Long?,
        toVersionCode: Long,
        background: Boolean,
    ): PendingIntent {
        val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        } else {
            PendingIntent.FLAG_UPDATE_CURRENT
        }
        val intent = Intent(context, InstallResultReceiver::class.java)
            .putExtra(InstallResultReceiver.EXTRA_BACKGROUND, background)
            .putExtra(InstallResultReceiver.EXTRA_PACKAGE, packageName)
            .putExtra(InstallResultReceiver.EXTRA_FROM_VERSION, fromVersionCode ?: -1)
            .putExtra(InstallResultReceiver.EXTRA_TO_VERSION, toVersionCode)
        return PendingIntent.getBroadcast(context, sessionId, intent, flags)
    }
}

private class PlatformInstallSession(
    private val session: PackageInstaller.Session,
    private val commit: (PackageInstaller.Session) -> Unit,
) : ApkInstallSession {
    override fun openWrite(name: String, offset: Long, length: Long): OutputStream =
        session.openWrite(name, offset, length)

    override fun fsync(output: OutputStream) = session.fsync(output)
    override fun commit() = commit(session)
    override fun abandon() = session.abandon()
    override fun close() = session.close()
}
