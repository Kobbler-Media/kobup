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

class InstallService(private val context: Context) {

    suspend fun downloadAndInstall(
        repo: AppRepository,
        artifactId: String,
        expectedSha256: String?,
        dexMetadata: DexMetadataInfo?,
        onProgress: (Float) -> Unit,
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
        val sessionId = installer.createSession(params)

        body.byteStream().use { input ->
            writeApkToSession(
                session = PlatformInstallSession(installer.openSession(sessionId), commitIntent(sessionId)),
                source = input,
                totalBytes = totalBytes,
                expectedSha256 = expectedSha256,
                dexMetadata = profile,
                onProgress = onProgress,
            )
        }
    }

    private fun commitIntent(sessionId: Int): PendingIntent {
        val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        } else {
            PendingIntent.FLAG_UPDATE_CURRENT
        }
        val intent = Intent(context, InstallResultReceiver::class.java)
        return PendingIntent.getBroadcast(context, sessionId, intent, flags)
    }
}

private class PlatformInstallSession(
    private val session: PackageInstaller.Session,
    private val commitIntent: PendingIntent,
) : ApkInstallSession {
    override fun openWrite(name: String, offset: Long, length: Long): OutputStream =
        session.openWrite(name, offset, length)

    override fun fsync(output: OutputStream) = session.fsync(output)
    override fun commit() = session.commit(commitIntent.intentSender)
    override fun abandon() = session.abandon()
    override fun close() = session.close()
}
