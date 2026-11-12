plugins { id("com.android.application"); id("org.jetbrains.kotlin.android") }
android {
    namespace = "hu.dormmenu"
    compileSdk = 35
    buildFeatures { buildConfig = true }
    defaultConfig { applicationId = "hu.dormmenu"; minSdk = 26; targetSdk = 35; versionCode = 1; versionName = "0.1.0" }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
}
