# Auditoria técnica da Ultimus (Otimiza)

Relatório pedido em 22/09/2026. **Nenhum código foi alterado por causa deste
documento** — ele é a leitura do projeto inteiro contra o pedido, para você
aprovar o que entra antes de qualquer implementação.

Uma observação que muda como ler o resto: **boa parte do que o pedido descreve
já foi construída na 2.9**, que nasceu de uma auditoria anterior
(`AUDITORIA-2.9.md`). Este relatório não repete aquilo como se fosse novidade.
Ele separa três coisas: **o que já existe e está provado**, **o que existe e
não foi provado**, e **o que não existe**.

---

## 1. O que existe hoje, em números

| | |
|---|---|
| Rust | 139 arquivos, ~86.300 linhas |
| Interface do app | 8 arquivos TypeScript, ~15.300 linhas, mais o `index.html` |
| Site | Next.js exportado estático (61 arquivos), publicado no GitHub Pages |
| Testes | 1.395 passando; 52 ignorados (exigem hardware ou jogo aberto) |
| Catálogo | 39 ajustes escritos, **23 ativos** e 16 retirados (mantidos só para desfazer) |
| Registro central | 44 alterações declaradas, com risco, reinício e desfazer |

**Arquitetura.** Tauri: Rust faz tudo que toca o Windows; a tela é HTML/TS e
não executa PowerShell. Cada escrita no sistema grava o valor anterior num
histórico em disco (`changelog.rs`), e existe um diário de transação
(`transacao.rs`) que termina o que ficou pela metade se o programa morrer no
meio. O site é estático e **não tem servidor, banco nem contas**: a licença é
uma assinatura Ed25519 conferida no navegador.

---

## 2. O pedido, item a item, contra a realidade

### Já existe, e foi provado nesta máquina

| Pedido | Onde está | Prova |
|---|---|---|
| Frametime real por ETW, não FPS por processo | `frames.rs` | Mede pelo canal de eventos de apresentação do Windows; nenhuma injeção. Roblox medido a 59,5 FPS com a máquina a 35% |
| 1% low, 0,1% low, P95, P99, engasgos | `core/fluidez.rs` | 0,1% só com 10.000 quadros — abaixo disso ele diz que não sabe |
| Bottleneck analyzer com categorias | `modules/gargalo.rs` + Mapa de 40 s | Classificou "fora do hardware" no Roblox preso em 60 FPS |
| Antes/depois com reversão automática | `portao.rs` + `repeticoes.rs` | Regra: intervalos de 95% que não se tocam **e** piora ≥ 5% |
| Ruído estatístico | `repeticoes.rs` | "+0,5%" não é ganho: abaixo de 3 repetições a resposta é "não sei" |
| Restore engine (detect/apply/verify/revert) | `catalog.rs` + `windows/mod.rs` | Toda escrita relida do sistema; erro desfaz o que já fez |
| Transação e recuperação de queda | `transacao.rs` | Retoma ou desfaz o que ficou pela metade na abertura seguinte |
| Compatibilidade com anticheat | `anticheat.rs` | Medir quadros é liberado (não encosta no processo); mexer no processo do jogo é recusado com o anticheat ativo |
| Topologia de CPU e afinidade inteligente | `nucleos.rs`, `topologia.rs`, `cpuset.rs` | Auto CPU Set testa P-cores × todos os núcleos, alternando, e só fica se medir ganho |
| Modo de sessão (aplica ao abrir, restaura ao fechar) | `governador.rs` | EcoQoS e prioridade baixa em quem disputa CPU; nada é fechado nem congelado |
| Energia por máquina, não receita fixa | `motorenergia.rs` | Mede candidatos e escolhe; notebook na bateria fica no padrão |
| Perfil NVIDIA por jogo, com antes → depois | `nvdriver.rs` | Escrita e desfazer provados na GTX 1650 em 22/09 |
| Diagnóstico térmico | `thermal.rs` + `sensoresgpu.rs` | CPU por evento do Windows; GPU lida do driver (35 °C, 15 W de 75 W) |
| DPC por núcleo | `dpc.rs` | Núcleo 0 com ~2,5% nesta máquina, veredito normal |
| Registro central do que o produto altera | `registro.rs` | 44 alterações, com trava que reprova quem escrever sem estar na lista |
| Relatório de diagnóstico e código de suporte | `report.rs`, `suporte.rs`, `maquina.rs` | PDF + planilha de alterações + `OTZ-XXXX-XXXX-XXXX` |
| Modo Simples/Expert | `index.html` + `main.ts` | Conferido no navegador: painéis Expert escondidos no Simples |
| Não desligar segurança | `catalog.rs`, `naofazemos.rs` | Defender, Firewall e UAC não são tocados; VBS é Expert, com o preço escrito |

### Existe, mas nunca foi visto funcionando

| O quê | Por que não foi provado |
|---|---|
| Auto CPU Set em processador híbrido | Esta máquina tem 8 núcleos iguais; o botão fica desligado de propósito |
| Portão desfazendo sozinho | Precisa de 3 partidas medidas antes e 3 depois |
| Painel de placa dupla | Exige notebook com iGPU + dedicada |

### Não existe — são estas as lacunas de verdade

| Lacuna | Por que importa | Dificuldade |
|---|---|---|
| **Sensores da GPU durante a partida** | Temperatura, clock e power limit só são lidos sob demanda, por `nvidia-smi` (processo externo, ~100 ms). Durante o jogo o produto não sabe se a placa está limitada por temperatura — que é a causa que ele mais deveria pegar | Média (NVML em processo) |
| **DPC por driver** | Hoje diz qual núcleo, não qual driver. Sem isso o cliente sabe que há um problema e não sabe o que fazer | Alta (ETW de kernel) |
| **Eventos do Windows: WHEA e reset de driver de vídeo** | "Sua placa resetou o driver 3 vezes ontem" explica travamento que nenhum tweak resolve. O Windows já registra; ninguém lê | Baixa |
| **VRR/G-SYNC ligado** | O produto evita mexer em V-Sync porque não sabe se há VRR. Saber destrava um ajuste real | Média |
| **Assinatura do instalador e checksum publicado** | Todo cliente vê "editor desconhecido". Não há hash publicado para conferir o download | Baixa (checksum) / cara (certificado) |
| **Feature flags / canário / Labs** | Hoje toda mudança vai para 100% dos clientes de uma vez | Média |
| **Base de resultados e reputação por otimização** | É o que responderia "isto funciona em que máquinas?" com dados. Exige servidor e consentimento explícito | Alta |
| **Observabilidade e página de estado** | Não há como saber que algo quebrou antes do cliente avisar | Média |
| **Laboratório de hardware** | Um PC só. Intel híbrido, AMD X3D e placa AMD nunca foram testados | Externo |

---

## 3. Placebo: o que já saiu, e o que sobrou

A 2.9 removeu **vinte** ajustes. Entre eles, os que o pedido cita nominalmente:
`SystemResponsiveness`, `NetworkThrottlingIndex`, `Win32PrioritySeparation`,
MMCSS de jogos, desligar SysMain, desligar compressão de memória, prioridade
alta fixa no jogo, e o plano "máximo" sugerido às cegas. Também saíram as duas
limpezas irreversíveis do catálogo — **o catálogo inteiro hoje tem desfazer**.

Da lista de suspeitos do pedido, **o produto nunca teve**: limpeza de RAM,
apagar Prefetch, tweaks de timer/HPET/BCD, TCP/MTU universal, DNS vendido como
FPS, desligar pagefile, desligar Defender ou Windows Update, prioridade
Realtime, limpeza de standby list. Cada um está escrito em `naofazemos.rs`, com
o motivo, e aparece na tela.

**O que ainda merece julgamento:**

1. **Cache de shader.** O pedido diz para não vender apagar cache como
   otimização — e está certo: apagar causa recompilação e engasgo. Hoje o
   produto só oferece quando o cache é **mais antigo que o driver**, o que é o
   caso legítimo. Proponho deixar explícito na tela que apagar custa a primeira
   partida.
2. **MSI da placa de vídeo.** É o único MSI que o produto escreve, está no modo
   Expert e só age se estiver desligado. Mantém-se, com medição obrigatória.
3. **Efeitos visuais / transparência.** Não mudam FPS em tela cheia. Já são
   condicionais (só em PC fraco) e ficam fora do lote. Manter como conforto,
   nunca vendido como desempenho.

---

## 4. Riscos e dívida técnica

| Risco | Onde | Gravidade |
|---|---|---|
| Módulos que medem o mesmo por caminhos diferentes: `core::telemetria` × `monitor.rs`, `deriva` × `historico`, `core::travadas` × correlação de disco | app | Média — é dívida, não defeito |
| `seus_jogos` varre a biblioteca e chama PowerShell; custo de abertura não medido | `commands.rs` | Média |
| Auto CPU Set nunca rodou em híbrido: a regra existe, o caminho real não | `cpuset.rs` | Alta enquanto não houver hardware |
| Uma máquina só de teste | projeto | Alta |
| Instalador sem assinatura | entrega | Alta para conversão de venda |
| O serviço que cobra e emite chave (bot) **não está neste repositório** | fora | Não auditável aqui |

---

## 5. Segurança: o escopo real

O pedido pede auditoria de autenticação, RBAC, banco, uploads, painel
administrativo, 2FA. **Nada disso existe neste projeto** — e é importante dizer
em vez de fingir auditoria:

- **Não há servidor, banco de dados nem contas.** O "painel" do site roda no
  navegador; entrar é colar uma chave assinada, conferida localmente. Não há
  senha para vazar, sessão para roubar nem SQL para injetar.
- **O que existe de superfície real é:** o site estático, o instalador no
  GitHub Releases, o `convite.json` lido da branch `main`, e o serviço do bot
  (fora deste repositório) que cobra e assina licenças.

O que foi feito e conferido nesta rodada:

- **CSP** em todas as páginas, com os scripts do Next por hash SHA-256 —
  `script-src` sem `'unsafe-inline'`. Script injetado é bloqueado (provado no
  navegador). `frame-ancestors` não funciona por `<meta>`: exige domínio próprio.
- **`Referrer-Policy`** aplicada; links externos com `noopener noreferrer`;
  nenhuma dependência com vulnerabilidade conhecida; nenhum segredo no
  repositório (varredura por chave privada, token e API key: zero).
- **Atualizador**: só **avisa**; não baixa nem instala nada. Não há canal de
  atualização para comprometer — o que elimina a classe inteira de ataque que o
  pedido teme.
- **Esteira**: roda tipos, lint e os 1.395 testes antes de publicar; permissões
  mínimas declaradas por workflow.

**O que falta em segurança, honestamente:**

1. Instalador sem assinatura digital (P1 de negócio, não de código).
2. Checksum SHA-256 publicado junto do release (P1, barato).
3. `frame-ancestors`/HSTS exigem domínio próprio (P2).
4. O bot: rate limit, validação de entrada, 2FA de administrador e log de
   auditoria **não foram auditados** porque o código não está aqui. Se você
   quiser, é a próxima auditoria — e é onde mora o dinheiro.

---

## 6. Tecnologias propostas

| Proposta | Ganho | Dificuldade | Prioridade |
|---|---|---|---|
| **NVML em processo** (`nvml.dll`) para amostrar temperatura, clock e power limit **durante** a medição | Fecha "limitado por temperatura" com dado do jogo, não de depois. Substitui o `nvidia-smi` (que abre processo) | Média | P1 |
| **Eventos WHEA + reset de driver (4101)** | Explica travamento e tela preta que nenhum ajuste resolve. O Windows já registra | Baixa | P1 |
| **ETW de kernel para DPC por driver** | Transforma "o núcleo 0 está ocupado" em "o driver X está segurando o núcleo 0" | Alta | P2 |
| **PresentMon** | Traria latência de apresentação pronta. **Não recomendo**: distribuir binário de terceiro contraria a regra do produto, e o ETW próprio já entrega frametime | — | Não fazer |
| **Detecção de VRR** | Destrava decisão sobre V-Sync por jogo | Média | P2 |
| **Feature flags locais + seção Labs** | Experimental deixa de se misturar com recomendado; dá para desligar um ajuste sem publicar versão | Média | P2 |
| **Base de resultados com consentimento** | Responde "funciona em que máquina" com dados reais | Alta (exige servidor) | P3 |

---

## 7. Roadmap proposto

### P0 — antes de qualquer coisa
1. **Convite permanente do Discord** — feito hoje (`discord.gg/ultimus`); falta chegar à `main` para consertar quem já instalou.
2. **Publicar a 2.9** (merge, tag, notas, site) — a versão está pronta e parada.

### P1 — próxima rodada
3. Sensores da GPU durante a partida (NVML), com temperatura no relatório de prova.
4. Leitor de eventos WHEA e reset de driver de vídeo, na aba Diagnóstico.
5. Checksum SHA-256 publicado no release e mostrado na página de download.
6. Medir o custo de abertura do app depois da 2.9 (o orçamento de 1,2 s da 1.7).

### P2 — depois
7. DPC por driver (ETW de kernel).
8. Detecção de VRR e decisão de V-Sync por jogo.
9. Feature flags + Labs.
10. Unificar o que mede duas vezes (`telemetria` × `monitor`, `deriva` × `historico`).
11. Domínio próprio: HSTS, `frame-ancestors`, e o checkout Pix ligado.

### P3 — quando houver base
12. Base de resultados com consentimento e reputação por otimização.
13. Laboratório de hardware (híbrido Intel, AMD X3D, placa AMD).
14. Observabilidade e página de estado do serviço.

---

## 8. O que eu recomendo NÃO fazer

- **Não reescrever o motor de otimização.** O registro central já dá a visão
  única que faltava; trocar o executor de vinte módulos agora é risco sem
  ganho para o cliente.
- **Não empacotar binário de terceiro** (PresentMon, instaladores de driver,
  "debloat" de terceiros). Contraria a regra que o produto vende.
- **Não criar contas e senhas no site** só para ter "autenticação". Chave
  assinada conferida no navegador é mais segura do que um banco de senhas que
  eu teria de proteger.
- **Não prometer número que não foi medido.** Toda porcentagem do produto
  precisa continuar vindo de medição, com "não sei" quando for o caso.
