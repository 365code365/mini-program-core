// 独立的构建，只为把 SDK 打成 AAR。业务工程不需要 include 它，
// 直接依赖 mavenLocal / 内网 Maven 上的 dev.minirender:mini-render 即可。
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

rootProject.name = "mini-render-sdk"
