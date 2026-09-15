plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.vanniktech.publish)
}

android {
    namespace = "com.kanari.kanari_crypto"
    compileSdk = 37

    defaultConfig {
        minSdk = 24

        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

dependencies {
    implementation(platform(libs.androidx.compose.bom))

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.compose.foundation)
    implementation(libs.androidx.compose.foundation.layout)
    implementation(libs.androidx.material3)
    implementation(libs.androidx.compose.material.icons.extended)
    implementation(libs.androidx.lifecycle.runtime.compose)

    implementation(libs.kotlinx.serialization.json)
    implementation(libs.jna) { artifact { type = "aar" } }

    debugImplementation(libs.androidx.ui.tooling)
}

mavenPublishing {
    coordinates(
        groupId = "io.github.jamesatomc",
        artifactId = "kanari-crypto",
        version = "0.3.0",
    )

    pom {
        name = "Kanari Crypto (Kotlin)"
        description = "Android / Jetpack Compose library for Kanari cryptography (Rust core via UniFFI: keypair, mnemonic, HD wallet, sign/verify, Blake3, PQC + hybrid curves)."
        inceptionYear = "2026"
        url = "https://github.com/jamesatomc/kanari-sdk"

        licenses {
            license {
                name = "Apache License 2.0"
                url = "https://www.apache.org/licenses/LICENSE-2.0"
            }
        }

        developers {
            developer {
                id = "jamesatomc"
                name = "James Atomc"
                url = "https://github.com/jamesatomc"
            }
        }

        scm {
            url = "https://github.com/jamesatomc/kanari-sdk"
            connection = "scm:git:git://github.com/jamesatomc/kanari-sdk.git"
            developerConnection = "scm:git:ssh://git@github.com/jamesatomc/kanari-sdk.git"
        }
    }

    // Central Portal (Sonatype) publishing + GPG signing.
    // Credentials มาจาก env / gradle.properties (ไม่ต้อง hardcode):
    //   ORG_GRADLE_PROJECT_mavenCentralUsername / mavenCentralPassword
    //   ORG_GRADLE_PROJECT_signingInMemoryKey / signingInMemoryKeyPassword
    publishToMavenCentral(true)
    // sign เฉพาะตอนมี GPG key (publishToMavenLocal จะได้ไม่พัง)
    val hasSigningKey =
        providers.environmentVariable("ORG_GRADLE_PROJECT_signingInMemoryKey").isPresent ||
            project.findProperty("signingInMemoryKey") != null
    if (hasSigningKey) {
        signAllPublications()
    }
}
