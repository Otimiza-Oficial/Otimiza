Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.23.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.23.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

---

# Interface redesenhada: clean, profissional e funcional

A versão 0.23.0 traz uma reformulação completa da interface visual, mantendo a
filosofia do produto: honestidade, clareza e medições reais.

## O que mudou

### Topbar moderna e limpa
- Design glassmorphism com blur sutil e gradientes
- Bordas com gradiente diagonal para profundidade
- Sombras em camadas para hierarquia visual
- Busca no topo com hover animado e efeito de elevação

### Scroll funcional
- Rolagem vertical agora funciona corretamente em todo o conteúdo
- Scrollbar customizada que combina com o tema escuro
- Navegação fluida entre seções longas

### Telas de login e agradecimento aprimoradas
- **Portão (login)**: Fundo com gradientes radiais sutis, backdrop-filter blur
  aumentado, caixa com bordas em gradiente, integração suave dos pilares com
  gradiente horizontal
- **Chegada (agradecimento)**: Cartão glassmorphism com blur 12px por trás do
  texto, gradiente de dissolução da arte dos pilares em direção ao conteúdo,
  animações escalonadas mantidas

### Os pilares continuam protagonistas
As três colunas clássicas — uma para processador, memória e disco — continuam
sendo desenhadas com as próprias medições desta máquina. Não é ilustração: é o
computador da pessoa, com cada coluna se desfazendo pela leitura da peça que
representa.

## O que não mudou

A interface ficou mais limpa e moderna, mas o núcleo continua o mesmo:
- Números medidos, não inventados
- Otimizações reversíveis
- Comparação antes/depois honesta
- Veredito baseado em dados reais da máquina

## Detalhes técnicos

- Adicionadas variáveis CSS: `--panel-rgb`, `--t-h2`, `--t-nota`
- Aliases de cores: `--ok`, `--aviso`, `--erro`, `--borda-forte`
- Consistência tipográfica melhorada
- Efeitos de profundidade com múltiplas camadas de sombra
- Backdrop-filter para efeitos glassmorphism onde apropriado

---

## O que esta versão não promete

Interface bonita não muda desempenho. O que move o número no jogo continua
sendo a configuração dele e o hardware — e o programa continua dizendo isso
na primeira coisa que você lê, agora com uma interface mais limpa.
