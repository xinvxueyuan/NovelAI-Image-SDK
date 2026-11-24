// Kotlin/JVM binding for novelai-image-sdk.
//
// UniFFI's Kotlin generator emits one .kt file that talks to the cdylib through
// JNA. The native libraries are packaged as resources and extracted at runtime
// by NativeLoader, because a JAR cannot load a .dll/.so straight from its own
// archive.

plugins {
    `java-library`
    `maven-publish`
    kotlin("jvm") version "2.1.0"
}

group = "io.github.xinvxueyuan"
// Kept in step with Cargo.toml by `cargo xtask sync-versions`.
version = "0.1.0"

repositories {
    mavenCentral()
}

dependencies {
    // UniFFI's Kotlin bindings use JNA for the FFI calls.
    api("net.java.dev.jna:jna:5.15.0")
    // Generated async methods use `suspendCancellableCoroutine`.
    api("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
    testImplementation(kotlin("test"))
}

kotlin {
    jvmToolchain(17)
}

sourceSets {
    main {
        kotlin.srcDir("generated")
        // `resources/<os>-<arch>/` holds the native libraries produced by
        // `cargo xtask bindings --language kotlin` plus the release build.
        resources.srcDir("src/main/resources")
    }
}

tasks.withType<Test> {
    useJUnitPlatform()
}

publishing {
    publications {
        create<MavenPublication>("maven") {
            from(components["java"])
            pom {
                name.set("novelai-image-sdk")
                description.set(
                    "NovelAI image-generation SDK with Chrome-fingerprinted transport",
                )
                url.set("https://github.com/xinvxueyuan/NovelAI-Image-SDK")
                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }
                scm {
                    connection.set("scm:git:https://github.com/xinvxueyuan/NovelAI-Image-SDK.git")
                    developerConnection.set("scm:git:ssh://git@github.com/xinvxueyuan/NovelAI-Image-SDK.git")
                    url.set("https://github.com/xinvxueyuan/NovelAI-Image-SDK")
                }
            }
        }
    }
    repositories {
        // The release workflow sets these for Maven Central.
        maven {
            name = "central"
            url = uri(
                System.getenv("MAVEN_CENTRAL_URL")
                    ?: "https://s01.oss.sonatype.org/service/local/staging/deploy/maven2/",
            )
            credentials {
                username = System.getenv("MAVEN_CENTRAL_USERNAME")
                password = System.getenv("MAVEN_CENTRAL_PASSWORD")
            }
        }
    }
}
