package com.kobup.app

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.kobup.app.data.local.PrefsManager
import com.kobup.app.data.model.CatalogItem
import com.kobup.app.ui.screens.AppDetailScreen
import com.kobup.app.ui.screens.CatalogScreen
import com.kobup.app.ui.screens.SettingsScreen
import com.kobup.app.ui.screens.SetupScreen
import com.kobup.app.ui.theme.KobupTheme
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (!packageManager.hasSystemFeature(PackageManager.FEATURE_LEANBACK)) {
            enableEdgeToEdge()
        }
        setContent {
            KobupTheme {
                KobupNavHost()
            }
        }
    }
}

@Composable
private fun KobupNavHost() {
    val context = LocalContext.current
    val prefs = remember { PrefsManager(context) }
    val isSetupDone by prefs.isSetupDone.collectAsState(initial = null)
    val isTv = remember {
        context.packageManager.hasSystemFeature(PackageManager.FEATURE_LEANBACK)
    }

    val navController = rememberNavController()
    val json = remember { Json { ignoreUnknownKeys = true } }

    // Wait for initial prefs load
    val setupDone = isSetupDone ?: return

    val startDest = if (setupDone) "catalog" else "setup"

    val notificationPermission = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) {}
    LaunchedEffect(setupDone) {
        if (setupDone && !isTv && Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            notificationPermission.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
    }

    NavHost(navController = navController, startDestination = startDest) {
        composable("setup") {
            SetupScreen(
                isTv = isTv,
                onSetupComplete = {
                    navController.navigate("catalog") {
                        popUpTo("setup") { inclusive = true }
                    }
                },
            )
        }

        composable("catalog") {
            CatalogScreen(
                isTv = isTv,
                onAppClick = { item ->
                    val encoded = android.net.Uri.encode(json.encodeToString(item))
                    navController.navigate("detail/$encoded")
                },
                onSettingsClick = {
                    navController.navigate("settings")
                },
            )
        }

        composable("detail/{itemJson}") { backStackEntry ->
            val itemJson = backStackEntry.arguments?.getString("itemJson") ?: return@composable
            val decoded = android.net.Uri.decode(itemJson)
            val item = json.decodeFromString<CatalogItem>(decoded)
            AppDetailScreen(
                initialItem = item,
                onBack = { navController.popBackStack() },
            )
        }

        composable("settings") {
            SettingsScreen(
                onBack = { navController.popBackStack() },
                onLogout = {
                    navController.navigate("setup") {
                        popUpTo(0) { inclusive = true }
                    }
                },
            )
        }
    }
}
