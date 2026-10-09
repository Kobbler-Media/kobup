package com.kobup.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Android
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Badge
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.focusRestorer
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.kobup.app.data.model.CatalogItem
import com.kobup.app.data.model.LatestBySlugResponse
import com.kobup.app.data.repository.AppRepository
import com.kobup.app.ui.components.requestFocusSafely
import com.kobup.app.ui.components.tvFocusable
import kotlinx.coroutines.launch

private const val INITIAL_FOCUS_FRAMES = 12

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CatalogScreen(
    isTv: Boolean,
    onAppClick: (CatalogItem) -> Unit,
    onSettingsClick: () -> Unit,
) {
    val context = LocalContext.current
    val repo = remember { AppRepository(context) }
    val scope = rememberCoroutineScope()

    var catalog by remember { mutableStateOf<List<CatalogItem>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var isRefreshing by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var selfUpdate by remember { mutableStateOf<LatestBySlugResponse?>(null) }

    fun refresh(showRefreshIndicator: Boolean = false) {
        scope.launch {
            if (showRefreshIndicator) isRefreshing = true else isLoading = true
            error = null
            try {
                catalog = repo.refreshCatalog()
                selfUpdate = repo.checkSelfUpdate()
            } catch (e: Exception) {
                error = e.message ?: "Failed to load catalog"
            } finally {
                isLoading = false
                isRefreshing = false
            }
        }
    }

    LaunchedEffect(Unit) { refresh() }

    val initialFocusRequester = remember { FocusRequester() }
    val firstCardFocusRequester = remember { FocusRequester() }
    var gridHasFocus by remember { mutableStateOf(false) }

    LaunchedEffect(isTv, isLoading, error, catalog.isEmpty()) {
        if (!isTv || isLoading) return@LaunchedEffect
        if (error != null) {
            initialFocusRequester.requestFocusSafely()
            return@LaunchedEffect
        }
        if (catalog.isEmpty()) return@LaunchedEffect
        repeat(INITIAL_FOCUS_FRAMES) {
            withFrameNanos {}
            if (gridHasFocus) return@LaunchedEffect
            initialFocusRequester.requestFocusSafely()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Kobup") },
                actions = {
                    IconButton(
                        onClick = { refresh(showRefreshIndicator = true) },
                        modifier = if (isTv) Modifier.tvFocusable(shape = CircleShape) else Modifier,
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                    IconButton(
                        onClick = onSettingsClick,
                        modifier = if (isTv) Modifier.tvFocusable(shape = CircleShape) else Modifier,
                    ) {
                        Icon(Icons.Default.Settings, contentDescription = "Settings")
                    }
                },
            )
        },
    ) { padding ->
        Column(modifier = Modifier.padding(padding)) {
            // Self-update banner
            if (selfUpdate != null) {
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 8.dp)
                        .then(if (isTv) Modifier.tvFocusable() else Modifier)
                        .clickable {
                            // Navigate to self-update — find the kobup item in catalog
                            val kobupItem = catalog.find { it.packageName == "com.kobup.app" }
                            if (kobupItem != null) onAppClick(kobupItem)
                        },
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.primaryContainer,
                    ),
                ) {
                    Row(
                        modifier = Modifier.padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                "Kobup update available",
                                style = MaterialTheme.typography.titleSmall,
                                fontWeight = FontWeight.Bold,
                            )
                            Text(
                                "v${selfUpdate?.versionName} ready to install",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.7f),
                            )
                        }
                        Text(
                            "Update",
                            style = MaterialTheme.typography.labelLarge,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.primary,
                        )
                    }
                }
            }

            Box(modifier = Modifier.weight(1f)) {
            when {
                isLoading -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator()
                    }
                }
                error != null -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Text(
                                text = error ?: "",
                                color = MaterialTheme.colorScheme.error,
                                style = if (isTv) MaterialTheme.typography.bodyLarge else MaterialTheme.typography.bodyMedium,
                            )
                            Spacer(Modifier.height(16.dp))
                            androidx.compose.material3.Button(
                                onClick = { refresh() },
                                modifier = Modifier
                                    .focusRequester(initialFocusRequester)
                                    .then(if (isTv) Modifier.tvFocusable() else Modifier),
                            ) {
                                Text("Retry")
                            }
                        }
                    }
                }
                catalog.isEmpty() -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Text(
                            "No apps available yet",
                            style = if (isTv) MaterialTheme.typography.headlineSmall else MaterialTheme.typography.bodyLarge,
                        )
                    }
                }
                isTv -> {
                    TvCatalogGrid(
                        catalog = catalog,
                        repo = repo,
                        onAppClick = onAppClick,
                        firstCardFocusRequester = firstCardFocusRequester,
                        modifier = Modifier
                            .onFocusChanged { gridHasFocus = it.hasFocus }
                            .focusRequester(initialFocusRequester)
                            .focusRestorer(firstCardFocusRequester)
                            .focusGroup(),
                    )
                }
                else -> {
                    PullToRefreshBox(
                        isRefreshing = isRefreshing,
                        onRefresh = { refresh(showRefreshIndicator = true) },
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        PhoneCatalogList(catalog, repo, onAppClick)
                    }
                }
            }

            if (isTv && isRefreshing) {
                CircularProgressIndicator(
                    modifier = Modifier
                        .align(Alignment.TopCenter)
                        .padding(top = 16.dp),
                )
            }
            }
        }
    }
}

@Composable
private fun PhoneCatalogList(
    catalog: List<CatalogItem>,
    repo: AppRepository,
    onAppClick: (CatalogItem) -> Unit,
) {
    val updates = remember(catalog) {
        catalog.filter { item ->
            val installed = repo.getInstalledVersion(item.packageName)
            installed != null && item.latestVersion != null && installed != item.latestVersion.versionName
        }
    }

    LazyColumn(
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (updates.isNotEmpty()) {
            item {
                Text(
                    "Updates available",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(bottom = 4.dp),
                )
            }
            items(updates, key = { "update-${it.id}" }) { item ->
                AppCard(item, repo, isTv = false, onClick = { onAppClick(item) })
            }
            item {
                Spacer(Modifier.height(8.dp))
                Text(
                    "All apps",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(bottom = 4.dp),
                )
            }
        }
        items(catalog, key = { it.id }) { item ->
            AppCard(item, repo, isTv = false, onClick = { onAppClick(item) })
        }
    }
}

@Composable
private fun TvCatalogGrid(
    catalog: List<CatalogItem>,
    repo: AppRepository,
    onAppClick: (CatalogItem) -> Unit,
    firstCardFocusRequester: FocusRequester,
    modifier: Modifier = Modifier,
) {
    val updates = remember(catalog) {
        catalog.filter { item ->
            val installed = repo.getInstalledVersion(item.packageName)
            installed != null && item.latestVersion != null && installed != item.latestVersion.versionName
        }
    }

    LazyVerticalGrid(
        columns = GridCells.Adaptive(280.dp),
        modifier = modifier,
        contentPadding = PaddingValues(32.dp),
        horizontalArrangement = Arrangement.spacedBy(20.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        if (updates.isNotEmpty()) {
            item(span = { GridItemSpan(maxLineSpan) }) {
                Text(
                    "Updates available",
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(bottom = 8.dp),
                )
            }
            items(updates, key = { "update-${it.id}" }) { item ->
                AppCard(
                    item = item,
                    repo = repo,
                    isTv = true,
                    focusRequester = if (item.id == updates.first().id) firstCardFocusRequester else null,
                    onClick = { onAppClick(item) },
                )
            }
            item(span = { GridItemSpan(maxLineSpan) }) {
                Text(
                    "All apps",
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(top = 16.dp, bottom = 8.dp),
                )
            }
        }
        items(catalog, key = { it.id }) { item ->
            AppCard(
                item = item,
                repo = repo,
                isTv = true,
                focusRequester = if (updates.isEmpty() && item.id == catalog.firstOrNull()?.id) {
                    firstCardFocusRequester
                } else {
                    null
                },
                onClick = { onAppClick(item) },
            )
        }
    }
}

@Composable
private fun AppCard(
    item: CatalogItem,
    repo: AppRepository,
    isTv: Boolean,
    onClick: () -> Unit,
    focusRequester: FocusRequester? = null,
) {
    val installedVersion = remember(item.packageName) { repo.getInstalledVersion(item.packageName) }
    val hasUpdate = installedVersion != null &&
        item.latestVersion != null &&
        installedVersion != item.latestVersion.versionName

    val iconSize = if (isTv) 64.dp else 48.dp
    val cardPadding = if (isTv) 20.dp else 16.dp

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .then(if (focusRequester != null) Modifier.focusRequester(focusRequester) else Modifier)
            .then(if (isTv) Modifier.tvFocusable() else Modifier)
            .clickable(onClick = onClick),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
    ) {
        if (isTv) {
            // TV: vertical card layout — icon on top, text below, no wrapping issues
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(cardPadding),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    if (item.iconUrl != null) {
                        AsyncImage(
                            model = item.iconUrl,
                            contentDescription = "${item.name} icon",
                            modifier = Modifier.size(iconSize),
                        )
                    } else {
                        Icon(
                            imageVector = Icons.Default.Android,
                            contentDescription = null,
                            modifier = Modifier.size(iconSize),
                            tint = MaterialTheme.colorScheme.primary,
                        )
                    }
                    Spacer(Modifier.width(16.dp))
                    Text(
                        text = item.name,
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Bold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f),
                    )
                    if (hasUpdate) {
                        Spacer(Modifier.width(8.dp))
                        Badge(
                            containerColor = MaterialTheme.colorScheme.primary,
                            contentColor = MaterialTheme.colorScheme.onPrimary,
                        ) { Text("Update") }
                    }
                }
                if (item.description.isNotBlank()) {
                    Spacer(Modifier.height(8.dp))
                    Text(
                        text = item.description,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                Spacer(Modifier.height(8.dp))
                if (installedVersion != null) {
                    Text(
                        text = "Installed: $installedVersion",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    Text(
                        text = "Not installed",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                if (item.latestVersion != null) {
                    Text(
                        text = "Latest: ${item.latestVersion.versionName}",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        } else {
            // Phone: horizontal row layout
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(cardPadding),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (item.iconUrl != null) {
                    AsyncImage(
                        model = item.iconUrl,
                        contentDescription = "${item.name} icon",
                        modifier = Modifier.size(iconSize),
                    )
                } else {
                    Icon(
                        imageVector = Icons.Default.Android,
                        contentDescription = null,
                        modifier = Modifier.size(iconSize),
                        tint = MaterialTheme.colorScheme.primary,
                    )
                }
                Spacer(Modifier.width(16.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = item.name,
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    if (item.description.isNotBlank()) {
                        Text(
                            text = item.description,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    Spacer(Modifier.height(4.dp))
                    if (installedVersion != null) {
                        Text(
                            text = "Installed: $installedVersion",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    } else {
                        Text(
                            text = "Not installed",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    if (item.latestVersion != null) {
                        Text(
                            text = "Latest: ${item.latestVersion.versionName}",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                if (hasUpdate) {
                    Badge(
                        containerColor = MaterialTheme.colorScheme.primary,
                        contentColor = MaterialTheme.colorScheme.onPrimary,
                    ) { Text("Update") }
                }
            }
        }
    }
}
