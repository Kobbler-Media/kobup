package com.kobup.app.service

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.io.OutputStream
import java.security.MessageDigest

private val APK = ByteArray(20_000) { (it % 251).toByte() }

private fun sha256(bytes: ByteArray) =
    MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }

private val PROFILE = ByteArray(300) { (it % 7).toByte() }

private class FakeSession(private val bytesBeforeFailure: Int = -1) : ApkInstallSession {
    val entries = linkedMapOf<String, ByteArrayOutputStream>()
    val staged get() = entries.values.first()
    var committed = false
    var abandoned = false
    var closed = false

    override fun openWrite(name: String, offset: Long, length: Long): OutputStream =
        if (bytesBeforeFailure < 0) ByteArrayOutputStream().also { entries[name] = it } else FailingStream(bytesBeforeFailure)

    override fun fsync(output: OutputStream) = Unit
    override fun commit() { committed = true }
    override fun abandon() { abandoned = true }
    override fun close() { closed = true }

    private class FailingStream(private val limit: Int) : OutputStream() {
        private var count = 0
        override fun write(b: Int) = write(byteArrayOf(b.toByte()), 0, 1)
        override fun write(b: ByteArray, off: Int, len: Int) {
            count += len
            if (count > limit) throw IOException("ENOSPC: No space left on device")
        }
    }
}

private fun run(session: FakeSession, sha: String?, apk: ByteArray = APK, profile: DexMetadata? = null) =
    runCatching {
        writeApkToSession(session, ByteArrayInputStream(apk), apk.size.toLong(), sha, profile) {}
    }

class ApkInstallSessionTest {

    @Test
    fun `commits and never abandons on success`() {
        val session = FakeSession()
        val result = run(session, sha256(APK))

        assertTrue(result.isSuccess)
        assertTrue(session.committed)
        assertFalse(session.abandoned)
        assertTrue(session.closed)
        assertArrayEquals(APK, session.staged.toByteArray())
    }

    @Test
    fun `reports progress up to completion`() {
        val seen = mutableListOf<Float>()
        writeApkToSession(FakeSession(), ByteArrayInputStream(APK), APK.size.toLong(), null, null) { seen += it }

        assertEquals(1f, seen.last(), 0.0001f)
    }

    @Test
    fun `abandons the session when the write runs out of space`() {
        val session = FakeSession(bytesBeforeFailure = 4096)
        val result = run(session, sha256(APK))

        assertTrue(result.exceptionOrNull() is IOException)
        assertFalse(session.committed)
        assertTrue("session leaked after write failure", session.abandoned)
    }

    @Test
    fun `abandons the session when the hash does not match`() {
        val session = FakeSession()
        val result = run(session, sha256(byteArrayOf(1, 2, 3)))

        assertTrue(result.exceptionOrNull() is SecurityException)
        assertFalse(session.committed)
        assertTrue("session leaked after hash mismatch", session.abandoned)
    }

    @Test
    fun `abandons the session when the download stream dies mid-transfer`() {
        val session = FakeSession()
        val truncated = object : ByteArrayInputStream(APK) {
            override fun read(b: ByteArray, off: Int, len: Int): Int =
                if (pos > 8192) throw IOException("connection reset") else super.read(b, off, len)
        }
        val result = runCatching {
            writeApkToSession(session, truncated, APK.size.toLong(), null, null) {}
        }

        assertTrue(result.exceptionOrNull() is IOException)
        assertFalse(session.committed)
        assertTrue("session leaked after download failure", session.abandoned)
    }

    // The platform pairs a .dm with the APK of the same name; any other name and the
    // session fails validation or the profile is ignored.
    @Test
    fun `stages the profile beside the apk under the matching name`() {
        val session = FakeSession()
        val result = run(session, sha256(APK), profile = DexMetadata(PROFILE, sha256(PROFILE)))

        assertTrue(result.isSuccess)
        assertEquals(listOf("base.apk", "base.dm"), session.entries.keys.toList())
        assertArrayEquals(APK, session.entries.getValue("base.apk").toByteArray())
        assertArrayEquals(PROFILE, session.entries.getValue("base.dm").toByteArray())
        assertTrue(session.committed)
    }

    @Test
    fun `abandons the session when the profile hash does not match`() {
        val session = FakeSession()
        val result = run(session, sha256(APK), profile = DexMetadata(PROFILE, sha256(APK)))

        assertTrue(result.exceptionOrNull() is SecurityException)
        assertFalse(session.committed)
        assertTrue("session leaked after profile hash mismatch", session.abandoned)
    }
}
