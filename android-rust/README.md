# Android em Rust

Alvo independente com pacote `com.ytdlpdeck.rust`. O APK usa `NativeActivity`
e a WebView do sistema. Rust gera as telas, recebe os formulários e baixa
arquivos HTTPS diretos para Downloads usando MediaStore. A página do app
bloqueia scripts e permite o player oficial do YouTube em uma moldura
isolada. Os estilos, fontes, ícones e shaders são reutilizados durante o build.

Esta versão inclui as cinco etapas, busca e capas do YouTube, fundo OpenGL ES,
console de transferência e histórico local. A transferência aceita links
diretos, vídeos (MP4) e capas de publicações públicas do TikTok, incluindo links
encurtados. Extração de outras plataformas, conversão de formato e importação
de cookies ainda precisam de implementação.

## Compilar no Windows

Com o SDK Android 35, NDK 27, JDK, `cargo-apk` e alvo Rust ARM64 instalados:

```powershell
npm ci
rustup target add aarch64-linux-android
cargo test --locked --manifest-path android-rust/Cargo.toml
npm run android:rust:build -- -Development
```

O APK de teste fica em `android-rust/target/dist/dev/`. Sem `-Development`,
o comando usa a chave local de release do projeto e escreve em
`android-rust/target/dist/signed/`. Ambos usam Rust otimizado e incremental,
sem rodar Gradle ou empacotar o frontend desktop. Um APK validado é
reutilizado quando as entradas permanecem iguais; use `-Force` para
reempacotar mesmo sem alterações.
