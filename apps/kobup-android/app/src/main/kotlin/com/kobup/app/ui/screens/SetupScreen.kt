package com.kobup.app.ui.screens

import android.content.Intent
import android.os.Build
import android.provider.Settings
import android.util.Log
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kobup.app.ui.components.requestFocusSafely
import com.kobup.app.ui.components.tvFocusable
import com.kobup.app.data.local.PrefsManager
import com.kobup.app.data.repository.AppRepository
import com.kobup.app.service.PinService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext

enum class SetupStep {
    CREATING_PIN,
    SHOWING_PIN,
    WAITING_FOR_CLAIM,
    REGISTERING_DEVICE,
    CHECK_PERMISSIONS,
    DONE,
    ERROR,
}

@Composable
fun SetupScreen(
    isTv: Boolean,
    onSetupComplete: () -> Unit,
) {
    val context = LocalContext.current
    val prefs = remember { PrefsManager(context) }
    val repo = remember { AppRepository(context) }
    val pinService = remember { PinService() }

    var step by remember { mutableStateOf(SetupStep.CREATING_PIN) }
    var pin by remember { mutableStateOf("") }
    var errorMessage by remember { mutableStateOf("") }
    var attempt by remember { mutableIntStateOf(0) }

    val actionFocusRequester = remember { FocusRequester() }

    LaunchedEffect(isTv, step) {
        if (!isTv) return@LaunchedEffect
        if (step != SetupStep.CHECK_PERMISSIONS && step != SetupStep.ERROR) return@LaunchedEffect
        actionFocusRequester.requestFocusSafely()
    }

    // PIN creation + polling + registration — one continuous flow
    // Restarts when `attempt` changes (i.e. user taps Retry)
    LaunchedEffect(attempt) {
        try {
            val createdPin = withContext(Dispatchers.IO) { pinService.createPin() }
            pin = createdPin
            step = SetupStep.SHOWING_PIN

            // Poll until claimed
            while (true) {
                delay(2000)
                val result = withContext(Dispatchers.IO) { pinService.pollPin(pin) }
                if (result.pin != pin) {
                    pin = result.pin
                }
                if (result.apiUrl != null && result.token != null) {
                    prefs.saveServerConfig(result.apiUrl, result.token)
                    repo.resetApi()
                    step = SetupStep.REGISTERING_DEVICE
                    break
                }
            }

            // Register device
            val deviceName = Build.MODEL
            repo.registerDevice(deviceName, isTv)
            try {
                repo.syncPushToken()
            } catch (e: Exception) {
                Log.w("Kobup", "Push token registration failed", e)
            }
            step = if (needsInstallPermission()) {
                SetupStep.CHECK_PERMISSIONS
            } else {
                SetupStep.DONE
            }
        } catch (e: Exception) {
            errorMessage = e.message ?: "Setup failed"
            step = SetupStep.ERROR
        }
    }

    // Handle DONE and retry separately
    LaunchedEffect(step) {
        if (step == SetupStep.DONE) {
            onSetupComplete()
        }
    }

    Surface(modifier = Modifier.fillMaxSize()) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(32.dp),
            contentAlignment = Alignment.Center,
        ) {
            when (step) {
                SetupStep.CREATING_PIN -> {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        CircularProgressIndicator()
                        Spacer(Modifier.height(16.dp))
                        Text("Creating PIN...")
                    }
                }

                SetupStep.SHOWING_PIN, SetupStep.WAITING_FOR_CLAIM -> {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center,
                    ) {
                        Text(
                            text = "Setup",
                            style = if (isTv) MaterialTheme.typography.headlineLarge else MaterialTheme.typography.headlineMedium,
                        )
                        Spacer(Modifier.height(if (isTv) 32.dp else 24.dp))
                        Text(
                            text = "Enter this PIN in the Kobup web admin:",
                            style = if (isTv) MaterialTheme.typography.headlineSmall else MaterialTheme.typography.bodyLarge,
                            textAlign = TextAlign.Center,
                        )
                        Spacer(Modifier.height(if (isTv) 32.dp else 24.dp))
                        Text(
                            text = pin,
                            // Sized so a 10-character pin still fits a phone;
                            // pin length is server-side config, not a constant.
                            fontSize = if (isTv) 44.sp else 30.sp,
                            fontWeight = FontWeight.Bold,
                            fontFamily = FontFamily.Monospace,
                            letterSpacing = 4.sp,
                            maxLines = 1,
                            color = MaterialTheme.colorScheme.primary,
                        )
                        Spacer(Modifier.height(32.dp))
                        CircularProgressIndicator(modifier = Modifier.size(24.dp))
                        Spacer(Modifier.height(8.dp))
                        Text(
                            text = "Waiting for confirmation...",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }

                SetupStep.REGISTERING_DEVICE -> {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        CircularProgressIndicator()
                        Spacer(Modifier.height(16.dp))
                        Text("Registering device...")
                    }
                }

                SetupStep.CHECK_PERMISSIONS -> {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Text(
                            text = "Permission Required",
                            style = if (isTv) MaterialTheme.typography.headlineMedium else MaterialTheme.typography.headlineSmall,
                        )
                        Spacer(Modifier.height(if (isTv) 24.dp else 16.dp))
                        Text(
                            text = "Kobup needs permission to install apps. Please enable \"Install unknown apps\" for Kobup in your device settings.",
                            style = if (isTv) MaterialTheme.typography.bodyLarge else MaterialTheme.typography.bodyLarge,
                            textAlign = TextAlign.Center,
                        )
                        Spacer(Modifier.height(24.dp))
                        Button(
                            onClick = {
                                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                                    val intent = Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES).apply {
                                        data = android.net.Uri.parse("package:${context.packageName}")
                                    }
                                    context.startActivity(intent)
                                }
                            },
                            modifier = Modifier
                                .heightIn(min = if (isTv) 56.dp else 48.dp)
                                .focusRequester(actionFocusRequester)
                                .then(if (isTv) Modifier.tvFocusable() else Modifier),
                        ) {
                            Text("Open Settings")
                        }
                        Spacer(Modifier.height(16.dp))
                        Button(
                            onClick = { step = SetupStep.DONE },
                            modifier = Modifier
                                .heightIn(min = if (isTv) 56.dp else 48.dp)
                                .then(if (isTv) Modifier.tvFocusable() else Modifier),
                        ) {
                            Text("Continue Anyway")
                        }
                    }
                }

                SetupStep.ERROR -> {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center,
                    ) {
                        Text(
                            text = "Setup Error",
                            style = if (isTv) MaterialTheme.typography.headlineMedium else MaterialTheme.typography.headlineSmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                        Spacer(Modifier.height(if (isTv) 24.dp else 16.dp))
                        Text(
                            text = errorMessage,
                            style = MaterialTheme.typography.bodyLarge,
                            textAlign = TextAlign.Center,
                        )
                        Spacer(Modifier.height(24.dp))
                        Button(
                            onClick = {
                                step = SetupStep.CREATING_PIN
                                errorMessage = ""
                                attempt++
                            },
                            modifier = Modifier
                                .heightIn(min = if (isTv) 56.dp else 48.dp)
                                .focusRequester(actionFocusRequester)
                                .then(if (isTv) Modifier.tvFocusable() else Modifier),
                        ) {
                            Text("Retry")
                        }
                    }
                }

                SetupStep.DONE -> {
                    CircularProgressIndicator()
                }
            }
        }
    }
}

private fun needsInstallPermission(): Boolean {
    return Build.VERSION.SDK_INT >= Build.VERSION_CODES.O
}
