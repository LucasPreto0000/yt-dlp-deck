# YT-DLP Deck

Aplicativo desktop para Windows construído com Tauri 2, Rust, HTML, CSS e JavaScript.

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

O código nativo está em `src-tauri\src\lib.rs`. A interface React/TypeScript está em
`src\App.tsx`, com o design system em `src\styles.css`.

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
