Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_..._x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_..._x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

---

# 0.28.0 — o Otimiza virou um produto

Esta página cobre tudo o que mudou desde a 0.17.

## A chave

O diagnóstico continua livre. Ele roda inteiro, mede tudo e mostra o que achou
sem pedir nada. **O que a chave libera são as correções.**

A chave é presa a um computador só. O código dele — `OTZ-XXXX-XXXX-XXXX` —
nasce do número de série da placa-mãe, e aparece na primeira tela do programa
com um botão de copiar ao lado.

**Formatar o Windows não muda esse código.** Trocar a placa-mãe muda; nesse caso
a chave é reemitida sem custo.

### O que a chave não faz

Ela não impede pirataria. Nenhuma licença que roda no computador do cliente
impede — quem editar o executável passa, e isso vale para qualquer programa
vendido assim. O que ela impede é o repasse casual: a chave que abre o seu PC
não abre o do vizinho.

Vale dizer isso agora e não deixar você descobrir depois.

## A interface, refeita

O programa passou a desenhar a própria janela, com os botões no canto e o título
no meio. O fundo é preto de verdade — em monitor OLED, que é o que boa parte de
quem joga usa, preto puro é pixel desligado.

A tela de ativação e a de agradecimento são novas. As duas mostram **os três
pilares**: uma coluna para o processador, uma para a memória e uma para o disco,
cada uma se desfazendo conforme a medição da peça que ela representa. Não é
enfeite — é o seu computador desenhado com os números que acabaram de ser lidos.

Sete abas viraram seis e três cartões que respondiam a mesma pergunta viraram
um.

## Correções

| O que estava errado | O que acontecia |
|---|---|
| Os botões da janela tinham 12px e ficavam cinzas | Não se achava como fechar o programa |
| O número dos medidores ficava longe do rótulo | Lia-se "19% MEMÓRIA" como par, e cada número parecia do vizinho |
| A bolinha de contagem empurrava o ícone da lateral | Nenhum ícone ficava no centro |
| `hidden` não escondia nada no CSS | Elementos escondidos apareciam vazios, como bolas brancas soltas |
| A rolagem estava travada | Conteúdo longo não descia |
| O nome do comprador saía com o código da compra colado | A tela de agradecimento mostrava "Obrigado, fulano.." |

O catálogo cresceu para o tamanho do que o mercado oferece, e cada item novo
entrou com a etiqueta honesta: o que muda FPS está marcado como tal, e o que é
higiene de Windows diz que é higiene de Windows. Os itens que trocam segurança
por desempenho ficam fora do botão automático, com o risco escrito e confirmação
própria.

## Atualizações

Quem comprou tem direito às versões seguintes — a chave não vence e não está
presa a versão nenhuma. Quando sair uma nova, o bot do Discord avisa por mensagem
direta com o link.

O instalador tem agora um endereço fixo que sempre aponta para a versão mais
nova, então o link que você guardar hoje continua valendo daqui a seis meses.

---

## O que esta versão não promete

**O catálogo maior não cria FPS.** A maior parte dos itens que entraram é higiene
de Windows, e cada linha continua dizendo isso sobre si mesma. O que move o
número no jogo continua sendo a configuração dele e o hardware.

**O "editor desconhecido" continua aparecendo.** O aviso do SmartScreen, lá em
cima, não é frescura do Windows: é ele dizendo, com razão, que não sabe quem
publicou este instalador. Resolver isso é comprar um certificado de assinatura,
e essa compra ainda não foi feita.
