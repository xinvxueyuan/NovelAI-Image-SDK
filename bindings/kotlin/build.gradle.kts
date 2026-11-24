// Kotlin/JVM binding for novelai-image-sdk.
//
// UniFFI's Kotlin generator emits one .kt file that talks to the cdylib through
// JNA. The native libraries are packaged as resources and extracted at runtime
// by NativeLoader, because a JAR cannot load a .dll/.so straight from its own
// archive.

plugins {
    `java-library`
    `maven-publish`
    signing
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

// Maven Central requires sources and javadoc artifacts for every publication.
java {
    withSourcesJar()
    withJavadocJar()
}

sourceSets {
    main {
        kotlin.srcDir("generated")
        // `src/main/resources/<os>-<arch>/` holds the native libraries that
        // `cargo xtask bindings --language kotlin` copies in; Gradle already
        // includes that directory, so it must not be added twice.
    }
}

tasks.withType<Test> {
    useJUnitPlatform()
}

// Maven Central rejects artifacts without a detached signature. The release
// workflow supplies the key through these variables; a local build without
// them still works and simply skips signing.
signing {
    val key = System.getenv("SIGNING_KEY")
    val password = System.getenv("SIGNING_PASSWORD")
    if (!key.isNullOrBlank()) {
        useInMemoryPgpKeys(key, password)
        sign(publishing.publications)
    }
}

publishing {
    publications {
        create<MavenPublication>("maven") {
            from(components["java"])
            pom {
                name.set("novelai-image-sdk")
                // Central validates these three coordinates.
                groupId = "io.github.xinvxueyuan"
                artifactId = "novelai-image-sdk"
                version = project.version.toString()
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
            // Central Portal's OSSRH-compatible endpoint. The old
            // `s01.oss.sonatype.org` host is retired.
            url = uri(
                System.getenv("MAVEN_CENTRAL_URL")
                    ?: "https://ossrh-staging-api.central.sonatype.com/service/local/staging/deploy/maven2/",
            )
            credentials {
                username = System.getenv("MAVEN_CENTRAL_USERNAME")
                password = System.getenv("MAVEN_CENTRAL_PASSWORD")
            }
        }
    }
}
