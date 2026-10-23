// 纯原生（不经 Flutter）的集成示例：一个 Activity + MiniProgramView，内置演示小程序。
//
//   bash tools/build-android-abi.sh                 # 先编引擎库（→ sdk/android/src/main/jniLibs）
//   cd sdk/android/example && ./gradlew installDebug
//   bash tools/android-native-check.sh              # 真机验收：渲染逐像素 + 各功能
//
// SDK 的 Kotlin 源码与 .so 直接引用 sdk/android 那一份（见 app/build.gradle.kts），不经 mavenLocal：
// 改了 SDK 立刻能在这里验证，也不会出现「新 Kotlin 配老 .so」的 UnsatisfiedLinkError。
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
plugins {
    // 与 Flutter 示例同一档（本机 Gradle 缓存里已有，离线也能编）
    id("com.android.application") version "8.11.1" apply false
    id("org.jetbrains.kotlin.android") version "2.2.20" apply false
}
rootProject.name = "mini-render-native-example"
include(":app")
