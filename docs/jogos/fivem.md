# FiveM (GTA V) — ficha de configuração

| | |
|---|---|
| Arquivo | `%APPDATA%\CitizenFX\gta5_settings.xml` (FiveM) e `Documentos\Rockstar Games\GTA V\settings.xml` (GTA V) |
| Formato | XML do GTA V (`<Chave value="..." />`) |
| Perfil no código | `modules/windows/configjogo.rs` (não é o `configmotor.rs`): perfis `competitivo` e `equilibrado`, aplicados pelo Turbo → Perfil do jogo |
| Motor do cliente | `%LOCALAPPDATA%\FiveM\FiveM.app\CitizenFX.ini`, **só leitura** (`modules/windows/citizenfx.rs`) |
| Situação | escreve só o que já existe no arquivo, só entre as aspas da chave; recusa com o jogo aberto; guarda o arquivo inteiro para o desfazer |

## O que os perfis mudam (`Perfil::mudancas_para` em `configjogo.rs`)
| Chave | Para | Perfil | Por quê |
|---|---|---|---|
| `VSync`, `VSyncMode`, `FPSLimit`, `MaxFPS` | 0 | todos | sincronia e limite do jogo somam atraso; o teto, quando há, é o do Otimiza (`plataforma/teto_do_jogo.rs`) |
| `MSAA`, `ReflectionMSAA` | 0 | equilibrado e competitivo | MSAA em 4x custa de 30 a 50% dos quadros numa placa de entrada (medido na máquina do dono) |
| `Tessellation`, `SSAO`, `MotionBlurStrength`, `DoF`, `HdStreamingInFlight` | 0 / false | equilibrado e competitivo | caros e quase invisíveis em movimento; streaming em voo dá tranco ao carregar cenário |
| `Windowed` | 0 (tela cheia) | equilibrado e competitivo | o jogo fala direto com a placa; **nunca** com gerador por captura (o Frame Boost não vê o jogo em tela cheia exclusiva) |
| `ReflectionQuality`, `ShadowQuality`, `WaterQuality`, `ParticleQuality`, `PostFX`, `GrassQuality` | 0 | competitivo | o resto que custa quadro |
| `TextureQuality` | 0 | competitivo, só com menos de 6 GB de memória de vídeo | textura quase não custa quadro até transbordar a memória, e aí é engasgo severo |

`RefreshRate` = 60 num monitor mais rápido é corrigido até no perfil que não mexe no visual.

## Anticheat
O FiveM não tem anticheat que proíba janela por cima: o Frame Boost e a mira funcionam. Servidores podem ter anticheat
próprio (do lado do servidor); o Otimiza não injeta nada nem mexe no processo do jogo.

## Cache
- Cache do FiveM (pastas de `FiveM.app\data`): limpeza por pasta no painel do FiveM (`FivemPainel.svelte`), com as
  pastas protegidas.
- Caches de shader do driver: pelo catálogo do jogo (`lib/catalogo.ts`).

## A confirmar
- O ganho medido de cada chave, antes × depois, em mais máquinas além da do dono.
