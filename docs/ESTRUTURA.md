# Organização das pastas

O código continua na raiz do projeto, para manter os comandos de desenvolvimento
e os caminhos de build existentes.

| Pasta ou arquivo | Conteúdo |
| --- | --- |
| `src/` | Interface React e estilos |
| `src-tauri/` | Código Rust, Android e configurações do aplicativo |
| `public/` | Recursos visuais públicos, incluindo `app-icon.svg` |
| `visual-core/` | Núcleo matemático em Rust compilado para WebAssembly |
| `scripts/` | Comandos auxiliares de build |
| `tests/` | Testes do frontend |
| `docs/` | Documentação e mapa do código |
| `arquivos-locais/anotacoes/` | Anotações pessoais |
| `arquivos-locais/diagnosticos/` | Arquivos de verificações manuais anteriores |
| `arquivos-locais/instaladores/` | Atalhos para os pacotes Windows e Android existentes |
| `Abrir YT-DLP Deck.lnk` | Atalho para o executável Windows existente |
| `cookie.txt` | Cookies pessoais; caminho preservado para configurações existentes |

`arquivos-locais/` e o atalho do aplicativo são ignorados pelo Git. Não coloque
cookies, chaves de assinatura ou outros dados pessoais nas pastas de código.

O executável Windows e suas ferramentas ficam juntos em
`src-tauri/target/release/`: `yt-dlp-deck.exe`, `yt-dlp.exe` e `ffmpeg.exe`.
Os instaladores continuam nos destinos originais dos builds. Os atalhos apenas
facilitam o acesso, sem duplicar os pacotes nem alterar esses destinos.

As pastas `.cargo`, `.git`, `.codex-remote-attachments` e
`.sandbox-readonly-blocked` são auxiliares de configuração, versionamento e
ambiente. Elas permanecem no lugar. `src-tauri/target/` e `src-tauri/gen/`
também contêm ferramentas e arquivos gerados necessários ao desenvolvimento.

Para desenvolver, execute os comandos npm na raiz do projeto, como antes.
