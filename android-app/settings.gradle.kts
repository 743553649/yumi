pluginManagement {
    repositories {
        maven { url = uri("http://maven.aliyun.com/repository/google"); isAllowInsecureProtocol = true }
        maven { url = uri("http://maven.aliyun.com/repository/public"); isAllowInsecureProtocol = true }
        maven { url = uri("https://maven.aliyun.com/repository/google") }
        maven { url = uri("https://maven.aliyun.com/repository/public") }
        maven { url = uri("https://mirrors.tencent.com/nexus/repository/maven-public/") }
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        maven { url = uri("http://maven.aliyun.com/repository/google"); isAllowInsecureProtocol = true }
        maven { url = uri("http://maven.aliyun.com/repository/public"); isAllowInsecureProtocol = true }
        maven { url = uri("https://maven.aliyun.com/repository/google") }
        maven { url = uri("https://maven.aliyun.com/repository/public") }
        maven { url = uri("https://mirrors.tencent.com/nexus/repository/maven-public/") }
        google()
        mavenCentral()
    }
}

rootProject.name = "yumi-bridge"
include(":app")
