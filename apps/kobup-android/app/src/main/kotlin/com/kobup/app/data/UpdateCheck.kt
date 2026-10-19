package com.kobup.app.data

import com.kobup.app.data.model.CatalogItem
import com.kobup.app.data.model.LatestVersion

fun hasUpdate(installedVersionName: String?, latest: LatestVersion?): Boolean =
    installedVersionName != null && latest != null && installedVersionName != latest.versionName

fun availableUpdates(catalog: List<CatalogItem>, installedVersion: (String) -> String?): List<CatalogItem> =
    catalog.filter { hasUpdate(installedVersion(it.packageName), it.latestVersion) }

fun unnotified(updates: List<CatalogItem>, notifiedVersionCodes: Map<String, Long>): List<CatalogItem> =
    updates.filter { notifiedVersionCodes[it.packageName] != it.latestVersion?.versionCode }
