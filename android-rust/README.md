# Android em Rust

Alvo independente com pacote `com.ytdlpdeck.rust`. O APK usa `NativeActivity`
e a WebView do sistema. Rust gera as telas, recebe os formulários e baixa
arquivos HTTPS diretos para Downloads usando MediaStore. A página do app
bloqueia scripts e permite o player oficial do YouTube em uma moldura
isolada. Os estilos, fontes, ícones e shaders são reutilizados durante o build.

Esta versão inclui as cinco etapas, busca e capas do YouTube, fundo OpenGL ES,
console de transferência e histórico local. A transferência aceita links
diretos. Extração de mídia de páginas, conversão de formato e importação de
cookies ainda precisam de implementação.

## Compilar no Windows

Com o SDK Android 35, NDK 27, JDK, `cargo-apk` e alvo Rust ARM64 instalados:

```powershell
npm ci
rustup target add aarch64-linux-android
$env:ANDROID_NDK_HOME = $env:NDK_HOME
cargo test --locked --manifest-path android-rust/Cargo.toml
cargo apk build --lib --manifest-path android-rust/Cargo.toml --target aarch64-linux-android
```

O APK de teste fica em `android-rust/target/debug/apk/YT-DLP-Deck-Rust-Android.apk`.
