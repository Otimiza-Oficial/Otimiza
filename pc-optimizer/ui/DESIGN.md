# Direção visual da interface (marca v2)

Lido antes de qualquer tela nova. A fonte da marca é a entrega aprovada pelo dono (`otimiza-marca-v2`: `MARCA.md`,
`tokens-marca.json`, `svg/`). Código é a fonte da verdade da interface: `src/tokens.css`, `src/componentes/base/`,
`src/componentes/Oti.svelte`, `src/componentes/marca/`, `src/App.svelte` e `src/componentes/Inicio.svelte`.

A v1 (bolha de vidro azul, `#6E9BFF`, poses do Oti) e o painel de vidro estilo visionOS saíram em 21/11/2026 (1.0.12).

## O que a interface é
Séria, premium, monocromática. Fundo preto (papel no claro), superfícies sólidas em grafite com borda fina e o fio de
luz no alto, tipografia Geist. Nada fantasioso nem cartunesco, nada com cara de template. A cor existe **só para
estado** (ok, atenção, perigo), sempre como ponto, ícone pequeno ou texto curto, nunca como fundo, borda lateral,
degradê ou enfeite.

## O Oti
- Disco chapado com dois olhos escuros em cápsula, inclinados −32°, no canto de cima à direita; o olho de trás
  encurtado, cortado pela borda. No escuro, o original (disco branco); no claro, o negativo (disco carvão, olhos
  brancos). Até 32 px, os olhos da versão pequena (`simbolo-16`).
- **O único movimento é o olhar**: os olhos giram em volta do centro. Cada "humor" do código (`OLHAR` em `Oti.svelte`)
  é só uma direção. Sem boca, brilho, reflexo, sombra, cor, pose ou gesto. A única exceção são as épocas do ano (abaixo):
  aí ele usa o enfeite da época, posto de lado para não cobrir os olhos.
- Aparece como elemento de marca: no Início (a cena com o brilho), na primeira execução, nos estados vazios de
  primeira vez (`Estado` com `oti`), no balão da conversa e na busca. Nunca em erro.
- **A cena**: o Oti olha para onde as partículas de luz se juntam (`marca/Brilho.svelte`, semente fixa, estático). O
  fundo "Limpo" (`marca/CeuDePoeira.svelte`) espalha poeira de luz mais densa no alto à direita, para onde ele olha.
- O logotipo é o SVG da entrega (`marca/Logo.svelte`): nunca se digita "Otimiza" com a fonte no lugar dele.
- Os ícones da bandeja saem de `brand/gerar_bandeja.py`, com a mesma geometria e os mesmos olhares.

## Épocas do ano (decisão do dono, 1.0.13)
Natal, Ano Novo, Carnaval, Páscoa, Festa Junina, Dia das Crianças e Halloween vêm **ligadas por padrão** (desligam em
Ajustes › Temas de época). Em cada uma:
- **A cena** (`marca/CenaDaEpoca.svelte`, no fundo Limpo): paisagem noturna em camadas (céu e luz da época, estrelas e
  brilhos da marca, relevo em dois planos e os objetos da época) e o Oti com o enfeite, olhando o brilho, no canto de
  baixo à esquerda. Composta para onde ela aparece: a faixa de baixo, a coluna abaixo do trilho e um céu escuro sob a
  barra. Tem versão de dia no tema claro. É um SVG estático com semente fixa: **nada se anima dentro dela** (repintaria
  a paisagem a cada quadro); só a entrada ao abrir.
- **O movimento** fica com o `EfeitoDeEpoca` (varal e partículas, camadas próprias por transform e opacidade). Para com
  "reduzir movimento" e com o jogo aberto.
- **A interface acompanha com detalhe**: o filete colorido no alto dos cartões (`--epoca-cor`), a geada no Natal, o traço
  da época no trilho, e no Início a saudação da época com o ornamento dela (`marca/OrnamentoDaEpoca.svelte`) na cor de
  texto da época (`--epoca-texto`, com AA medido nos dois temas) e uma luz baixa atrás do Oti. Texto e cor de ação
  continuam os da marca.
- **Prévia**: Ajustes › Temas de época › "Ver uma época agora" mostra qualquer época na hora, até fechar o app.
- Prints: `node scripts/prints-epocas.mjs <pasta> [largura] [altura]`.

## Tokens (`src/tokens.css`)
- **Cor**: primitivas da marca (preto, carvão, grafite, cinzas, papel, branco) e semânticas que os componentes usam:
  `--fundo`, `--superficie(-2/-3/-menu)`, `--tinta(-2/-3)`, `--linha(-forte)`, `--acao`, `--contra`, `--foco`,
  `--ok/--atencao/--perigo` e as versões `-texto`. Componente nunca usa `#hex` nem as primitivas.
- **Contraste medido** (`testes/contraste.test.ts`): todo texto ≥ 4,5:1 sobre fundo e superfícies nos dois temas;
  foco e ponto de estado ≥ 3:1.
- **Tipografia**: Geist só. Títulos 700 com `--rastreio-titulo` (−0,04em); rótulos (`.rotulo`) 500 em caixa alta com
  `--rastreio-rotulo`; texto 400; números com `tabular-nums`. Escala `--t-apoio` (12) … `--t-heroi` (56). **Nada
  abaixo de 12 px** (`testes/tamanho-minimo.test.ts`).
- **Espaço** `--e-1`…`--e-8` (4, 8, 12, 16, 24, 32, 48, 64); **raio** `--raio-1`…`--raio-4` e `--raio-pilula`;
  **sombra** `--sombra-1/2/3` curtas + `--fio` (a profundidade vem da borda e do contraste, não de nuvem cinza).
- **Movimento** `--mov-rapido/medio/entrar/lento` e `--curva-padrao/saida/entrada`. Sem mola, sem quique
  (`testes/movimento.test.ts`).

## Componentes-base (`src/componentes/base/`)
Toda tela usa estes antes de inventar o seu:
- `Botao`: principal (cheio na cor do texto, um por bloco), contorno, link e perigo; `trabalhando` diz o que está
  acontecendo e trava o clique. O verbo diz o que acontece ("Usar 180 Hz", "Medir antes"), nunca "OK".
- `Interruptor`: liga uma coisa que existe; quando não dá, o motivo e o que fazer aparecem no texto (não só no `title`).
- `Campo`: rótulo sempre visível, ajuda embaixo, erro no lugar da ajuda com `aria-invalid`.
- `Estado`: lendo (esqueleto no formato do que vem), vazio, erro e sem administrador, **sempre com o que fazer**.
  "Não consegui ler" é erro, não vazio. O detalhe técnico fica recolhido ("Detalhe para o suporte").
- `Esqueleto`, `SeloDeEstado` (ponto + texto), `Dica` (tooltip com mouse e teclado), `Abas` (tablist acessível,
  "segmento" ou "barra"), `Cartao` e `Confirmar` (o `<dialog>` nativo; ação que apaga ou muda o PC pede confirmação
  dentro do app, nunca `confirm()`).
- Classes globais para o que é só estilo: `.btn`, `.segmento`, `.selo` (`.medido` cheio, `.estimado` tracejado),
  `.ponto[data-tom]`, `.rotulo`, `.apoio`, `.secao`, `.recado`, `.lista`, `.interruptor`, `.campo`.

## Comportamento
- Cada bloco mostra um dado lido deste PC ou liga algo que existe de verdade. Nada finge funcionar.
- Número com origem: selo `medido` ou `estimado`. Sem promessa de ganho antes de medir. No Frame Boost ligado, o
  multiplicador mostrado é o **real** (tela ÷ jogo), com o porquê e o próximo passo (`lib/frameboost.ts`).
- **Microtexto** (pt-BR, frases curtas): todo estado e todo erro diz o que aconteceu e o que fazer. Erros passam por
  `explicarErro` (`lib/erros.ts`); o caminho do suporte é "Ajustes › Versão e suporte". Termo técnico ganha meia
  frase de explicação.
- **Primeira execução** (`BoasVindas.svelte`): um fluxo só com progresso em seis etapas: Conhecer (a história do tempo
  de cada quadro, marcada como ilustração) → Licença → Medir antes → Primeiro ajuste → Provar → Aparência (Escuro, Claro
  ou Sem fundo), usando as peças de verdade (ativação, laboratório, Otimizar agora). Dá para pular e voltar.

## Acessibilidade
- Abas de verdade: `tablist`/`tab`/`aria-selected`, setas (o trilho com as de cima e de baixo), Home, End e só a
  escolhida no Tab (`lib/abas.ts`). Grupos de rádio (tema, nível, filtro) andam pelas setas.
- Foco: o anel global de `:focus-visible` na cor do texto; um componente só muda o recuo, nunca tira o anel.
- Alto Contraste do Windows (`forced-colors`): toda superfície ganha borda, a escolha vira a cor de destaque do
  sistema; "mais contraste" engrossa linhas e o texto de apoio.

## Movimento e jogo aberto
- Movimento sóbrio: a troca de área sobe 6 px e aparece; o marcador das abas desliza; o olhar do Oti gira devagar.
  Nada pula, cresce ou quica no hover.
- "Reduzir movimento" e **jogo aberto** (classe `jogo-aberto` na raiz) param tudo: CSS pela regra global, transições
  do Svelte por `lib/movimento.ts`. Com o jogo aberto também some o desfoque.
- **A UI escondida não consulta nada**: todo intervalo passa pelo relógio central (`lib/relogio.ts`), que com a
  janela escondida não chama quem pergunta ao Windows; o jogo aberto chega pelo evento `jogo:sessao` do Rust.

## Conferir
`npm run testes:ui`, `npx svelte-check --workspace ui --tsconfig ./tsconfig.json`, `npm run fumaca:nova`, e os prints:
`node scripts/prints-nova.mjs` (todas as áreas) e `node scripts/prints-telas.mjs` (as telas redesenhadas e cada passo
da primeira execução, nos dois temas). O Oti em cada olhar: `npm run nova:web` e `/?galeria`.
