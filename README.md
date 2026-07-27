# YT-DLP Deck

Aplicativo para Windows e Android construído com Tauri 2, Rust, React,
TypeScript, Kotlin e Python.

FEITO INTEIRAMENTE POR IA, ENTAO TERÁ BUGS!!!!

## Usar

Instale pelo arquivo:

`src-tauri\target\release\bundle\nsis\YT-DLP Deck_1.0.0_x64-setup.exe`

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

Para gerar o APK ARM64 instalável:

```powershell
npm run android:build
```

O resultado fica em:

`src-tauri\target\android\YT-DLP-Deck-Android-v1.0.0-arm64.apk`

Requisitos de compilação: JDK 17, Android SDK 36, Build Tools, NDK
27.3.13750724 e os targets Rust do Android. O script também contorna o bloqueio
de links simbólicos do Windows e assina o APK com a chave local de
desenvolvimento. Para a Google Play, configure uma chave de produção.

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
