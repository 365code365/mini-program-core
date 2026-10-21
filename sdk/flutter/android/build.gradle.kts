group = "dev.minirender.flutter"
version = "0.1.0"

buildscript {
    val kotlinVersion = "2.2.20"
    repositories {
        google()
        mavenCentral()
    }

    dependencies {
        classpath("com.android.tools.build:gradle:8.11.1")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:$kotlinVersion")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

plugins {
    id("com.android.library")
    id("kotlin-android")
}

android {
    namespace = "dev.minirender.flutter"

    compileSdk = 36

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = JavaVersion.VERSION_17.toString()
    }

    sourceSets {
        getByName("main") {
            // JNI 绑定（dev.minirender.MiniEngine）与 .so 和原生 SDK 共用同一份，
            // 不复制：两边各存一份迟早对不上（新的 Kotlin 配老的 .so 会 UnsatisfiedLinkError）。
            // .so 由仓库根目录的 tools/build-mobile.sh android（或 build-android-abi.sh）产出。
            java.srcDirs("src/main/kotlin", "../../android/src/main/kotlin")
            jniLibs.srcDirs("../../android/src/main/jniLibs")
        }
    }

    defaultConfig {
        minSdk = 24
    }
}
