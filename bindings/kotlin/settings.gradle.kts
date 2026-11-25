pluginManagement {
    repositories {
        // Resolve the Kotlin plugin from Maven Central, not plugins.gradle.org.
        //
        // The Gradle Plugin Portal front-loads requests into repo.maven.apache.org
        // on a rate-limit budget shared by every hosted runner, and this job lost
        // three releases in a row to `429 Too Many Requests` while fetching a
        // single POM. Central publishes the same artifacts and answers this
        // workflow reliably. Gradle's retry settings do not cover a 429, so
        // retrying is not an option.
        mavenCentral()
        gradlePluginPortal()
    }
}

rootProject.name = "novelai-image-sdk"
