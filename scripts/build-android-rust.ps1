param([switch]$Development, [switch]$Force)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $root
try {
    if (!$env:ANDROID_NDK_HOME) { $env:ANDROID_NDK_HOME = $env:NDK_HOME }
    $sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { $env:ANDROID_SDK_ROOT }
    if (!$sdk -or !$env:ANDROID_NDK_HOME) { throw 'Configure o SDK e o NDK Android.' }
    $buildTools = (Get-ChildItem -LiteralPath (Join-Path $sdk 'build-tools') -Directory |
        Where-Object Name -Match '^\d+\.\d+\.\d+$' |
        Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1).FullName
    $signer = Join-Path $buildTools 'apksigner.bat'
    $debugKey = Join-Path $env:USERPROFILE '.android/debug.keystore'
    if ($Development) {
        $key = $debugKey
        $mode = 'dev'
    } else {
        $signingDir = Join-Path $env:LOCALAPPDATA 'YT-DLP-Deck/signing'
        $key = Join-Path $signingDir 'release.keystore'
        $mode = 'signed'
    }
    if (!(Test-Path -LiteralPath $key)) { throw "Chave Android não encontrada: $key" }
    if (!(Test-Path -LiteralPath 'node_modules/lucide-react/package.json')) {
        throw 'Execute npm ci para preparar os ícones e as fontes compartilhadas.'
    }
    $versionLine = Get-Content android-rust/Cargo.toml | Where-Object { $_ -match '^version\s*=\s*"(\d+\.\d+\.\d+)"' } | Select-Object -First 1
    $version = [regex]::Match($versionLine, '\d+\.\d+\.\d+').Value
    $target = 'arm64-v8a'
    $output = Join-Path $root "android-rust/target/dist/$mode"
    $name = "YT-DLP-Deck-Rust-Android-v$version-arm64.apk"
    $apk = Join-Path $output $name
    $stamp = Join-Path $output "$name.fingerprint"
    $files = Get-ChildItem -File -Recurse -LiteralPath @(
        'android-rust/src', 'android-rust/res', 'src/styles'
    )
    $inputs = @($files.FullName) + @(
        'android-rust/build.rs', 'android-rust/Cargo.toml', 'android-rust/Cargo.lock',
        'src/styles.css', 'src/config/motion.ts', 'src/visuals/gpuBackdrop.ts',
        'package-lock.json', '.cargo/config.toml', $PSCommandPath, $key,
        (Join-Path $env:ANDROID_NDK_HOME 'source.properties'), (Get-Command cargo-apk.exe).Source
    )
    $material = @($buildTools, $env:ANDROID_NDK_HOME, $mode, (rustc -vV),
        $env:RUSTFLAGS, $env:CARGO_ENCODED_RUSTFLAGS, $env:CARGO_BUILD_RUSTFLAGS, $env:RUSTUP_TOOLCHAIN)
    $material += $inputs | Sort-Object | ForEach-Object {
        "$_ $((Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash)"
    }
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $fingerprint = [BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes(($material -join "`n"))))
    } finally { $sha.Dispose() }
    if (!$Force -and (Test-Path -LiteralPath $apk) -and (Test-Path -LiteralPath $stamp)) {
        $apkHash = (Get-FileHash -LiteralPath $apk -Algorithm SHA256).Hash
        if ((Get-Content -Raw -LiteralPath $stamp).Trim() -eq "$fingerprint $apkHash") {
            Write-Output "APK sem alterações: $apk"
            return
        }
    }
    $env:CARGO_TARGET_DIR = Join-Path $root 'android-rust/target'
    # cargo-apk signs its intermediate output; production signing is done below.
    $env:CARGO_APK_RELEASE_KEYSTORE = $debugKey
    $env:CARGO_APK_RELEASE_KEYSTORE_PASSWORD = 'android'
    cargo apk build --lib --release --target aarch64-linux-android --manifest-path android-rust/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw 'Falha no build Rust para Android.' }
    $intermediate = Join-Path $env:CARGO_TARGET_DIR 'release/apk/YT-DLP-Deck-Rust-Android.apk'
    New-Item -ItemType Directory -Force -Path $output | Out-Null
    if ($Development) {
        Copy-Item -LiteralPath $intermediate -Destination $apk -Force
    } else {
        $settings = ConvertFrom-StringData (Get-Content -Raw -LiteralPath (Join-Path $signingDir 'signing.properties'))
        try {
            $env:DECK_RUST_APK_STORE_PASS = $settings.StorePassword
            $env:DECK_RUST_APK_KEY_PASS = $settings.KeyPassword
            & $signer sign --ks $key --ks-key-alias $settings.KeyAlias --ks-pass env:DECK_RUST_APK_STORE_PASS --key-pass env:DECK_RUST_APK_KEY_PASS --v4-signing-enabled false --out $apk $intermediate
            if ($LASTEXITCODE -ne 0) { throw 'Falha ao assinar o APK de release.' }
        } finally {
            Remove-Item Env:DECK_RUST_APK_STORE_PASS,Env:DECK_RUST_APK_KEY_PASS -ErrorAction SilentlyContinue
        }
    }
    & $signer verify $apk
    if ($LASTEXITCODE -ne 0) { throw 'A assinatura do APK não passou na verificação.' }
    $badging = (& (Join-Path $buildTools 'aapt.exe') dump badging $apk) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $badging -notmatch "versionName='$([regex]::Escape($version))'" -or
        $badging -notmatch "native-code: '$target'" -or $badging -notmatch "sdkVersion:'33'" -or
        $badging -match 'application-debuggable') {
        throw 'O APK não corresponde à versão Android 13+ ARM64 esperada.'
    }
    $apkHash = (Get-FileHash -LiteralPath $apk -Algorithm SHA256).Hash
    Set-Content -LiteralPath $stamp -Value "$fingerprint $apkHash" -Encoding ascii
    Write-Output "APK gerado: $apk"
} finally { Pop-Location }
