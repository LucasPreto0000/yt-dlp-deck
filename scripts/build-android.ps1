$ErrorActionPreference = "Stop"

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$tauriRoot = Join-Path $projectRoot "src-tauri"
$androidRoot = Join-Path $tauriRoot "gen\android"
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
$jdkRoot = if ($env:JAVA_HOME) {
    $env:JAVA_HOME
} else {
    (Get-ChildItem -LiteralPath "C:\Program Files\Eclipse Adoptium" -Directory |
        Where-Object Name -Like "jdk-17*" |
        Sort-Object Name -Descending |
        Select-Object -First 1).FullName
}

if (!$jdkRoot -or !(Test-Path -LiteralPath (Join-Path $jdkRoot "bin\java.exe"))) {
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
        & .\gradlew.bat :app:assembleArm64Release -x rustBuildArm64Release
        if ($LASTEXITCODE -ne 0) {
            throw "O Gradle não conseguiu montar o APK Android."
        }
    } finally {
        Pop-Location
    }

    $unsignedApk = (
        Get-ChildItem -Recurse -LiteralPath (Join-Path $androidRoot "app\build\outputs\apk") -Filter "*release-unsigned.apk" |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1
    ).FullName
    if (!$unsignedApk) {
        throw "O APK não assinado não foi encontrado."
    }

    $buildTools = (
        Get-ChildItem -LiteralPath (Join-Path $sdkRoot "build-tools") -Directory |
            Sort-Object { [version]$_.Name } -Descending |
            Select-Object -First 1
    ).FullName
    $keyDirectory = Join-Path $env:USERPROFILE ".android"
    $keyStore = Join-Path $keyDirectory "debug.keystore"
    New-Item -ItemType Directory -Force -Path $keyDirectory | Out-Null
    if (!(Test-Path -LiteralPath $keyStore)) {
        & (Join-Path $jdkRoot "bin\keytool.exe") `
            -genkeypair -keystore $keyStore -storepass android `
            -alias androiddebugkey -keypass android `
            -dname "CN=Android Debug,O=Android,C=US" `
            -keyalg RSA -keysize 2048 -validity 10000
        if ($LASTEXITCODE -ne 0) {
            throw "Não foi possível gerar a chave de desenvolvimento."
        }
    }

    $outputDirectory = Join-Path $tauriRoot "target\android"
    New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
    $alignedApk = Join-Path $outputDirectory "YT-DLP-Deck-Android-v1.0.0-arm64-aligned.apk"
    $finalApk = Join-Path $outputDirectory "YT-DLP-Deck-Android-v1.0.0-arm64.apk"
    & (Join-Path $buildTools "zipalign.exe") -f 4 $unsignedApk $alignedApk
    if ($LASTEXITCODE -ne 0) {
        throw "Falha ao alinhar o APK."
    }
    & (Join-Path $buildTools "apksigner.bat") sign `
        --ks $keyStore --ks-key-alias androiddebugkey `
        --ks-pass pass:android --key-pass pass:android `
        --out $finalApk $alignedApk
    if ($LASTEXITCODE -ne 0) {
        throw "Falha ao assinar o APK."
    }
    & (Join-Path $buildTools "apksigner.bat") verify --verbose $finalApk
    if ($LASTEXITCODE -ne 0) {
        throw "A assinatura do APK não passou na verificação."
    }

    Write-Output ""
    Write-Output "APK Android gerado com sucesso:"
    Write-Output $finalApk
} finally {
    Pop-Location
}
