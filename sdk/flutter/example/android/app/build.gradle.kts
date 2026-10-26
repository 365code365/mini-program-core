plugins {
    id("com.android.application")
    id("kotlin-android")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "dev.minirender.mini_render_example"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = JavaVersion.VERSION_17.toString()
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "dev.minirender.mini_render_example"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }
    // 只打 arm64（真机与 Apple 芯片上的模拟器都是它；构建用 `flutter build apk --target-platform android-arm64`）：
    // 插件的 jniLibs 里三个 ABI 的引擎库各 40MB 上下，全打进去 APK 是 180MB。defaultConfig 的 abiFilters
    // 会被 Flutter 的 Gradle 插件按 --target-platform 改写，挡不住插件带来的另外两份，所以在打包这一步排除
    packaging { jniLibs { excludes += listOf("lib/armeabi-v7a/**", "lib/x86_64/**", "lib/x86/**") } }

    buildTypes {
        release {
            // TODO: Add your own signing config for the release build.
            // Signing with the debug keys for now, so `flutter run --release` works.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

flutter {
    source = "../.."
}
