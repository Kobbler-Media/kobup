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

// Usage: ./gradlew :app:publishToKobup (KOBUP_TOKEN in .env)
apply(from = "${rootDir}/../../gradle/publish-to-kobup.gradle.kts")
tasks.named("publishToKobup") {
    doFirst {
        if (!releaseKeystore.exists()) {
            throw GradleException("Release keystore missing: ${releaseKeystore.absolutePath}")
        }
    }
}
