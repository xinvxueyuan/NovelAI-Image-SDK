// Loads the packaged native library for the current platform.
//
// UniFFI's generated code registers the library through JNA's direct mapping,
// which resolves `novelai_image_sdk_ffi` via `jna.library.path` and the system
// loader. A JAR cannot hand a `.dll`/`.so` to the system loader, so this object
// extracts the bundled library to a temporary directory and adds it to
// `jna.library.path`.
//
// Call `NativeLoader.load()` before touching any other type from this package,
// or set `NOVELAI_SDK_AUTOLOAD=1` to have the object do it on class load.

package io.github.xinvxueyuan.novelai.image.sdk

import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption

object NativeLoader {
    private const val LIBRARY_NAME = "novelai_image_sdk_ffi"

    @Volatile
    private var loaded = false

    init {
        if (System.getenv("NOVELAI_SDK_AUTOLOAD") != "0") {
            load()
        }
    }

    @JvmStatic
    fun load() {
        if (loaded) {
            return
        }
        synchronized(this) {
            if (loaded) {
                return
            }
            val platform = platformTag()
            val extension = when {
                platform.startsWith("win") -> "dll"
                platform.startsWith("mac") -> "dylib"
                else -> "so"
            }
            val resourceName = "$platform/lib$LIBRARY_NAME.$extension"

            val stream = NativeLoader::class.java.classLoader.getResourceAsStream(resourceName)
                ?: error(
                    "No native library for $platform. Expected $resourceName in the JAR; " +
                        "build with `cargo xtask bindings --language kotlin` and assemble " +
                        "the release artifact.",
                )

            val directory = Files.createTempDirectory("novelai-image-sdk").toFile()
            directory.deleteOnExit()
            val target = File(directory, libraryFileName(extension))
            target.deleteOnExit()

            stream.use { input ->
                Files.copy(input, target.toPath(), StandardCopyOption.REPLACE_EXISTING)
            }

            // Prepend so a packaged library wins over anything on the system.
            val existing = System.getProperty("jna.library.path")
            System.setProperty(
                "jna.library.path",
                if (existing.isNullOrBlank()) directory.absolutePath
                else "${directory.absolutePath}${File.pathSeparator}$existing",
            )
            loaded = true
        }
    }

    private fun libraryFileName(extension: String): String =
        if (extension == "dll") "$LIBRARY_NAME.dll" else "lib$LIBRARY_NAME.$extension"

    private fun platformTag(): String {
        val os = System.getProperty("os.name").lowercase()
        val arch = System.getProperty("os.arch").lowercase()
        val osTag = when {
            os.contains("win") -> "win"
            os.contains("mac") -> "mac"
            else -> "linux"
        }
        val archTag = when {
            arch.contains("aarch64") || arch.contains("arm64") -> "arm64"
            else -> "x64"
        }
        return "$osTag-$archTag"
    }
}
