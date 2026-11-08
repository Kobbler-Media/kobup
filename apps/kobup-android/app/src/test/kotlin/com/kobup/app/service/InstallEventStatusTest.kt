package com.kobup.app.service

import android.content.pm.PackageInstaller
import org.junit.Assert.assertEquals
import org.junit.Test

class InstallEventStatusTest {

    @Test
    fun mapsInstallerStatusToReportedStatus() {
        listOf(
            PackageInstaller.STATUS_PENDING_USER_ACTION to null,
            PackageInstaller.STATUS_SUCCESS to "success",
            PackageInstaller.STATUS_FAILURE_ABORTED to "aborted",
            PackageInstaller.STATUS_FAILURE_TIMEOUT to "timeout",
            PackageInstaller.STATUS_FAILURE_CONFLICT to "error",
            PackageInstaller.STATUS_FAILURE to "error",
        ).forEach { (status, expected) ->
            assertEquals("status $status", expected, installEventStatus(status))
        }
    }
}
