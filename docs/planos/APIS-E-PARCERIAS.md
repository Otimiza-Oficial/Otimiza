# APIs de terceiros e parcerias — pesquisa de 05/11/2026

> Checagem da Onda 0 da Nova Geração (capítulo 8 de `PLANO-1.0-NOVA-ERA.md`). Fontes primárias quando possível. O
> que não pôde ser confirmado está marcado **não verificado**. Não abriram: a wiki da Valve sobre GSI (403), os termos
> do SteamGridDB (vazio) e a documentação do IGDB (403).

## O que muda no plano

- **Pausar o Windows Update por API não existe na prática.**
  - `IAutomaticUpdates::Pause` não faz nada desde o Windows 10.
  - As políticas de pausa não valem no Windows Home.
- **Widget da Xbox Game Bar:** precisa de um projeto UWP separado, publicado na Microsoft Store (ou MSIX junto do app) e de uma conta no Partner Center. Sai da 1.0 e vai para depois.
- **Não há API no Windows para o 3D V-Cache da AMD** (só no Linux).
- **Não há API para o APO da Intel, nem para as "configurações ideais" do NVIDIA App.**
- **FrameView SDK:** não é público. Fica o PresentMon, que é a base dele.
- **Optical Flow da NVIDIA:** não roda na TU117, a GTX 1650 do dono.
- **GameInput:** `sampleRate` é a taxa que o mouse *declara*, não a medida. A taxa real vem dos carimbos de tempo (Raw Input).
- **Ping passivo TCP (`GetPerTcpConnectionEStats`):** exige administrador e só vale para TCP. A maioria dos jogos usa UDP, então o valor é limitado.
- **Uso comercial com condições:**
  - M-Lab exige contribuição;
  - RIPEstat exige permissão escrita;
  - IGDB pede parceria;
  - a Steam Web API fala em "personal use": a chave fica num servidor nosso, nunca no app.
- **WinRing0:** o Defender coloca em quarentena desde março de 2025 (CVE-2020-14979). Proibido no Otimiza.

## NVIDIA

| API | O que dá | Licença | Acesso | Exige instalado | Veredito | Fontes |
|---|---|---|---|---|---|---|
| NVAPI | Clocks, temperatura, perfis do driver (limitador, baixa latência) | **MIT** | GitHub público | Driver NVIDIA | **Pode entrar já** (já usamos) | github.com/NVIDIA/nvapi |
| NVML | Uso, energia, temperatura, clocks, memória | Header permissivo. Redistribuir a DLL: **não verificado**, não redistribuir | Público | DLL do driver, via `LoadLibrary` com caminho completo. GeForce com suporte "limitado" | **Pode entrar já** | docs.nvidia.com/deploy/nvml-api |
| Reflex / PC Latency Stats | Eventos ETW do jogo com Reflex; `MsPCLatency` (beta) no PresentMon 2.4+ | Para ler o ETW não precisa do SDK | ETW | Jogo com Reflex | **Pode entrar já**, pelo PresentMon | developer.nvidia.com/blog/understanding-and-measuring-pc-latency |
| Optical Flow SDK | Fluxo óptico em hardware e FRUC | DesignWorks SDK License: distribuir só as partes permitidas, em binário, dentro do app | developer.nvidia.com (login provável) | Turing+ (**menos a TU117**), driver ≥ 522.25 | **Pode entrar já**, cumprindo a licença, só em RTX | docs.nvidia.com/video-technologies/optical-flow-sdk/license |
| G-Assist plugins | Plugin no assistente local das RTX | Apache-2.0 | Público: `manifest.json` + executável, JSON-RPC. Hub via mod.io; regras de revisão **não verificadas** | NVIDIA App e RTX | **Pode entrar já** (nicho) | github.com/NVIDIA/G-Assist |
| FrameView SDK | — | — | Não é público | — | **Não serve**: usar o PresentMon | — |
| NVIDIA App "optimal settings" | — | — | Sem API: é curado pela NVIDIA | — | **Não serve** | forums.developer.nvidia.com |

## AMD

| API | O que dá | Licença | Acesso | Exige instalado | Veredito |
|---|---|---|---|---|---|
| ADLX | Métricas, Anti-Lag, Chill, Boost, RSR, AFMF, FreeSync, energia | PDF proprietário (**ler antes**; fonte secundária diz que distribuir o binário é permitido) | GitHub GPUOpen | Driver AMD (a DLL vem nele) | **Pode entrar já** (já usamos) |
| Ryzen Master Monitoring SDK | Métricas de CPU (só leitura, 1 chamada/s) | Proprietária e restritiva | Download público, v3.0.1.4971 (12/02/2026) | Driver do Ryzen Master e admin (**não verificado**) | **Precisa de acordo**; sem ele, só se o cliente já tiver o Ryzen Master |
| 3D V-Cache Optimizer | Modo cache/frequência | — | Só Linux (`amd_x3d_mode`) | — | **Não serve no Windows** |

## Intel

| API | O que dá | Licença | Acesso | Exige instalado | Veredito |
|---|---|---|---|---|---|
| IGCL | Controle e telemetria da Arc/iGPU | Intel SLA: só em plataformas Intel; **não empacotar a lib** | GitHub | Driver Intel | **Pode entrar já** (já usamos) |
| **PresentMon 2.6** (console, lib, serviço) | Frametime, clique → imagem, Animation Error, frame gen, `MsPCLatency`, telemetria de GPU | **MIT** (dá para redistribuir, inclusive o serviço) | Público. v2.6.0 (21/set) | ETW (admin ou "Performance Log Users"); o serviço instala por MSI. A compatibilidade binária de DLL própria não é garantida | **Pode entrar já**: é o núcleo |
| Intel APO | — | — | Sem API pública | — | **Não serve** |

## Microsoft

| API | O que dá | Acesso / exigências | Veredito |
|---|---|---|---|
| Xbox Game Bar Widget SDK | Overlay oficial na Game Bar | Widget **UWP** publicado na Store, ou MSIX junto do app Win32. Conta no Partner Center. App Tauri não é o widget | **Precisa de conta**: passa para depois da 1.0 |
| GameInput | Entrada de baixa latência | NuGet `Microsoft.GameInput` 4.0.306 (28/10/2026); redistribuível com termos; instala `GameInputRedist.msi` (admin) | **Pode entrar já** (mas a taxa do mouse sai do Raw Input) |
| QoS (NetQos) | Limite de banda por programa, DSCP | Admin. `ActiveStore` some ao reiniciar (fácil de desfazer). Doc oficial só para Server; **Windows Home não verificado** | **Pode entrar já**, testar no Home |
| WLAN API | `WlanScan`, varredura em segundo plano | A varredura só desliga conectado e volta ao desconectar. No 24H2, listar redes pede "localização para apps desktop" | **Pode entrar já** |
| `GetPerTcpConnectionEStats` | RTT e retransmissões por conexão | Admin; só TCP | **Pode entrar já** (valor limitado) |
| Windows Update Agent (pausar) | — | `Pause` não funciona desde o Windows 10; políticas sem Home | **Não serve** |

## Valve

| API | Licença / acesso | Veredito |
|---|---|---|
| **CS2 Game State Integration** | Oficial: arquivo `gamestate_integration_*.cfg` em `cfg/`, o jogo faz POST local. Declaração sobre VAC **não verificada** (wiki deu 403), mas não há injeção | **Pode entrar já** |
| Steam Web API | Termos de 2010: dados para "personal use" do usuário, 100 mil chamadas/dia, chave de responsabilidade nossa | **Pode entrar já**, com chave num servidor nosso, nunca no app |

## Rede

| Serviço | Licença / acesso | Veredito |
|---|---|---|
| M-Lab ndt7 | Regra de uso (AUP) de 15/07/2025: uso comercial deve contribuir; no máximo 4 testes automáticos/dia; o IP do cliente é publicado; exige consentimento | **Precisa de acordo** |
| Cloudflare speedtest | Biblioteca MIT; uso dos endpoints por terceiros **não verificado**; os termos proíbem sobrecarga | **Com risco**: pedir anuência ou endpoint próprio (Workers) |
| **IPinfo Lite** (IP → ASN/operadora) | CC BY-SA 4.0, uso comercial com atribuição ("IP address data is powered by IPinfo"); grátis e ilimitado | **Pode entrar já** (melhor opção) |
| Team Cymru | Termos comerciais **não verificados** | Evitar sem contato |
| RIPEstat | Uso comercial exige permissão escrita | **Precisa de acordo** |

## Capas de jogos

| API | Veredito |
|---|---|
| SteamGridDB | Termos **não verificados**; há relato de pedido para um launcher não integrar. **Contatar antes** |
| IGDB (Twitch) | Uso comercial pede parceria. **Precisa de acordo** |

Até lá, seguem as capas locais da Steam (`plataforma/capas.rs`).

## Periféricos RGB

| SDK | Exige | Veredito |
|---|---|---|
| Razer Chroma (agora WYVRN) | Synapse | **Pode entrar já** (ler o EULA) |
| Logitech LED/LIGHTSYNC | G HUB | **Pode entrar já** (ler a licença do zip) |
| Corsair iCUE v4 | iCUE | **Pode entrar já** (confirmar a licença) |
| SteelSeries GameSense | GG/Engine | **Pode entrar já** |
| OpenRGB (TCP 6742) | Servidor OpenRGB rodando | **Com cuidado**: GPL-2.0. Escrever cliente próprio a partir da documentação do protocolo, sem linkar biblioteca GPL (o crate `openrgb` é GPL). Opinião, não parecer jurídico |

## Discord

O Game SDK foi arquivado; o substituto é o Social SDK (termos de 17/03/2025). O Rich Presence por RPC/IPC local continua documentado e só precisa do Application ID. **Pode entrar já.** Se a política permite Rich Presence para app que não é jogo: **não verificado**.

## Sensores: WinRing0

- CVE-2020-14979: dá SYSTEM a qualquer usuário.
- O Defender coloca o driver em quarentena desde março de 2025.
- LibreHardwareMonitor e Fan Control trocaram por PawnIO, que também é driver de kernel.
- **No Otimiza:** nenhuma biblioteca com WinRing0; sensores só por NVAPI, ADLX, IGCL, PresentMon e WMI/PDH.
- Fontes: Neowin; github.com/Rem0o/FanControl.Releases/discussions/3017.

## Prioridades (valor ÷ esforço)

1. **PresentMon 2.6:** clique → imagem, Animation Error, frame gen e `MsPCLatency` (Reflex, sem tocar no jogo).
2. **Telemetria de placa pelo driver** (NVAPI, NVML, ADLX, IGCL): nada para redistribuir, sem kernel.
3. **IPinfo Lite + teste de latência e jitter próprio:** mostra a operadora (Vivo, Claro, TIM) e o trecho com problema.
4. **Rede nativa do Windows:** QoS por programa durante o jogo (`ActiveStore`, reversível) e a varredura do Wi-Fi.
5. **CS2 Game State Integration**, com o Discord Rich Presence como bônus.

## Parcerias e contatos

| Item | Onde |
|---|---|
| M-Lab | measurementlab.net/contribute (privacy@measurementlab.net) |
| RIPEstat | stat@ripe.net |
| IGDB | api-docs.igdb.com (URL da parceria **não verificada**) |
| SteamGridDB | steamgriddb.com |
| Widget da Game Bar | partner.microsoft.com |
| NVIDIA | developer.nvidia.com/developer-program |
| AMD Ryzen Master | amd.com/en/developer/ryzen-master-monitoring-sdk.html |
| Corsair | página iCUE Dev, citada no README de github.com/CorsairOfficial/cue-sdk |
| Logitech | logitechg.com/en-us/innovation/developer-lab.html |
| SteelSeries | github.com/SteelSeries/gamesense-sdk |
| Cloudflare | contato direto ou endpoints próprios |
