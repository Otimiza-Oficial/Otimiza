# Direção visual da interface nova

Lido antes de qualquer tela nova. Código é a fonte da verdade: `src/App.svelte`, `src/componentes/Oti.svelte`,
`src/componentes/Icone.svelte`, `src/tokens.css` e `src/lib/conversa.ts`.

## Referências do dono (07/11/2026)
- **Mascote**: o disco branco liso com dois olhos em cápsula inclinados, cavados com chanfro claro, cabeça meio de
  lado. Sem boca, sem mãos: os olhos são a expressão inteira.
- **Interface**: painel de vidro fosco no estilo visionOS sobre um cômodo desfocado, com a mesma estrutura:
  - barra de cima em pílula: avatar + "Bem-vindo de volta", busca com o assistente (o Oti no lugar do orbe),
    Claro/Escuro segmentado e o sino de avisos;
  - trilho vertical de ícones à esquerda (item ativo em círculo branco);
  - painel grande com a grade de blocos (bento): Jogos (busca + 3 capas), Banda de fundo (interruptor + programas),
    Último jogo (o "player"), Avisos (lembretes + número), anéis verdes (processador, memória, tela, Wi-Fi),
    ações rápidas (4 atalhos coloridos + linhas), Tela (o "ar-condicionado": 60 Hz grande, setas e linhas),
    Wi-Fi sem varredura e Caçador de DPC (interruptores);
  - abas embaixo com a pílula branca que desliza, e o "+" com as outras áreas.

## Comportamento
- Cada bloco mostra um dado lido deste PC ou liga algo que existe de verdade. O que ainda não migrou abre a área
  dizendo a onda em que chega. Nada finge funcionar.
- A luz do vidro segue o cursor em cada bloco; bloco sobe um pouco no hover; interruptores com mola; trocar de área
  entra com desfoque e escala; a pílula das abas desliza.
- O Oti fala num balão que desce da busca, letra a letra. A pergunta é entendida por palavra-chave (não é IA) e ele
  admite quando não entende. Navegar fecha o balão.
- **Jogo aberto**: o Oti avisa ("bom jogo") e só então a interface congela (classe `jogo-aberto`: sem animação, sem
  desfoque, vidro opaco).

## Regras
- Texto `#1d1d1f` no claro, `#f5f5f7` no escuro; secundário cinza. Ativo = branco sólido. Ligado = verde `#34c759`.
- Estado em ponto colorido ao lado do texto. Nunca borda colorida na lateral, texto em degradê ou número decorativo.
- Estados (lendo, vazio, erro, sem administrador) sempre desenhados, com o que fazer.
- Para conferir o mascote: `npm run nova:web` e `/?galeria`; o ícone do app sai de `/?icone` →
  `brand/nova-geracao/oti-icone.png` → `npx tauri icon`.
