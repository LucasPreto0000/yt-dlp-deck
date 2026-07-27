param(
    [switch]$Bundle
)

$ErrorActionPreference = "Stop"

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$tauriRoot = Join-Path $projectRoot "src-tauri"
$androidRoot = Join-Path $tauriRoot "gen\android"
$appVersion = (Get-Content -Raw -LiteralPath (Join-Path $tauriRoot "tauri.conf.json") |
    ConvertFrom-Json).version
$sdkRoot = if ($env:ANDROID_HOME) {
    $env:ANDROID_HOME
} else {
    Join-Path $env:LOCALAPPDATA "Android\Sdk"
}
$ndkRoot = if ($env:NDK_HOME) {
    $env:NDK_HOME
} else {
    Join-Path $sdkRoot "ndk\27.3.13750724"
}
$configuredJdk = $env:JAVA_HOME
$configuredJavac = if ($configuredJdk) { Join-Path $configuredJdk "bin\javac.exe" } else { $null }
$jdkRoot = if ($configuredJavac -and (Test-Path -LiteralPath $configuredJavac)) {
    $configuredJdk
} else {
    (Get-ChildItem -LiteralPath "C:\Program Files\Eclipse Adoptium" -Directory -ErrorAction SilentlyContinue |
        Where-Object Name -Like "jdk-17*" |
        Sort-Object Name -Descending |
        Select-Object -First 1).FullName
}

if (!$jdkRoot -or !(Test-Path -LiteralPath (Join-Path $jdkRoot "bin\javac.exe"))) {
    throw "JDK 17 não encontrado. Instale o Temurin 17 antes de compilar."
}
if (!(Test-Path -LiteralPath (Join-Path $sdkRoot "platforms\android-36"))) {
    throw "Android SDK 36 não encontrado em $sdkRoot."
}
if (!(Test-Path -LiteralPath $ndkRoot)) {
    throw "Android NDK 27.3.13750724 não encontrado em $ndkRoot."
}

$env:JAVA_HOME = $jdkRoot
$env:ANDROID_HOME = $sdkRoot
$env:ANDROID_SDK_ROOT = $sdkRoot
$env:NDK_HOME = $ndkRoot

Push-Location $projectRoot
try {
    if (!(Test-Path -LiteralPath $androidRoot)) {
        & npm run android:init
        if ($LASTEXITCODE -ne 0) {
            throw "Falha ao inicializar o projeto Tauri Android."
        }
    }

    & npm run tauri android build -- --apk --target aarch64
    $tauriExit = $LASTEXITCODE

    $rustLibrary = Join-Path $tauriRoot "target\aarch64-linux-android\release\libyt_dlp_deck_lib.so"
    if (!(Test-Path -LiteralPath $rustLibrary)) {
        throw "A biblioteca Rust para Android não foi gerada (código Tauri: $tauriExit)."
    }

    $jniDirectory = Join-Path $androidRoot "app\src\main\jniLibs\arm64-v8a"
    New-Item -ItemType Directory -Force -Path $jniDirectory | Out-Null
    $resolvedAndroid = (Resolve-Path -LiteralPath $androidRoot).Path
    $resolvedJni = (Resolve-Path -LiteralPath $jniDirectory).Path
    if (!$resolvedJni.StartsWith($resolvedAndroid, [StringComparison]::OrdinalIgnoreCase)) {
        throw "O destino JNI calculado está fora do projeto Android."
    }
    Copy-Item -LiteralPath $rustLibrary -Destination (Join-Path $resolvedJni "libyt_dlp_deck_lib.so") -Force

    Push-Location $androidRoot
    try {
        $gradleTask = if ($Bundle) { ":app:bundleArm64Release" } else { ":app:assembleArm64Release" }
        & .\gradlew.bat $gradleTask -x rustBuildArm64Release
        if ($LASTEXITCODE -ne 0) {
            throw "O Gradle não conseguiu montar o pacote Android."
        }
    } finally {
        Pop-Location
    }

    $buildTools = (
        Get-ChildItem -LiteralPath (Join-Path $sdkRoot "build-tools") -Directory |
            Sort-Object { [version]$_.Name } -Descending |
            Select-Object -First 1
    ).FullName

    $productionVariables = @(
        $env:YTDLP_ANDROID_KEYSTORE,
        $env:YTDLP_ANDROID_KEY_ALIAS,
        $env:YTDLP_ANDROID_STORE_PASSWORD,
        $env:YTDLP_ANDROID_KEY_PASSWORD
    )
    $productionCount = @($productionVariables | Where-Object { ![string]::IsNullOrWhiteSpace($_) }).Count
    if ($productionCount -ne 0 -and $productionCount -ne $productionVariables.Count) {
        throw "Defina todas as variáveis YTDLP_ANDROID_KEYSTORE, KEY_ALIAS, STORE_PASSWORD e KEY_PASSWORD."
    }
    $useProductionKey = $productionCount -eq $productionVariables.Count

    if ($useProductionKey) {
        $keyStore = (Resolve-Path -LiteralPath $env:YTDLP_ANDROID_KEYSTORE).Path
        $keyAlias = $env:YTDLP_ANDROID_KEY_ALIAS
        $storePassword = $env:YTDLP_ANDROID_STORE_PASSWORD
        $keyPassword = $env:YTDLP_ANDROID_KEY_PASSWORD
        $signatureLabel = "produção"
    } else {
        $keyDirectory = Join-Path $env:USERPROFILE ".android"
        $keyStore = Join-Path $keyDirectory "debug.keystore"
        $keyAlias = "androiddebugkey"
        $storePassword = "android"
        $keyPassword = "android"
        $signatureLabel = "desenvolvimento"
        New-Item -ItemType Directory -Force -Path $keyDirectory | Out-Null
        if (!(Test-Path -LiteralPath $keyStore)) {
            & (Join-Path $jdkRoot "bin\keytool.exe") `
                -genkeypair -keystore $keyStore -storepass $storePassword `
                -alias $keyAlias -keypass $keyPassword `
                -dname "CN=Android Debug,O=Android,C=US" `
                -keyalg RSA -keysize 2048 -validity 10000
            if ($LASTEXITCODE -ne 0) {
                throw "Não foi possível gerar a chave de desenvolvimento."
            }
        }
    }

    $outputDirectory = Join-Path $tauriRoot "target\android"
    New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
    if ($Bundle) {
        $unsignedBundle = (
            Get-ChildItem -Recurse -LiteralPath (Join-Path $androidRoot "app\build\outputs\bundle") -Filter "*.aab" |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
        ).FullName
        if (!$unsignedBundle) {
            throw "O Android App Bundle não foi encontrado."
        }
        $finalBundle = Join-Path $outputDirectory "YT-DLP-Deck-Android-v$appVersion-arm64.aab"
        Copy-Item -LiteralPath $unsignedBundle -Destination $finalBundle -Force
        & (Join-Path $jdkRoot "bin\jarsigner.exe") `
            -keystore $keyStore -storepass $storePassword -keypass $keyPassword `
            -sigalg SHA256withRSA -digestalg SHA-256 $finalBundle $keyAlias
        if ($LASTEXITCODE -ne 0) {
            throw "Falha ao assinar o Android App Bundle."
        }
        & (Join-Path $jdkRoot "bin\jarsigner.exe") -verify $finalBundle
        if ($LASTEXITCODE -ne 0) {
            throw "A assinatura do Android App Bundle não passou na verificação."
        }
        Write-Output ""
        Write-Output "AAB Android gerado com assinatura de ${signatureLabel}:"
        Write-Output $finalBundle
    } else {
        $unsignedApk = (
            Get-ChildItem -Recurse -LiteralPath (Join-Path $androidRoot "app\build\outputs\apk") -Filter "*release-unsigned.apk" |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
        ).FullName
        if (!$unsignedApk) {
            throw "O APK não assinado não foi encontrado."
        }
        $alignedApk = Join-Path $outputDirectory "YT-DLP-Deck-Android-v$appVersion-arm64-aligned.apk"
        $finalApk = Join-Path $outputDirectory "YT-DLP-Deck-Android-v$appVersion-arm64.apk"
        & (Join-Path $buildTools "zipalign.exe") -f 4 $unsignedApk $alignedApk
        if ($LASTEXITCODE -ne 0) {
            throw "Falha ao alinhar o APK."
        }
        & (Join-Path $buildTools "apksigner.bat") sign `
            --ks $keyStore --ks-key-alias $keyAlias `
            --ks-pass "pass:$storePassword" --key-pass "pass:$keyPassword" `
            --out $finalApk $alignedApk
        if ($LASTEXITCODE -ne 0) {
            throw "Falha ao assinar o APK."
        }
        & (Join-Path $buildTools "apksigner.bat") verify --verbose $finalApk
        if ($LASTEXITCODE -ne 0) {
            throw "A assinatura do APK não passou na verificação."
        }
        Write-Output ""
        Write-Output "APK Android gerado com assinatura de ${signatureLabel}:"
        Write-Output $finalApk
    }
} finally {
    Pop-Location
}
