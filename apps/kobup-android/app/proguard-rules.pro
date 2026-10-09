# kotlinx.serialization
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.AnnotationsKt
-keepclassmembers class kotlinx.serialization.json.** { *** Companion; }
-keepclasseswithmembers class kotlinx.serialization.json.** { kotlinx.serialization.KSerializer serializer(...); }
-keep,includedescriptorclasses class com.kobup.app.**$$serializer { *; }
-keepclassmembers class com.kobup.app.** { *** Companion; }
-keepclasseswithmembers class com.kobup.app.** { kotlinx.serialization.KSerializer serializer(...); }

# Retrofit
-keepattributes Signature
-keepattributes Exceptions
-keep class retrofit2.** { *; }
-keepclassmembers,allowshrinking,allowobfuscation interface * {
    @retrofit2.http.* <methods>;
}

# Room 2.7+ / WorkManager 2.10+: Room instantiates generated _Impl classes by
# reflection, and R8 strips the no-arg constructor without these rules.
-keep class * extends androidx.room.RoomDatabase {
    <init>();
}
-keep class androidx.work.impl.WorkDatabase_Impl { <init>(); }
