// mini-render 的 Android 库模块：产出 AAR（含各 ABI 的 libmini_render.so）。
//
//   bash tools/build-mobile.sh android      # 先编出 .so 放进 src/main/jniLibs/
//   cd sdk/android && ./gradlew publishToMavenLocal
//
// 之后业务工程只要：
//   repositories { mavenLocal() }
//   implementation("dev.minirender:mini-render:0.1.0")
plugins {
    id("com.android.library") version "8.5.0"
    id("org.jetbrains.kotlin.android") version "1.9.24"
    id("maven-publish")
}

android {
    namespace = "dev.minirender"
    compileSdk = 34

    defaultConfig {
        // 与 tools/build-mobile.sh 里的 NDK API level 保持一致
        minSdk = 24
    }

    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }

    publishing {
        singleVariant("release") { withSourcesJar() }
    }
}

publishing {
    publications {
        register<MavenPublication>("release") {
            groupId = "dev.minirender"
            artifactId = "mini-render"
            version = "0.1.0"
            afterEvaluate { from(components["release"]) }
            pom {
                name.set("mini-render")
                description.set("自绘小程序渲染引擎（不基于 WebView）")
                licenses { license { name.set("MIT") } }
            }
        }
    }
}
