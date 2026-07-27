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

$quickJsVersion = "2026-06-04"
$quickJsTarget = Join-Path $projectRoot "src-tauri\plugins\mobile-downloader\android\src\main\jniLibs\arm64-v8a\libqjs.so"
$quickJsStamp = "$quickJsTarget.version"
if (!(Test-Path -LiteralPath $quickJsTarget) -or
    !(Test-Path -LiteralPath $quickJsStamp) -or
    (Get-Content -Raw -LiteralPath $quickJsStamp).Trim() -ne $quickJsVersion) {
    $quickJsWork = Join-Path $tauriRoot "target\quickjs-android"
    $quickJsArchive = Join-Path $quickJsWork "quickjs.tar.xz"
    $quickJsSource = Join-Path $quickJsWork "source"
    New-Item -ItemType Directory -Force -Path $quickJsWork | Out-Null
    if (!(Test-Path -LiteralPath $quickJsArchive) -or
        (Get-Item -LiteralPath $quickJsArchive).Length -lt 1024) {
        & curl.exe --fail --location --silent --show-error `
            --output $quickJsArchive `
            "https://bellard.org/quickjs/quickjs-$quickJsVersion.tar.xz"
        if ($LASTEXITCODE -ne 0) {
            throw "Não foi possível baixar o código-fonte oficial do QuickJS."
        }
    }
    if (Test-Path -LiteralPath $quickJsSource) {
        $resolvedQuickJsSource = (Resolve-Path -LiteralPath $quickJsSource).Path
        $resolvedQuickJsWork = (Resolve-Path -LiteralPath $quickJsWork).Path
        if (!$resolvedQuickJsSource.StartsWith($resolvedQuickJsWork, [StringComparison]::OrdinalIgnoreCase)) {
            throw "A pasta temporária do QuickJS ficou fora do diretório esperado."
        }
        Remove-Item -LiteralPath $resolvedQuickJsSource -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $quickJsSource | Out-Null
    & tar.exe -xf $quickJsArchive -C $quickJsSource --strip-components=1
    if ($LASTEXITCODE -ne 0) {
        throw "Não foi possível extrair o código-fonte oficial do QuickJS."
    }
    $clang = Join-Path $ndkRoot "toolchains\llvm\prebuilt\windows-x86_64\bin\aarch64-linux-android33-clang.cmd"
    if (!(Test-Path -LiteralPath $clang)) {
        throw "Compilador Android arm64 do NDK não encontrado."
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $quickJsTarget) | Out-Null
    Push-Location $quickJsSource
    try {
        @"
#include <stdint.h>
const uint8_t qjsc_repl[] = { 0 };
const uint32_t qjsc_repl_size = 0;
"@ | Set-Content -LiteralPath (Join-Path $quickJsSource "qjsc_repl_stub.c") -Encoding ascii
        $quickJsVersionDefine = "-DCONFIG_VERSION=\`"$quickJsVersion\`""
        & $clang -O2 -fPIE -pie -D_GNU_SOURCE $quickJsVersionDefine `
            -o $quickJsTarget qjs.c quickjs.c dtoa.c libregexp.c libunicode.c cutils.c quickjs-libc.c `
            qjsc_repl_stub.c `
            -lm -ldl
        if ($LASTEXITCODE -ne 0 -or !(Test-Path -LiteralPath $quickJsTarget)) {
            throw "Não foi possível compilar o runtime QuickJS para Android arm64."
        }
        Set-Content -LiteralPath $quickJsStamp -Value $quickJsVersion -Encoding ascii
    } finally {
        Pop-Location
    }
}

Push-Location $projectRoot
try {
    if (!(Test-Path -LiteralPath $androidRoot)) {
        & npm run android:init
        if ($LASTEXITCODE -ne 0) {
            throw "Falha ao inicializar o projeto Tauri Android."
        }
    }

    $generatedBuildFile = Join-Path $androidRoot "build.gradle.kts"
    $generatedBuild = Get-Content -Raw -LiteralPath $generatedBuildFile
    $updatedGeneratedBuild = $generatedBuild -replace (
        'org\.jetbrains\.kotlin:kotlin-gradle-plugin:[^"]+'
    ), 'org.jetbrains.kotlin:kotlin-gradle-plugin:2.2.20'
    if ($updatedGeneratedBuild -ne $generatedBuild) {
        Set-Content -LiteralPath $generatedBuildFile -Value $updatedGeneratedBuild -Encoding utf8
    }

    $generatedAppBuildFile = Join-Path $androidRoot "app\build.gradle.kts"
    $generatedAppBuild = Get-Content -Raw -LiteralPath $generatedAppBuildFile
    $androidDependencyVersions = [ordered]@{
        'androidx\.webkit:webkit:[^"]+' = 'androidx.webkit:webkit:1.16.0'
        'androidx\.activity:activity-ktx:[^"]+' = 'androidx.activity:activity-ktx:1.13.0'
        'com\.google\.android\.material:material:[^"]+' = 'com.google.android.material:material:1.14.0'
        'androidx\.lifecycle:lifecycle-process:[^"]+' = 'androidx.lifecycle:lifecycle-process:2.11.0'
        'androidx\.test\.ext:junit:[^"]+' = 'androidx.test.ext:junit:1.3.0'
        'androidx\.test\.espresso:espresso-core:[^"]+' = 'androidx.test.espresso:espresso-core:3.7.0'
    }
    $updatedGeneratedAppBuild = $generatedAppBuild
    foreach ($dependencyPattern in $androidDependencyVersions.Keys) {
        $updatedGeneratedAppBuild = $updatedGeneratedAppBuild -replace (
            $dependencyPattern
        ), $androidDependencyVersions[$dependencyPattern]
    }
    if ($updatedGeneratedAppBuild -ne $generatedAppBuild) {
        Set-Content -LiteralPath $generatedAppBuildFile -Value $updatedGeneratedAppBuild -Encoding utf8
    }

    $androidIconSource = Join-Path $tauriRoot "icons\android"
    $androidResourceDestination = Join-Path $androidRoot "app\src\main\res"
    if (!(Test-Path -LiteralPath $androidIconSource)) {
        throw "Os recursos do ícone Android não foram encontrados."
    }
    Copy-Item -Path (Join-Path $androidIconSource "*") `
        -Destination $androidResourceDestination -Recurse -Force

    $generatedManifestFile = Join-Path $androidRoot "app\src\main\AndroidManifest.xml"
    $generatedManifest = Get-Content -Raw -LiteralPath $generatedManifestFile
    if ($generatedManifest -notmatch 'android:roundIcon=') {
        $updatedGeneratedManifest = $generatedManifest -replace (
            'android:icon="@mipmap/ic_launcher"'
        ), "android:icon=`"@mipmap/ic_launcher`"`r`n        android:roundIcon=`"@mipmap/ic_launcher_round`""
        Set-Content -LiteralPath $generatedManifestFile `
            -Value $updatedGeneratedManifest -Encoding utf8
    }

    $verificationSource = Join-Path $tauriRoot "android\verification-metadata.xml"
    $verificationDirectory = Join-Path $androidRoot "gradle"
    if (!(Test-Path -LiteralPath $verificationSource)) {
        throw "O catálogo SHA-256 das dependências Android não foi encontrado."
    }
    New-Item -ItemType Directory -Force -Path $verificationDirectory | Out-Null
    Copy-Item -LiteralPath $verificationSource `
        -Destination (Join-Path $verificationDirectory "verification-metadata.xml") -Force

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
        $keyDirectory = Join-Path $env:LOCALAPPDATA "YT-DLP-Deck\signing"
        $keyStore = Join-Path $keyDirectory "release.keystore"
        $signingProperties = Join-Path $keyDirectory "signing.properties"
        New-Item -ItemType Directory -Force -Path $keyDirectory | Out-Null
        if (Test-Path -LiteralPath $signingProperties) {
            $localSigning = ConvertFrom-StringData (Get-Content -Raw -LiteralPath $signingProperties)
            $keyAlias = $localSigning.KeyAlias
            $storePassword = $localSigning.StorePassword
            $keyPassword = $localSigning.KeyPassword
        } else {
            $keyAlias = "yt-dlp-deck-release"
            $passwordBytes = New-Object byte[] 32
            $passwordGenerator = [Security.Cryptography.RandomNumberGenerator]::Create()
            try {
                $passwordGenerator.GetBytes($passwordBytes)
            } finally {
                $passwordGenerator.Dispose()
            }
            $storePassword = [Convert]::ToBase64String($passwordBytes)
            $keyPassword = $storePassword
            @"
KeyAlias=$keyAlias
StorePassword=$storePassword
KeyPassword=$keyPassword
"@ | Set-Content -LiteralPath $signingProperties -Encoding utf8
        }
        if (!(Test-Path -LiteralPath $keyStore)) {
            & (Join-Path $jdkRoot "bin\keytool.exe") `
                -genkeypair -keystore $keyStore -storepass $storePassword `
                -alias $keyAlias -keypass $keyPassword `
                -dname "CN=YT-DLP Deck Release,O=YT-DLP Deck,C=BR" `
                -keyalg RSA -keysize 4096 -validity 10000
            if ($LASTEXITCODE -ne 0) {
                throw "Não foi possível gerar a chave local de produção."
            }
        }
        $signatureLabel = "produção local persistente"
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
        $env:YTDLP_DECK_SIGN_STORE_PASS = $storePassword
        $env:YTDLP_DECK_SIGN_KEY_PASS = $keyPassword
        try {
            & (Join-Path $buildTools "apksigner.bat") sign `
                --ks $keyStore --ks-key-alias $keyAlias `
                --ks-pass "env:YTDLP_DECK_SIGN_STORE_PASS" `
                --key-pass "env:YTDLP_DECK_SIGN_KEY_PASS" `
                --out $finalApk $alignedApk
        } finally {
            Remove-Item Env:YTDLP_DECK_SIGN_STORE_PASS -ErrorAction SilentlyContinue
            Remove-Item Env:YTDLP_DECK_SIGN_KEY_PASS -ErrorAction SilentlyContinue
        }
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
