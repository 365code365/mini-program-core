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
        versionCode = 4
        versionName = "1.1.2"
        // 真机与 Apple 芯片上的模拟器都是 arm64；三个 ABI 全打进去 APK 要大一百多 MB，装包也慢
        ndk { abiFilters += listOf("arm64-v8a") }
    }
    sourceSets["main"].apply {
        // SDK 不复制：直接编 sdk/android 的那一份 Kotlin 与 .so
        java.srcDirs("src/main/kotlin", "../../src/main/kotlin")
        jniLibs.srcDirs("../../src/main/jniLibs")
        // 演示小程序与 Flutter 示例共用（assets/demo/…），启动时解包到 filesDir
        assets.srcDirs("../../../flutter/example/assets", layout.buildDirectory.dir("generated/gameAssets"))
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

// 小游戏夹具（仓库的 sample-game/）拷进 assets/game/：GameActivity 打开它们（game-doc/05 §2.1）。
// 不用软链：AssetManager 解包后软链会失效
val copyGameAssets by tasks.registering(Copy::class) {
    from("../../../../sample-game")
    into(layout.buildDirectory.dir("generated/gameAssets/game"))
    exclude("**/README.md")
    // AAPT 默认丢掉以 _ 开头的资源目录（ignoreAssetsPattern 含 `_*`）：_lib 打包成 shared-lib，GameAssets 解包时还原
    eachFile { path = path.replace("_lib/", "shared-lib/") }
    includeEmptyDirs = false
}
tasks.named("preBuild") { dependsOn(copyGameAssets) }
