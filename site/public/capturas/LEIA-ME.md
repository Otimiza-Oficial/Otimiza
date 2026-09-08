# Capturas de tela do Otimiza

As sete abas do aplicativo, uma por arquivo. O site le por estes nomes — se o
nome nao bater, a imagem some da pagina.

| Arquivo | Aba | Onde aparece no site |
|---|---|---|
| `painel.png` | Painel | Hero, logo abaixo dos botoes |
| `otimizacoes.png` | Otimizacoes | Secao dos pilares, ao lado da PROVA |
| `diagnostico.png` | Diagnostico | Galeria, em largura inteira |
| `jogos.png` | Jogos | Galeria |
| `espaco.png` | Espaco | Galeria |
| `sistema.png` | Sistema | Galeria |
| `reparo.png` | Reparo | Galeria |

## Ao trocar uma captura

Basta sobrescrever o arquivo com o mesmo nome. Nao precisa mexer em codigo.

Se a aba mudar de conteudo a ponto da legenda ficar errada, a legenda esta em
`src/data/i18n/{pt,en,es}.ts` — e precisa mudar nas TRES.

## Antes de salvar, apague daqui

Estes arquivos vao para a internet publica:

- **O codigo da maquina** (`OTZ-XXXX-XXXX-XXXX`)
- Nome de usuario do Windows em caminho de pasta
- Numero de serie de disco ou placa
- Nome de arquivo pessoal na tela de Espaco

As sete atuais foram conferidas uma a uma e estao limpas.

## Como tirar

1. Otimiza **maximizado**, monitor de 1920x1080 ou maior
2. `Win + Shift + S`, retangulo, **so a janela** — sem a barra de tarefas
3. **PNG**, nunca JPG: JPG borra texto pequeno

Largura de 2400px basta; acima disso so pesa a pagina.
