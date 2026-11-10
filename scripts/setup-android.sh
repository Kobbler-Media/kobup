#!/bin/bash
set -e
cd "$(dirname "$0")/.."

GRADLE_VERSION=$(sed -n 's|^distributionUrl=.*/gradle-\(.*\)-bin\.zip$|\1|p' apps/kobup-android/gradle/wrapper/gradle-wrapper.properties)

echo "Setting up Android projects..."

for dir in apps/kobup-android apps/kobup-sdk apps/kobup-sample-host; do
    if [ -d "$dir" ] && [ ! -f "$dir/gradlew" ]; then
        echo "Generating Gradle wrapper in $dir..."
        (cd "$dir" && gradle wrapper --gradle-version "$GRADLE_VERSION" 2>/dev/null || echo "  Note: Install Gradle to generate wrapper, or use Android Studio")
    fi
done

echo "Done. Open the Android projects in Android Studio to complete setup."
echo "You will need to add google-services.json to apps/kobup-android/app/"
