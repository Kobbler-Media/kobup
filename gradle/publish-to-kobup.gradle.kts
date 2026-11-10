// Kobup publish helper. Apply from an Android app module's build.gradle.kts:
//   apply(from = "/path/to/kobup/gradle/publish-to-kobup.gradle.kts")
//
// Needs .kobup.json (server, projectSlug, projectName, packageName, channel) and
// version.properties (versionCode, versionName) in the root project, and KOBUP_TOKEN in
// the environment or the root project's .env.
//
// Usage: ./gradlew :app:publishToKobup
//
// Kotlin rather than Groovy: Gradle's Groovy cannot compile a script on a newer JDK than
// it knows, and fails the whole build at configuration time when it tries.

tasks.register("publishToKobup") {
    // Read here, not in the action: an action that touches the script object cannot be stored in
    // the configuration cache, and apps applying this script may run with it on.
    val publishRootDir: File = rootDir
    val publishModulePath: String = project.path
    val publishApkDir: File = layout.buildDirectory.dir("outputs/apk/release").get().asFile

    doLast {
        fun run(vararg cmd: String): Pair<Int, String> {
            val p = ProcessBuilder(*cmd).directory(publishRootDir).redirectErrorStream(true).start()
            val out = p.inputStream.bufferedReader().readText().trim()
            return p.waitFor() to out
        }

        val dirtyKt = run("git", "status", "--porcelain", "--", "*.kt", "**/*.kt").second
        if (dirtyKt.isNotBlank()) {
            throw GradleException("Uncommitted .kt changes — commit or stash before publishing:\n$dirtyKt")
        }

        val dotEnv = File(publishRootDir, ".env").takeIf { it.exists() }?.readLines().orEmpty()
        val token = System.getenv("KOBUP_TOKEN")
            ?: dotEnv.firstOrNull { it.startsWith("KOBUP_TOKEN=") }?.substringAfter("=")?.trim()?.takeIf { it.isNotEmpty() }
            ?: throw GradleException("KOBUP_TOKEN not set. Add it to .env or export it (a CI token from the Kobup web admin).")

        val configFile = File(publishRootDir, ".kobup.json")
        if (!configFile.exists()) throw GradleException("Missing .kobup.json in ${publishRootDir.absolutePath}")
        @Suppress("UNCHECKED_CAST")
        val config = groovy.json.JsonSlurper().parseText(configFile.readText()) as Map<String, Any?>
        val server = config["server"] as String
        val slug = config["projectSlug"] as String
        val projectName = config["projectName"] as? String ?: slug
        val packageName = config["packageName"] as? String ?: ""
        val channel = config["channel"] as? String ?: "stable"

        val gitHash = run("git", "rev-parse", "--short=7", "HEAD").second

        val propsFile = File(publishRootDir, "version.properties")
        if (!propsFile.exists()) throw GradleException("Missing version.properties in ${publishRootDir.absolutePath}")
        val original = propsFile.readText()
        val lines = propsFile.readLines().toMutableList()
        val codeIdx = lines.indexOfFirst { it.startsWith("versionCode=") }
        val nameIdx = lines.indexOfFirst { it.startsWith("versionName=") }
        val oldCode = lines[codeIdx].substringAfter("=").trim().toInt()
        val oldName = lines[nameIdx].substringAfter("=").trim()
        val newCode = oldCode + 1
        val parts = oldName.substringBefore("+").split(".").toMutableList()
        if (parts.size == 3) parts[2] = (parts[2].toIntOrNull()?.plus(1) ?: 0).toString()
        val newName = "${parts.joinToString(".")}+$gitHash"
        lines[codeIdx] = "versionCode=$newCode"
        lines[nameIdx] = "versionName=$newName"
        propsFile.writeText(lines.joinToString("\n") + "\n")
        logger.lifecycle("Bumped version: $oldName ($oldCode) → $newName ($newCode)")

        fun fail(message: String): Nothing {
            propsFile.writeText(original)
            throw GradleException(message)
        }

        publishApkDir.listFiles()?.filter { it.name.endsWith(".apk") }?.forEach { it.delete() }
        logger.lifecycle("Building release APK(s)...")
        val (buildExit, buildOutput) = run(File(publishRootDir, "gradlew").absolutePath, "$publishModulePath:assembleRelease")
        if (buildExit != 0) fail("assembleRelease failed:\n$buildOutput")

        val apks = publishApkDir.listFiles()?.filter { it.name.endsWith(".apk") }.orEmpty()
        // Without a keystore the release build quietly comes out unsigned, and no device
        // can install it.
        val unsigned = apks.filter { it.name.endsWith("-unsigned.apk") }
        if (unsigned.isNotEmpty()) fail("Refusing to publish unsigned APK: ${unsigned.joinToString { it.name }}")
        if (apks.isEmpty()) fail("No release APK in ${publishApkDir.absolutePath}")

        // The baseline profile as dex metadata, one per API band, so the installer can have
        // ART compile the app at install time rather than leave it on the JIT.
        val outputMetadata = File(publishApkDir, "output-metadata.json")
        val dmFields = if (!outputMetadata.exists()) emptyList() else {
            @Suppress("UNCHECKED_CAST")
            val bands = (groovy.json.JsonSlurper().parseText(outputMetadata.readText()) as Map<String, Any?>)["baselineProfiles"]
                as? List<Map<String, Any?>> ?: emptyList()
            bands.flatMap { band ->
                val dm = (band["baselineProfiles"] as? List<*>)?.firstOrNull()?.let { File(publishApkDir, it.toString()) }
                if (dm?.exists() == true) listOf("-F", "dm_${band["minApi"]}=@${dm.absolutePath}") else emptyList()
            }
        }

        logger.lifecycle("Ensuring project '$slug' exists on $server...")
        run(
            "curl", "-s", "-X", "POST", "$server/api/v1/admin/projects",
            "-H", "Authorization: Bearer $token",
            "-H", "Content-Type: application/json",
            "-d", """{"slug":"$slug","name":"$projectName","packageName":"$packageName"}""",
        )

        // ABI splits go up in one request: the server files them under one release and
        // hands each device the smallest APK its ABIs can run.
        val apkFields = apks.flatMap { listOf("-F", "apk=@${it.absolutePath}") }
        logger.lifecycle("Publishing ${apks.joinToString { it.name }} (v$newName code $newCode) with ${dmFields.size / 2} profile(s) to $server")
        val (uploadExit, uploadOutput) = run(
            *arrayOf(
                "curl", "-s", "-f", "-X", "POST", "$server/api/v1/ci/projects/$slug/release",
                "-H", "Authorization: Bearer $token",
                "-F", "channel=$channel",
                "-F", "publish=true",
            ) + apkFields + dmFields,
        )
        if (uploadExit != 0) fail("Kobup publish failed (exit $uploadExit):\n$uploadOutput")
        logger.lifecycle("Published successfully:\n$uploadOutput")

        run("curl", "-s", "-X", "POST", "$server/api/v1/ci/projects/$slug/push-refresh", "-H", "Authorization: Bearer $token")
        logger.lifecycle("Push refresh sent to devices.")
    }
}
