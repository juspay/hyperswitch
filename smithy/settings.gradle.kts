pluginManagement {
    repositories {
        mavenCentral()
        gradlePluginPortal()
    }

    plugins {
        id("software.amazon.smithy") version "0.5.1"
    }
}

rootProject.name = "hyperswitch-sdk-generator"
