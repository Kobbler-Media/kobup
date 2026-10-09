plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jetbrains.kotlin.plugin.serialization")
}

// Apply google-services only if the config file exists
if (file("google-services.json").exists()) {
    apply(plugin = "com.google.gms.google-services")
}

fun readVersionProps(): Map<String, String> =
    file("${rootDir}/version.properties").readLines()
        .filter { it.contains("=") }
        .associate { val (k, v) = it.split("=", limit = 2); k.trim() to v.trim() }

val ver = readVersionProps()

val releaseKeystore = file(System.getenv("KOBUP_KEYSTORE_PATH") ?: "${rootDir}/keystore/release.jks")

android {
    namespace = "com.kobup.app"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.kobup.app"
        minSdk = 24
        targetSdk = 36
        versionCode = ver["versionCode"]!!.toInt()
        versionName = ver["versionName"]!!

        val pinServiceUrl = providers.gradleProperty("pinServiceUrl").orNull ?: "https://pin.biboop.com"
        buildConfigField("String", "PIN_SERVICE_URL", "\"${pinServiceUrl.trimEnd('/')}\"")
        buildConfigField("String", "PIN_NAMESPACE", "\"kobup\"")
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    signingConfigs {
        create("release") {
            if (releaseKeystore.exists()) {
                storeFile = releaseKeystore
                storePassword = env.fetchOrNull("KOBUP_KEYSTORE_PASSWORD") ?: "CHANGEME"
                keyAlias = "kobup"
                keyPassword = env.fetchOrNull("KOBUP_KEY_PASSWORD") ?: "CHANGEME"
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            val releaseSigning = signingConfigs.findByName("release")
            if (releaseSigning?.storeFile?.exists() == true) {
                signingConfig = releaseSigning
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlin {
        jvmToolchain(17)
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")

    // Compose BOM
    val composeBom = platform("androidx.compose:compose-bom:2026.08.00")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.compose.ui:ui-tooling-preview")
    debugImplementation("androidx.compose.ui:ui-tooling")
    implementation("com.google.android.material:material:1.14.0")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.navigation:navigation-compose:2.9.8")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.11.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.11.0")

    // Networking
    implementation("com.squareup.retrofit2:retrofit:3.0.0")
    implementation("com.squareup.retrofit2:converter-kotlinx-serialization:3.0.0")
    implementation("com.squareup.okhttp3:okhttp:5.5.0")
    implementation("com.squareup.okhttp3:logging-interceptor:5.5.0")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.11.0")

    // DataStore
    implementation("androidx.datastore:datastore-preferences:1.2.1")

    // WorkManager
    implementation("androidx.work:work-runtime-ktx:2.11.2")

    // Firebase
    implementation(platform("com.google.firebase:firebase-bom:34.18.0"))
    implementation("com.google.firebase:firebase-messaging")

    // Coil for image loading
    implementation("io.coil-kt.coil3:coil-compose:3.5.0")
    implementation("io.coil-kt.coil3:coil-network-okhttp:3.5.0")
}

// -- Publish to Kobup --
// Usage: ./gradlew :app:publishToKobup
// Requires: KOBUP_TOKEN env var (create a CI token in the web admin)
// Auto-bumps versionCode, builds release APK, uploads, publishes, pushes to devices.

fun exec(vararg cmd: String): String {
    val p = ProcessBuilder(*cmd).directory(rootDir).redirectErrorStream(true).start()
    val out = p.inputStream.bufferedReader().readText().trim()
    p.waitFor()
    return out
}

tasks.register("publishToKobup") {
    doLast {
        // Block publish if there are uncommitted .kt changes
        val dirtyKt = exec("git", "status", "--porcelain", "--", "*.kt", "**/*.kt")
        if (dirtyKt.isNotBlank()) {
            throw GradleException("Uncommitted .kt changes — commit or stash before publishing:\n$dirtyKt")
        }

        val token = env.fetchOrNull("KOBUP_TOKEN")
            ?: throw GradleException("KOBUP_TOKEN not set. Add it to .env or export it. Create a CI token in the Kobup web admin.")

        // Without this the release build quietly falls back to unsigned and publishes
        // an APK no device can install.
        if (!releaseKeystore.exists()) {
            throw GradleException("Release keystore missing: ${releaseKeystore.absolutePath}")
        }

        val gitHash = exec("git", "rev-parse", "--short=7", "HEAD")

        // Bump versionCode and versionName in version.properties
        val propsFile = file("${rootDir}/version.properties")
        val lines = propsFile.readLines().toMutableList()
        val codeIdx = lines.indexOfFirst { it.startsWith("versionCode=") }
        val nameIdx = lines.indexOfFirst { it.startsWith("versionName=") }
        val oldCode = lines[codeIdx].substringAfter("=").trim().toInt()
        val oldName = lines[nameIdx].substringAfter("=").trim()
        val newCode = oldCode + 1
        // Bump patch version and append git hash: 0.1.2+abc1234
        val baseName = oldName.substringBefore("+")
        val parts = baseName.split(".").toMutableList()
        if (parts.size == 3) parts[2] = (parts[2].toIntOrNull()?.plus(1) ?: 0).toString()
        val newName = "${parts.joinToString(".")}+$gitHash"
        lines[codeIdx] = "versionCode=$newCode"
        lines[nameIdx] = "versionName=$newName"
        propsFile.writeText(lines.joinToString("\n") + "\n")
        logger.lifecycle("Bumped version: $oldName ($oldCode) → $newName ($newCode)")

        // Build release APK
        val apkDir = layout.buildDirectory.dir("outputs/apk/release").get().asFile
        apkDir.listFiles()?.filter { it.name.endsWith(".apk") }?.forEach { it.delete() }
        logger.lifecycle("Building release APK...")
        val buildProc = ProcessBuilder(
            "${rootDir}/gradlew", ":app:assembleRelease",
        ).directory(rootDir).redirectErrorStream(true).start()
        val buildOutput = buildProc.inputStream.bufferedReader().readText()
        val buildExit = buildProc.waitFor()
        if (buildExit != 0) {
            throw GradleException("assembleRelease failed:\n$buildOutput")
        }

        // Read .kobup.json config
        val configFile = file("${rootDir}/.kobup.json")
        if (!configFile.exists()) throw GradleException("Missing .kobup.json in project root")

        @Suppress("UNCHECKED_CAST")
        val config = groovy.json.JsonSlurper().parseText(configFile.readText()) as Map<String, Any>
        val server = config["server"] as String
        val slug = config["projectSlug"] as String
        val projectName = config["projectName"] as? String ?: slug
        val packageName = config["packageName"] as? String ?: ""
        val channel = config["channel"] as String

        // Ensure project exists (idempotent upsert)
        logger.lifecycle("Ensuring project '$slug' exists on $server...")
        val createProc = ProcessBuilder(
            "curl", "-s", "-X", "POST",
            "$server/api/v1/admin/projects",
            "-H", "Authorization: Bearer $token",
            "-H", "Content-Type: application/json",
            "-d", """{"slug":"$slug","name":"$projectName","packageName":"$packageName"}""",
        ).redirectErrorStream(true).start()
        val createOutput = createProc.inputStream.bufferedReader().readText()
        val createExit = createProc.waitFor()
        if (createExit != 0) {
            logger.warn("Project create/check returned non-zero (may be fine if using CI token): $createOutput")
        }

        val apks = apkDir.listFiles()?.filter { it.name.endsWith(".apk") }.orEmpty()
        val unsigned = apks.filter { it.name.endsWith("-unsigned.apk") }
        if (unsigned.isNotEmpty()) {
            throw GradleException("Refusing to publish unsigned APK: ${unsigned.joinToString { it.name }}")
        }
        val apk = apks.singleOrNull()
            ?: throw GradleException(
                "Expected one release APK in ${apkDir.absolutePath}, found ${apks.size}" +
                    apks.joinToString(prefix = ": ") { it.name }
            )

        logger.lifecycle("Publishing ${apk.name} (v$newName code $newCode) to $server")

        val process = ProcessBuilder(
            "curl", "-s", "-f", "-X", "POST",
            "$server/api/v1/ci/projects/$slug/release",
            "-H", "Authorization: Bearer $token",
            "-F", "apk=@${apk.absolutePath}",
            "-F", "channel=$channel",
            "-F", "publish=true",
        ).redirectErrorStream(true).start()

        val output = process.inputStream.bufferedReader().readText()
        val exitCode = process.waitFor()

        if (exitCode != 0) {
            // Revert version on failure
            lines[codeIdx] = "versionCode=$oldCode"
            lines[nameIdx] = "versionName=$oldName"
            propsFile.writeText(lines.joinToString("\n") + "\n")
            throw GradleException("Kobup publish failed (exit $exitCode):\n$output")
        }

        logger.lifecycle("Published successfully:\n$output")

        // Push refresh to notify devices
        ProcessBuilder(
            "curl", "-s", "-X", "POST",
            "$server/api/v1/ci/projects/$slug/push-refresh",
            "-H", "Authorization: Bearer $token",
        ).start().waitFor()

        logger.lifecycle("Push refresh sent to devices.")
    }
}
