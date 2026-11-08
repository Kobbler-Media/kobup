package com.kobup.app.data

import com.kobup.app.data.model.CatalogItem
import com.kobup.app.data.model.LatestVersion
import org.junit.Assert.assertEquals
import org.junit.Test

private fun item(pkg: String, versionName: String?, versionCode: Long = 1) = CatalogItem(
    id = pkg,
    slug = pkg,
    name = pkg,
    description = "",
    packageName = pkg,
    latestVersion = versionName?.let { LatestVersion(it, versionCode, "stable") },
)

class UpdateCheckTest {

    @Test
    fun hasUpdateCases() {
        data class Case(val installed: String?, val latest: String?, val expected: Boolean)
        listOf(
            Case(installed = null, latest = "1.0", expected = false),
            Case(installed = "1.0", latest = null, expected = false),
            Case(installed = "1.0", latest = "1.0", expected = false),
            Case(installed = "1.0", latest = "1.1", expected = true),
        ).forEach { c ->
            assertEquals(c.toString(), c.expected, hasUpdate(c.installed, item("a", c.latest).latestVersion))
        }
    }

    @Test
    fun availableUpdatesOnlyIncludesInstalledAppsBehindLatest() {
        val installed = mapOf("a" to "1.0", "b" to "2.0")
        val catalog = listOf(item("a", "1.1"), item("b", "2.0"), item("c", "3.0"))

        val updates = availableUpdates(catalog) { installed[it] }

        assertEquals(listOf("a"), updates.map { it.packageName })
    }

    @Test
    fun unnotifiedSkipsVersionsAlreadyNotified() {
        val updates = listOf(item("a", "1.1", versionCode = 11), item("b", "2.1", versionCode = 21))
        val notified = mapOf("a" to 11L, "b" to 20L)

        assertEquals(listOf("b"), unnotified(updates, notified).map { it.packageName })
    }

    @Test
    fun resolvedNotificationsCases() {
        data class Case(val notified: Long, val installed: Long?, val resolved: Boolean)
        listOf(
            Case(notified = 11, installed = 10, resolved = false),
            Case(notified = 11, installed = 11, resolved = true),
            Case(notified = 11, installed = 12, resolved = true),
            Case(notified = 11, installed = null, resolved = true),
        ).forEach { c ->
            val resolved = resolvedNotifications(mapOf("a" to c.notified)) { c.installed }
            assertEquals(c.toString(), if (c.resolved) setOf("a") else emptySet<String>(), resolved)
        }
    }

    @Test
    fun autoUpdatableOnlyIncludesUpdatesKobupInstalled() {
        val installed = mapOf("a" to "1.0", "b" to "1.0", "c" to "1.0", "d" to "2.0")
        val installers = mapOf("a" to "com.kobup.app", "b" to "com.android.shell", "d" to "com.kobup.app")
        val catalog = listOf(item("a", "1.1"), item("b", "1.1"), item("c", "1.1"), item("d", "2.0"), item("e", "1.0"))

        val updates = autoUpdatable(catalog, "com.kobup.app", { installed[it] }, { installers[it] })

        assertEquals(listOf("a"), updates.map { it.packageName })
    }
}
