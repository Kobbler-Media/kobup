package com.kobup.app.ui.screens

import android.content.pm.PackageManager
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.kobup.app.data.local.PrefsManager
import com.kobup.app.service.AutoUpdateWorker
import com.kobup.app.ui.components.requestFocusSafely
import com.kobup.app.ui.components.tvFocusable
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    onBack: () -> Unit,
    onLogout: () -> Unit,
) {
    val context = LocalContext.current
    val isTv = remember {
        context.packageManager.hasSystemFeature(PackageManager.FEATURE_LEANBACK)
    }
    val prefs = remember { PrefsManager(context) }
    val scope = rememberCoroutineScope()

    val serverUrl by prefs.serverUrl.collectAsState(initial = "")
    val deviceId by prefs.deviceId.collectAsState(initial = "")
    val channel by prefs.channel.collectAsState(initial = PrefsManager.DEFAULT_CHANNEL)
    val autoUpdate by prefs.autoUpdate.collectAsState(initial = true)

    var showChannelDialog by remember { mutableStateOf(false) }
    var channelInput by remember { mutableStateOf("") }

    val channelCardFocusRequester = remember { FocusRequester() }
    val channelFieldFocusRequester = remember { FocusRequester() }

    LaunchedEffect(isTv) {
        if (isTv) channelCardFocusRequester.requestFocusSafely()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings") },
                navigationIcon = {
                    IconButton(
                        onClick = onBack,
                        modifier = if (isTv) Modifier.tvFocusable(shape = CircleShape) else Modifier,
                    ) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(if (isTv) 32.dp else 24.dp),
        ) {
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(if (isTv) 20.dp else 16.dp)) {
                    Text(
                        text = "Connection",
                        style = if (isTv) MaterialTheme.typography.titleLarge else MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                    )
                    Spacer(Modifier.height(12.dp))
                    SettingsRow("Server", serverUrl ?: "Not configured")
                    Spacer(Modifier.height(8.dp))
                    SettingsRow("Device ID", deviceId ?: "Not registered")
                }
            }

            Spacer(Modifier.height(24.dp))

            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .focusRequester(channelCardFocusRequester)
                    .clickable {
                        channelInput = channel
                        showChannelDialog = true
                    }
                    .then(if (isTv) Modifier.tvFocusable() else Modifier),
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(if (isTv) 20.dp else 16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = "Release channel",
                            style = if (isTv) MaterialTheme.typography.titleLarge else MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                        )
                        Spacer(Modifier.height(8.dp))
                        SettingsRow("Updates from", channel)
                    }
                    Icon(Icons.Default.Edit, contentDescription = "Change channel")
                }
            }

            Spacer(Modifier.height(24.dp))

            if (AutoUpdateWorker.supported) {
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable {
                            val next = !autoUpdate
                            scope.launch {
                                prefs.saveAutoUpdate(next)
                                if (!next) AutoUpdateWorker.cancel(context)
                            }
                        }
                        .then(if (isTv) Modifier.tvFocusable() else Modifier),
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(if (isTv) 20.dp else 16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = "Auto-install updates",
                                style = if (isTv) MaterialTheme.typography.titleLarge else MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                            )
                            Spacer(Modifier.height(8.dp))
                            Text(
                                text = "Updates apps installed through kobup while the device is idle on Wi-Fi",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        Switch(checked = autoUpdate, onCheckedChange = null)
                    }
                }

                Spacer(Modifier.height(24.dp))
            }

            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(if (isTv) 20.dp else 16.dp)) {
                    Text(
                        text = "About",
                        style = if (isTv) MaterialTheme.typography.titleLarge else MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                    )
                    Spacer(Modifier.height(12.dp))
                    SettingsRow("Version", "0.1.0")
                }
            }

            Spacer(Modifier.weight(1f))

            Button(
                onClick = {
                    scope.launch {
                        prefs.clear()
                        onLogout()
                    }
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(min = if (isTv) 56.dp else 48.dp)
                    .then(if (isTv) Modifier.tvFocusable() else Modifier),
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.error,
                ),
            ) {
                Text("Reset & Reconfigure")
            }
        }
    }

    if (showChannelDialog) {
        AlertDialog(
            onDismissRequest = { showChannelDialog = false },
            title = { Text("Release channel") },
            text = {
                Column {
                    Text(
                        "Which channel this device pulls updates from. Applies on the next catalog refresh.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.height(16.dp))
                    OutlinedTextField(
                        value = channelInput,
                        onValueChange = { channelInput = it },
                        label = { Text("Channel") },
                        singleLine = true,
                        modifier = Modifier
                            .fillMaxWidth()
                            .focusRequester(channelFieldFocusRequester),
                    )
                    if (isTv) {
                        LaunchedEffect(Unit) { channelFieldFocusRequester.requestFocusSafely() }
                    }
                    Spacer(Modifier.height(12.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        for (preset in listOf("stable", "beta")) {
                            AssistChip(
                                onClick = { channelInput = preset },
                                label = { Text(preset) },
                                modifier = if (isTv) Modifier.tvFocusable() else Modifier,
                            )
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        val next = channelInput.trim().ifBlank { PrefsManager.DEFAULT_CHANNEL }
                        scope.launch {
                            prefs.saveChannel(next)
                            showChannelDialog = false
                        }
                    },
                    modifier = if (isTv) Modifier.tvFocusable() else Modifier,
                ) {
                    Text("Save")
                }
            },
            dismissButton = {
                TextButton(
                    onClick = { showChannelDialog = false },
                    modifier = if (isTv) Modifier.tvFocusable() else Modifier,
                ) {
                    Text("Cancel")
                }
            },
        )
    }
}

@Composable
private fun SettingsRow(label: String, value: String) {
    Column {
        Text(
            text = label,
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
        )
    }
}
