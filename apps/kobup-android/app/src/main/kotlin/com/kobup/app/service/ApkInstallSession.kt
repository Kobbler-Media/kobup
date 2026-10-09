package com.kobup.app.service

import java.io.InputStream
import java.io.OutputStream
import java.security.MessageDigest

interface ApkInstallSession {
    fun openWrite(name: String, offset: Long, length: Long): OutputStream
    fun fsync(output: OutputStream)
    fun commit()
    fun abandon()
    fun close()
}

class DexMetadata(val bytes: ByteArray, val sha256: String)

// The platform finds an APK's dex metadata by swapping `.apk` for `.dm` in its name.
private const val APK_ENTRY = "base.apk"
private const val DM_ENTRY = "base.dm"
private const val BUFFER_SIZE = 8192

private fun ByteArray.hex() = joinToString("") { "%02x".format(it) }

fun writeApkToSession(
    session: ApkInstallSession,
    source: InputStream,
    totalBytes: Long,
    expectedSha256: String?,
    dexMetadata: DexMetadata?,
    onProgress: (Float) -> Unit,
) {
    val digest = MessageDigest.getInstance("SHA-256")
    var written = 0L

    try {
        session.openWrite(APK_ENTRY, 0, totalBytes).use { output ->
            val buffer = ByteArray(BUFFER_SIZE)
            while (true) {
                val read = source.read(buffer)
                if (read == -1) break
                output.write(buffer, 0, read)
                digest.update(buffer, 0, read)
                written += read
                if (totalBytes > 0) onProgress(written.toFloat() / totalBytes)
            }
            session.fsync(output)
        }

        if (expectedSha256 != null) {
            val actual = digest.digest().hex()
            if (!actual.equals(expectedSha256, ignoreCase = true)) {
                throw SecurityException("SHA256 mismatch: expected $expectedSha256 but got $actual")
            }
        }

        if (dexMetadata != null) {
            val actual = MessageDigest.getInstance("SHA-256").digest(dexMetadata.bytes).hex()
            if (!actual.equals(dexMetadata.sha256, ignoreCase = true)) {
                throw SecurityException("Profile SHA256 mismatch: expected ${dexMetadata.sha256} but got $actual")
            }
            session.openWrite(DM_ENTRY, 0, dexMetadata.bytes.size.toLong()).use { output ->
                output.write(dexMetadata.bytes)
                session.fsync(output)
            }
        }

        session.commit()
    } catch (t: Throwable) {
        // An uncommitted session keeps its staged copy of the APK on /data until the
        // installer GCs it days later, which on a small device blocks the retry.
        runCatching { session.abandon() }
        throw t
    } finally {
        runCatching { session.close() }
    }
}
