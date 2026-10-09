package com.kobup.app.ui.screens

import android.content.pm.PackageManager
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Launch
import androidx.compose.material.icons.filled.Android
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Update
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.kobup.app.data.model.CatalogItem
import com.kobup.app.data.model.LatestBySlugResponse
import com.kobup.app.data.repository.AppRepository
import com.kobup.app.service.InstallResultReceiver
import com.kobup.app.service.InstallService
import com.kobup.app.ui.components.requestFocusSafely
import com.kobup.app.ui.components.tvFocusable
import android.content.pm.PackageInstaller
import kotlinx.coroutines.launch

enum class InstallState {
    IDLE,
    DOWNLOADING,
    INSTALLING,
    SUCCESS,      // Download done, PackageInstaller launched, waiting for result
    INSTALLED,    // PackageInstaller confirmed success
    ERROR,
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AppDetailScreen(
    item: CatalogItem,
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    val isTv = remember {
        context.packageManager.hasSystemFeature(PackageManager.FEATURE_LEANBACK)
    }
    val repo = remember { AppRepository(context) }
    val installService = remember { InstallService(context) }
    val scope = rememberCoroutineScope()

    val installedVersion = remember(item.packageName) { repo.getInstalledVersion(item.packageName) }
    val installedVersionCode = remember(item.packageName) { repo.getInstalledVersionCode(item.packageName) }
    val isInstalled = installedVersion != null
    val launchIntent = remember(item.packageName, isInstalled) {
        if (isInstalled) repo.getLaunchIntent(item.packageName) else null
    }
    val latestVersionName = item.latestVersion?.versionName
    val hasUpdate = isInstalled && latestVersionName != null && installedVersion != latestVersionName

    var artifactInfo by remember { mutableStateOf<LatestBySlugResponse?>(null) }
    var artifactLoading by remember { mutableStateOf(true) }
    var installState by remember { mutableStateOf(InstallState.IDLE) }
    var progress by remember { mutableFloatStateOf(0f) }
    var errorMessage by remember { mutableStateOf("") }

    // Observe install results from the PackageInstaller broadcast
    val installResult by InstallResultReceiver.lastResult.collectAsState()
    LaunchedEffect(installResult) {
        val result = installResult ?: return@LaunchedEffect
        if (installState == InstallState.SUCCESS || installState == InstallState.INSTALLING) {
            if (result.status == PackageInstaller.STATUS_SUCCESS) {
                installState = InstallState.INSTALLED
            } else if (result.status != PackageInstaller.STATUS_PENDING_USER_ACTION) {
                errorMessage = result.message ?: "Install failed (status ${result.status})"
                installState = InstallState.ERROR
            }
            InstallResultReceiver.clear()
        }
    }

    LaunchedEffect(item.slug) {
        artifactLoading = true
        try {
            artifactInfo = repo.getLatestBySlug(item.slug)
        } catch (_: Exception) {
            // No artifact available
        }
        artifactLoading = false
    }

    val actionFocusRequester = remember { FocusRequester() }
    val hasActionButton = when (installState) {
        InstallState.IDLE -> !artifactLoading && artifactInfo != null
        InstallState.INSTALLED, InstallState.ERROR -> true
        else -> false
    }

    LaunchedEffect(isTv, artifactLoading, installState) {
        if (!isTv) return@LaunchedEffect
        if (!hasActionButton && launchIntent == null) return@LaunchedEffect
        actionFocusRequester.requestFocusSafely()
    }

    fun startInstall() {
        val info = artifactInfo ?: return
        scope.launch {
            installState = InstallState.DOWNLOADING
            progress = 0f
            try {
                installService.downloadAndInstall(
                    repo = repo,
                    artifactId = info.artifactId,
                    expectedSha256 = info.sha256,
                    dexMetadata = info.dexMetadata,
                    onProgress = { progress = it },
                )
                installState = InstallState.SUCCESS
                repo.reportInstallEvent(
                    packageName = item.packageName,
                    fromVersionCode = installedVersionCode,
                    toVersionCode = info.versionCode,
                    status = "success",
                )
            } catch (e: Exception) {
                errorMessage = e.message ?: "Install failed"
                installState = InstallState.ERROR
                repo.reportInstallEvent(
                    packageName = item.packageName,
                    fromVersionCode = installedVersionCode,
                    toVersionCode = info.versionCode,
                    status = "error",
                    message = e.message,
                )
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(item.name) },
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
                .padding(if (isTv) 32.dp else 24.dp)
                .verticalScroll(rememberScrollState()),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = Icons.Default.Android,
                    contentDescription = null,
                    modifier = Modifier.size(if (isTv) 80.dp else 64.dp),
                    tint = MaterialTheme.colorScheme.primary,
                )
                Spacer(Modifier.width(if (isTv) 24.dp else 16.dp))
                Column {
                    Text(
                        text = item.name,
                        style = if (isTv) MaterialTheme.typography.headlineMedium else MaterialTheme.typography.headlineSmall,
                        fontWeight = FontWeight.Bold,
                    )
                    Text(
                        text = item.packageName,
                        style = if (isTv) MaterialTheme.typography.bodyMedium else MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            Spacer(Modifier.height(if (isTv) 32.dp else 24.dp))

            Text(
                text = item.description,
                style = if (isTv) MaterialTheme.typography.bodyLarge else MaterialTheme.typography.bodyLarge,
            )

            Spacer(Modifier.height(if (isTv) 32.dp else 24.dp))

            // Version info
            Column(verticalArrangement = Arrangement.spacedBy(if (isTv) 12.dp else 8.dp)) {
                if (installedVersion != null) {
                    DetailRow("Installed version", installedVersion)
                }
                if (latestVersionName != null) {
                    DetailRow("Latest version", latestVersionName)
                }
                if (item.latestVersion?.channel != null) {
                    DetailRow("Channel", item.latestVersion.channel)
                }
                val sizeBytes = artifactInfo?.sizeBytes
                if (sizeBytes != null) {
                    DetailRow("Size", formatFileSize(sizeBytes))
                }
            }

            Spacer(Modifier.height(32.dp))

            if (launchIntent != null) {
                OutlinedButton(
                    onClick = { context.startActivity(launchIntent) },
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(min = if (isTv) 56.dp else 48.dp)
                        .then(if (hasActionButton) Modifier else Modifier.focusRequester(actionFocusRequester))
                        .then(if (isTv) Modifier.tvFocusable() else Modifier),
                ) {
                    Icon(Icons.AutoMirrored.Filled.Launch, contentDescription = null)
                    Spacer(Modifier.width(8.dp))
                    Text("Open ${item.name}")
                }
                Spacer(Modifier.height(if (isTv) 16.dp else 12.dp))
            }

            // Install/Update button
            when (installState) {
                InstallState.IDLE -> {
                    if (artifactLoading) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            CircularProgressIndicator(modifier = Modifier.size(24.dp))
                            Spacer(Modifier.width(12.dp))
                            Text("Loading artifact info...")
                        }
                    } else if (artifactInfo != null) {
                        Button(
                            onClick = { startInstall() },
                            modifier = Modifier
                                .fillMaxWidth()
                                .heightIn(min = if (isTv) 56.dp else 48.dp)
                                .focusRequester(actionFocusRequester)
                                .then(if (isTv) Modifier.tvFocusable() else Modifier),
                        ) {
                            Icon(
                                imageVector = if (hasUpdate) Icons.Default.Update else Icons.Default.Download,
                                contentDescription = null,
                            )
                            Spacer(Modifier.width(8.dp))
                            Text(
                                text = when {
                                    hasUpdate -> "Update to $latestVersionName"
                                    isInstalled -> "Reinstall"
                                    else -> "Install"
                                },
                            )
                        }
                    } else {
                        Text(
                            text = "No download available",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }

                InstallState.DOWNLOADING -> {
                    Column {
                        Text("Downloading...", style = MaterialTheme.typography.bodyMedium)
                        Spacer(Modifier.height(8.dp))
                        LinearProgressIndicator(
                            progress = { progress },
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Spacer(Modifier.height(4.dp))
                        Text(
                            text = "${(progress * 100).toInt()}%",
                            style = MaterialTheme.typography.labelSmall,
                        )
                    }
                }

                InstallState.INSTALLING -> {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        CircularProgressIndicator(modifier = Modifier.size(24.dp))
                        Spacer(Modifier.width(12.dp))
                        Text("Installing...")
                    }
                }

                InstallState.SUCCESS -> {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        CircularProgressIndicator(modifier = Modifier.size(24.dp))
                        Spacer(Modifier.width(12.dp))
                        Text("Installing — confirm in the system dialog if prompted...", style = MaterialTheme.typography.bodyMedium)
                    }
                }

                InstallState.INSTALLED -> {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            Icons.Default.CheckCircle,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                        )
                        Spacer(Modifier.width(8.dp))
                        Text("Installed successfully!", style = MaterialTheme.typography.bodyMedium)
                    }
                    Spacer(Modifier.height(12.dp))
                    Button(
                        onClick = onBack,
                        modifier = Modifier
                            .focusRequester(actionFocusRequester)
                            .then(if (isTv) Modifier.tvFocusable() else Modifier),
                    ) {
                        Text("Back to catalog")
                    }
                }

                InstallState.ERROR -> {
                    Column {
                        Text(
                            text = "Error: $errorMessage",
                            color = MaterialTheme.colorScheme.error,
                        )
                        Spacer(Modifier.height(12.dp))
                        Button(
                            onClick = { startInstall() },
                            modifier = Modifier
                                .focusRequester(actionFocusRequester)
                                .then(if (isTv) Modifier.tvFocusable() else Modifier),
                        ) {
                            Text("Retry")
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun DetailRow(label: String, value: String) {
    Row {
        Text(
            text = "$label: ",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.Medium,
        )
    }
}

private fun formatFileSize(bytes: Long): String {
    return when {
        bytes >= 1_000_000_000 -> "%.1f GB".format(bytes / 1_000_000_000.0)
        bytes >= 1_000_000 -> "%.1f MB".format(bytes / 1_000_000.0)
        bytes >= 1_000 -> "%.1f KB".format(bytes / 1_000.0)
        else -> "$bytes B"
    }
}
