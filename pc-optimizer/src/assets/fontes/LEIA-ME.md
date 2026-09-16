# As fontes do Otimiza

Três arquivos, todos **variáveis** (um arquivo cobre todos os pesos) e todos
empacotados dentro do programa. Somam 79 KB.

| Arquivo | Família | Onde aparece |
|---|---|---|
| `jakarta-variavel.woff2` | Plus Jakarta Sans | títulos (`--font-display`) |
| `geist-variavel.woff2` | Geist | texto de leitura (`--font-sans`) |
| `geist-mono-variavel.woff2` | Geist Mono | número, código e rótulo (`--font-mono`) |

## Por que embutidas, e não baixadas

O Otimiza precisa abrir igual sem internet. Uma fonte buscada em tempo de
execução deixaria a primeira tela trocando de letra na frente do cliente — e,
sem rede, ele veria o programa inteiro na fonte do sistema.

São as mesmas três do site (`web/`), porque o programa e a página são o mesmo
produto: o cliente não deveria sentir que trocou de empresa ao instalar.

## Licença

As duas famílias são publicadas sob a **SIL Open Font License 1.1**, que
permite uso, redistribuição e empacotamento dentro de um produto, inclusive
comercial. O que a licença proíbe é vender a fonte sozinha.

- Plus Jakarta Sans — Tokotype
- Geist e Geist Mono — Vercel

Os arquivos são os subconjuntos latinos gerados pelo `next/font` durante o build
do site, copiados sem alteração.

## Ao trocar

Sobrescrever o arquivo mantendo o nome basta: quem aponta para eles é o
`@font-face` no topo de `src/styles.css`.
