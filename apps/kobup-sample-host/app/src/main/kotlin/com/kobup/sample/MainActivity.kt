package com.kobup.sample

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.kobup.sdk.KobupSdk
import com.kobup.sdk.KobupUpdateStatus

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                SampleScreen()
            }
        }
    }
}

@Composable
fun SampleScreen() {
    val context = LocalContext.current
    val activity = context as? android.app.Activity
    var statusText by remember { mutableStateOf("Not checked") }
    var isInstalled by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        isInstalled = KobupSdk.isInstalled(context)
    }

    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text("Kobup SDK Sample", style = MaterialTheme.typography.headlineMedium)

        Text(
            if (isInstalled) "Kobup: Installed" else "Kobup: Not installed",
            style = MaterialTheme.typography.bodyLarge
        )

        Button(onClick = {
            isInstalled = KobupSdk.isInstalled(context)
        }) {
            Text("Check Kobup Installed")
        }

        Button(
            onClick = {
                val status = KobupSdk.getUpdateStatus(context, context.packageName)
                statusText = when (status) {
                    is KobupUpdateStatus.Available -> "Update available: ${status.versionName}"
                    is KobupUpdateStatus.UpToDate -> "Up to date"
                    is KobupUpdateStatus.Unknown -> "Unknown (check in Kobup)"
                    is KobupUpdateStatus.KobupNotInstalled -> "Kobup not installed"
                }
            },
            enabled = isInstalled
        ) {
            Text("Check for Updates")
        }

        Text("Status: $statusText")

        Button(
            onClick = { KobupSdk.openKobup(context) },
            enabled = isInstalled
        ) {
            Text("Open Kobup")
        }

        Button(
            onClick = { KobupSdk.openKobup(context, context.packageName) },
            enabled = isInstalled
        ) {
            Text("Open Kobup (my app page)")
        }

        if (activity != null) {
            Button(
                onClick = { KobupSdk.maybePromptForUpdate(activity, context.packageName) },
                enabled = isInstalled
            ) {
                Text("Prompt for Update")
            }
        }
    }
}
