# YT-DLP Deck

Aplicativo para Windows e Android construído com Tauri 2, Rust, React,
TypeScript, Kotlin e Python.

FEITO INTEIRAMENTE POR IA, ENTAO TERÁ BUGS!!!!

## Downloads oficiais

- [Baixar para Windows (`yt-dlp-deck.exe`)](https://github.com/LucasPreto0000/yt-dlp-deck/releases/download/v1.1.16/yt-dlp-deck.exe)
- [Baixar para Android ARM64 (`.apk`)](https://github.com/LucasPreto0000/yt-dlp-deck/releases/download/v1.1.16/YT-DLP-Deck-Android-v1.1.16-arm64.apk)
- [Ver a release completa e as notas da versão](https://github.com/LucasPreto0000/yt-dlp-deck/releases/tag/v1.1.16)

O APK é compatível com Android 13 ou superior em aparelhos ARM64. No Android,
pode ser necessário permitir a instalação de aplicativos desconhecidos.

## Usar no Windows

Ao abrir, o aplicativo verifica se estes dois arquivos estão na mesma pasta do
`yt-dlp-deck.exe`:

1. `yt-dlp.exe`;
2. `ffmpeg.exe`.

Se algum estiver ausente, a interface mostra um aviso e permite checar novamente
depois que os arquivos forem colocados na pasta.

Quando os dois selos aparecem como prontos, clique em `yt-dlp` ou `FFmpeg` para
procurar e instalar silenciosamente a atualização daquela ferramenta. O app
valida o pacote do FFmpeg e restaura os executáveis anteriores se a troca falhar.
As consultas reutilizam conexões e versões em cache, então verificações repetidas
respondem quase instantaneamente sem abrir os executáveis novamente.

Os arquivos baixados ficam em:

`Downloads\YT-DLP Deck\<Plataforma>`

## Interface e experiência

- tipografia local dedicada para títulos, textos e telemetria;
- central de download com thumbnail, estado visual e progresso animado;
- download da imagem de capa de vídeos do TikTok e Instagram;
- legenda opcional junto com vídeo ou áudio, priorizando a do autor e usando automática/traduzida como fallback;
- console ao vivo com destaque de mensagens, controle de rolagem e cópia do log;
- histórico agrupado por data com ações contextuais e estado vazio;
- notificações animadas, verificação visual e atualização silenciosa de yt-dlp e FFmpeg;
- interações consistentes com os cursores personalizados do aplicativo.

## Desenvolvimento

Consulte [a organização das pastas](docs/ESTRUTURA.md) para localizar o código,
os arquivos pessoais e os atalhos do aplicativo e dos instaladores.

```powershell
npm install
npm run desktop:dev
```

Para gerar apenas o `.exe`:

```powershell
npm run desktop:exe
```

Para gerar o `.exe` e o instalador:

```powershell
npm run desktop:build
```

Builds repetidos reutilizam o frontend e os artefatos Rust quando os arquivos de
entrada não mudaram. Para forçar o frontend completo, use `npm run build:full`.
Mantenha `src-tauri/target` para aproveitar a compilação incremental; após uma
limpeza dessa pasta, a primeira compilação volta a ser mais demorada.

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
- armazenamento moderno via MediaStore para Android 13 ou superior;
- verificação preventiva de espaço livre antes do download;
- yt-dlp-ejs e QuickJS incorporados para extração completa do YouTube;
- navegação imersiva opcional, com controle nas configurações;
- busca pré-aquecida e cache temporário para respostas mais rápidas;
- exclusão dos cookies importados diretamente pelo aplicativo.
- capas de TikTok/Instagram e legendas sem depender do FFmpeg.

Para gerar o APK ARM64 instalável:

```powershell
npm run android:build
```

O resultado fica em:

`src-tauri\target\android\YT-DLP-Deck-Android-v1.1.22-arm64.apk`

Para gerar um Android App Bundle:

```powershell
npm run android:bundle
```

O AAB fica em:

`src-tauri\target\android\YT-DLP-Deck-Android-v1.1.22-arm64.aab`

Requisitos de compilação: JDK 17, Android SDK 36, Build Tools, NDK
27.3.13750724 e os targets Rust do Android. O script também contorna o bloqueio
de links simbólicos do Windows, compila o QuickJS para ARM64 e assina o APK com
uma chave local persistente de release, criada fora do repositório. Para a
Google Play, configure uma chave de produção.

O script Android preserva os intermediários do Gradle e os pacotes Python para
acelerar builds seguintes. Se o cache Android ficar corrompido ou inacessível,
execute `npm run android:build -- -RepairCache` para reconstruí-lo.

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
interface React/TypeScript é coordenada por `src\App.tsx`, com componentes em
`src\components`, recursos em `src\features` e opções em `src\config`.
O design system está em `src\styles`, importado por `src\styles.css`.
As regras puras de download ficam em `src-tauri\src\download`.
Consulte o [mapa da arquitetura](docs/ARCHITECTURE.md) para localizar cada recurso
e os comandos de validação rápida.

O background gráfico usa WebGPU/WGSL, com fallback WebGL2/GLSL em
`src\visuals\gpuBackdrop.ts`. Os parâmetros da animação são calculados pelo módulo
WebAssembly escrito em Rust em `visual-core\lib.rs`. Para compilar em uma
máquina nova, instale o alvo com `rustup target add wasm32-unknown-unknown`.

## Importação Spotify e qualidade de áudio (Windows)

Na plataforma Spotify, cole um link `https://open.spotify.com/track/...` e use
“Importar faixa e buscar no YouTube”. O Spotify fornece apenas o título via
[oEmbed](https://developer.spotify.com/documentation/embeds/reference/oembed).
Escolha e confira a prévia de um resultado no YouTube antes de continuar; ajuste
o nome/artista na pesquisa se necessário. Não é download do áudio do Spotify,
não requer conta Premium e não contorna DRM. Álbuns e playlists não são importados.
Baixe apenas conteúdo para o qual você tenha autorização.

Os perfis Windows de áudio selecionam `bestaudio/best`. MP3 usa 320 kbps;
FLAC evita novas perdas na conversão, sem upsampling ou alteração forçada de
canais. Converter uma fonte com perdas em FLAC não recupera informação perdida.

## Cursores incluídos

O aplicativo inclui o conjunto escuro “Windows 11 Cursors Concept”, criado por
[Jepri Creations](https://www.deviantart.com/jepricreations). Os arquivos e a
licença original estão em `src\assets\cursors`.

## Fontes das ferramentas

- yt-dlp: `https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe`
- FFmpeg para Windows: `https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.7z`
- FFmpeg Git para Windows: `https://www.gyan.dev/ffmpeg/builds/ffmpeg-git-essentials.7z`
