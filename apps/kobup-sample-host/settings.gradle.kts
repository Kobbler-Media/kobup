pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}
rootProject.name = "kobup-sample-host"
include(":app")
include(":kobup-sdk")
project(":kobup-sdk").projectDir = file("../kobup-sdk/sdk")
