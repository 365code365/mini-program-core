import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "dev.minirender.example"
    compileSdk = 36
    defaultConfig {
        applicationId = "dev.minirender.example"
        // 与 SDK（sdk/android/build.gradle.kts）、tools/build-mobile.sh 的 NDK API level 一致
        minSdk = 24
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0"
        // 真机与 Apple 芯片上的模拟器都是 arm64；三个 ABI 全打进去 APK 要大一百多 MB，装包也慢
        ndk { abiFilters += listOf("arm64-v8a") }
    }
    sourceSets["main"].apply {
        // SDK 不复制：直接编 sdk/android 的那一份 Kotlin 与 .so
        java.srcDirs("src/main/kotlin", "../../src/main/kotlin")
        jniLibs.srcDirs("../../src/main/jniLibs")
        // 演示小程序与 Flutter 示例共用（assets/demo/…），启动时解包到 filesDir
        assets.srcDirs("../../../flutter/example/assets")
    }
    buildTypes {
        // 量流畅度装 release（`./gradlew installRelease`）：debug 包是 DEBUGGABLE，ART 不做 AOT、
        // 主线程与引擎线程上的 Kotlin 都慢一截，量出来的「顿」有一部分是包本身。示例不发布，用 debug 签名
        release { signingConfig = signingConfigs.getByName("debug") }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin {
    compilerOptions { jvmTarget.set(JvmTarget.JVM_17) }
}
