pluginManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
}
plugins {
    id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "Kanari App"
include(":app")
// Local kanari-kotlin: Rust crypto (incl. zkLogin FFI) via UniFFI + JNA.
// Replaces the published Maven artifact so the app always tracks the tree.
includeBuild("../../packages/kanari-kotlin/android") { dependencySubstitution { substitute(module("io.github.jamesatomc:kanari-crypto")).using(project(":kanari-crypto")) } }
 