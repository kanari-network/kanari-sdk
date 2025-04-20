plugins {
    kotlin("jvm") version "1.8.0"
    kotlin("plugin.serialization") version "1.8.0"
    id("java-library")
}

group = "io.kanari"
version = "1.0-SNAPSHOT"

repositories {
    mavenCentral()
}

dependencies {
    // Kotlin
    implementation(kotlin("stdlib"))
    
    // Serialization
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.5.0")
    
    // Crypto
    implementation("org.bouncycastle:bcprov-jdk15on:1.70")
    implementation("de.mkammerer:argon2-jvm:2.11")
    implementation("org.bitcoinj:bitcoinj-core:0.16.1")
    
    // Testing
    testImplementation(kotlin("test"))
}

tasks.test {
    useJUnitPlatform()
}

kotlin {
    jvmToolchain(11)
}
