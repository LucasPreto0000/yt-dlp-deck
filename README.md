# YT-DLP Deck

Aplicativo para Windows e Android construído com Tauri 2, Rust, React,
TypeScript, Kotlin e Python.

FEITO INTEIRAMENTE POR IA, ENTAO TERÁ BUGS!!!!

## Downloads oficiais

- [Baixar para Windows (`yt-dlp-deck.exe`)](https://github.com/LucasPreto0000/yt-dlp-deck/releases/download/v1.1.3/yt-dlp-deck.exe)
- [Baixar para Android ARM64 (`.apk`)](https://github.com/LucasPreto0000/yt-dlp-deck/releases/download/v1.1.3/YT-DLP-Deck-Android-v1.1.3-arm64.apk)
- [Ver a release completa e as notas da versão](https://github.com/LucasPreto0000/yt-dlp-deck/releases/tag/v1.1.3)

O APK é compatível com Android 7 ou superior em aparelhos ARM64. No Android,
pode ser necessário permitir a instalação de aplicativos desconhecidos.

## Usar no Windows

Ao abrir, o aplicativo verifica se estes dois arquivos estão na mesma pasta do
`yt-dlp-deck.exe`:

1. `yt-dlp.exe`;
2. `ffmpeg.exe`.

Se algum estiver ausente, a interface mostra um aviso e permite checar novamente
depois que os arquivos forem colocados na pasta.

Os arquivos baixados ficam em:

`Downloads\YT-DLP Deck\<Plataforma>`

## Desenvolvimento

```powershell
npm install
npm run desktop:dev
```

Para gerar novamente o `.exe` e o instalador:

```powershell
npm run desktop:build
```

### Android

A versão Android é independente: incorpora Python 3.14, yt-dlp e FFmpegKit,
sem exigir Termux ou arquivos externos. Ela salva a mídia em:

`Downloads/YT-DLP Deck/<Plataforma>`

Recursos móveis:

- download em primeiro plano com notificação de progresso;
- pausar, retomar e cancelar;
- console completo do yt-dlp e FFmpeg;
- histórico persistente para abrir, compartilhar ou excluir arquivos;
- seletor nativo da pasta de destino;
- suporte ao menu Compartilhar do Android;
- importação segura de `cookies.txt`;
- opção de baixar somente por Wi-Fi;
- armazenamento compatível do Android 7 ao Android atual;
- yt-dlp-ejs incorporado e fragmentos adaptados ao consumo de bateria.

Para gerar o APK ARM64 instalável:

```powershell
npm run android:build
```

O resultado fica em:

`src-tauri\target\android\YT-DLP-Deck-Android-v1.1.3-arm64.apk`

Para gerar um Android App Bundle:

```powershell
npm run android:bundle
```

O AAB fica em:

`src-tauri\target\android\YT-DLP-Deck-Android-v1.1.3-arm64.aab`

Requisitos de compilação: JDK 17, Android SDK 36, Build Tools, NDK
27.3.13750724 e os targets Rust do Android. O script também contorna o bloqueio
de links simbólicos do Windows e assina o APK com a chave local de
desenvolvimento. Para a Google Play, configure uma chave de produção.

Nunca salve a chave ou as senhas no repositório. O script usa estas variáveis
quando todas estiverem definidas:

```powershell
$env:YTDLP_ANDROID_KEYSTORE = "C:\caminho\chave-upload.jks"
$env:YTDLP_ANDROID_KEY_ALIAS = "upload"
$env:YTDLP_ANDROID_STORE_PASSWORD = "senha-do-keystore"
$env:YTDLP_ANDROID_KEY_PASSWORD = "senha-da-chave"
npm run android:bundle
```

O código nativo desktop está em `src-tauri\src\lib.rs`. O motor Android está em
`src-tauri\plugins\mobile-downloader`, com Kotlin, Python/yt-dlp e FFmpegKit. A
interface React/TypeScript está em `src\App.tsx`, com o design system em
`src\styles.css`.

O background gráfico usa WebGPU/WGSL, com fallback WebGL2/GLSL em
`src\visuals\gpuBackdrop.ts`. Os parâmetros da animação são calculados pelo módulo
WebAssembly escrito em AssemblyScript em `assembly\visual-core.ts`.

## Cursores

O aplicativo inclui o conjunto escuro “Windows 11 Cursors Concept”, criado por
[Jepri Creations](https://www.deviantart.com/jepricreations). Os arquivos e a
licença original estão em `src\assets\cursors`.

## Fontes das ferramentas

- yt-dlp: `https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe`
- FFmpeg para Windows: `https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip`
