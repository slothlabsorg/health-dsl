plugins {
    kotlin("jvm") version "2.2.20"
    `maven-publish`
}

group = "com.slothlabs"
version = (findProperty("version") as String?)?.removePrefix("v") ?: "0.1.0"

repositories {
    mavenCentral()
}

dependencies {
    api("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")

    testImplementation(kotlin("test"))
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.10.2")
}

kotlin {
    jvmToolchain(17)
}

java {
    withSourcesJar()
    withJavadocJar()
}

tasks.test {
    useJUnitPlatform()
}

publishing {
    publications {
        create<MavenPublication>("maven") {
            from(components["java"])

            pom {
                name.set("health-dsl")
                description.set(
                    "A tiny Kotlin DSL for declaring service readiness/liveness checks — " +
                        "run concurrently, aggregated correctly, serialized anywhere.",
                )
                url.set("https://github.com/slothlabsorg/health-dsl")

                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }

                developers {
                    developer {
                        id.set("slothlabsorg")
                        name.set("SlothLabs")
                        url.set("https://github.com/slothlabsorg")
                    }
                }

                scm {
                    url.set("https://github.com/slothlabsorg/health-dsl")
                    connection.set("scm:git:https://github.com/slothlabsorg/health-dsl.git")
                    developerConnection.set("scm:git:ssh://git@github.com/slothlabsorg/health-dsl.git")
                }
            }
        }
    }

    repositories {
        // GitHub Packages — only wired up when credentials are present, so local
        // `publishToMavenLocal` and JitPack builds need no GitHub auth.
        val ghUser = providers.gradleProperty("gpr.user").orElse(providers.environmentVariable("GITHUB_ACTOR"))
        val ghToken = providers.gradleProperty("gpr.token").orElse(providers.environmentVariable("GITHUB_TOKEN"))
        if (ghUser.isPresent && ghToken.isPresent) {
            maven {
                name = "GitHubPackages"
                url = uri("https://maven.pkg.github.com/slothlabsorg/health-dsl")
                credentials {
                    username = ghUser.get()
                    password = ghToken.get()
                }
            }
        }
    }
}
