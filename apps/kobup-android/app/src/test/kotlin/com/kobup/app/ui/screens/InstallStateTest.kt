package com.kobup.app.ui.screens

import org.junit.Assert.assertEquals
import org.junit.Test

class InstallStateTest {

    @Test
    fun finishedStatesGiveWayWhenANewerVersionArrives() {
        data class Case(val state: InstallState, val attempted: Long?, val latest: Long?, val expected: InstallState)
        listOf(
            Case(InstallState.INSTALLED, attempted = 10, latest = 10, expected = InstallState.INSTALLED),
            Case(InstallState.INSTALLED, attempted = 10, latest = 11, expected = InstallState.IDLE),
            Case(InstallState.ERROR, attempted = 10, latest = 10, expected = InstallState.ERROR),
            Case(InstallState.ERROR, attempted = 10, latest = 11, expected = InstallState.IDLE),
            Case(InstallState.DOWNLOADING, attempted = 10, latest = 11, expected = InstallState.DOWNLOADING),
            Case(InstallState.SUCCESS, attempted = 10, latest = 11, expected = InstallState.SUCCESS),
            Case(InstallState.IDLE, attempted = null, latest = 11, expected = InstallState.IDLE),
        ).forEach { c ->
            assertEquals(c.toString(), c.expected, visibleInstallState(c.state, c.attempted, c.latest))
        }
    }
}
