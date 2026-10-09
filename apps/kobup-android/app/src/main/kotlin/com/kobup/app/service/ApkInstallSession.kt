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

private const val ENTRY_NAME = "kobup_install"
private const val BUFFER_SIZE = 8192

fun writeApkToSession(
    session: ApkInstallSession,
    source: InputStream,
    totalBytes: Long,
    expectedSha256: String?,
    onProgress: (Float) -> Unit,
) {
    val digest = MessageDigest.getInstance("SHA-256")
    var written = 0L

    try {
        session.openWrite(ENTRY_NAME, 0, totalBytes).use { output ->
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
            val actual = digest.digest().joinToString("") { "%02x".format(it) }
            if (!actual.equals(expectedSha256, ignoreCase = true)) {
                throw SecurityException("SHA256 mismatch: expected $expectedSha256 but got $actual")
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
