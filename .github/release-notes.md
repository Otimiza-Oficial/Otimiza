Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.2.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.2.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

Boa parte das otimizações mexe em configurações protegidas do sistema e precisa
de administrador. O Otimiza avisa quando isso acontece e reabre com permissão, se
você deixar — nunca por conta própria.

## Novidades desta versão

**Interface em cinco abas.** Painel, Otimizações, Diagnóstico, Resultado e
Sistema. Os sinais vitais de CPU, memória e disco ficam fixos no topo: trocar de
aba não faz você perder de vista o que a máquina está fazendo.

**Gerenciador de inicialização.** Mostra o que sobe junto com o Windows e permite
desligar, escrevendo no mesmo lugar que o Gerenciador de Tarefas usa. Nada é
apagado — e desfazer restaura o valor anterior byte a byte.

**Quem está pesando agora.** Lista ao vivo dos programas que mais consomem, com o
selo de quem volta sozinho no próximo boot.

**Ponto de restauração antes de otimizar.** E, se não der para criar, o Otimiza
diz o motivo real em vez de fingir que criou.

**Firmware e hardware.** Detecta memória em canal único, XMP desligado, limites de
núcleo no boot e queda de desempenho por temperatura. Cada achado diz onde se
resolve: software, BIOS ou troca de peça.

**Sete otimizações novas**, incluindo interrupções diretas da placa de vídeo,
impedir que a placa de rede durma, tirar a busca da internet do menu Iniciar e
parar de compartilhar atualizações do Windows pela sua banda de subida.

## O que o Otimiza não faz

Existe uma lista de coisas que rendem desempenho e que nós **recusamos** fazer.
Ela está dentro do programa, na aba Otimizações:

- Desativar as proteções da CPU contra Spectre e Meltdown
- Desligar Windows Update, Defender ou firewall
- "Limpeza de registro", que não tem ganho medível e quebra programa instalado
- Liberar memória à força, o que deixa o gráfico bonito e o PC mais lento
- Escrever na BIOS — em placa de consumo, errar ali inutiliza a placa-mãe

## Como conferir o resultado

Na aba **Resultado**: meça antes, otimize, meça depois. Se o ganho não aparecer,
o programa vai dizer isso. Ele se recusa a emitir veredito com o PC ocupado, e
duas das medições aparecem marcadas como "só referência" porque oscilam demais
sozinhas para provar qualquer coisa.

## Tudo é reversível

Cada mudança grava o valor anterior antes de escrever. "Desfazer tudo" restaura o
que existia, não algo parecido. A única exceção é apagar arquivos temporários,
que é marcada como **sem volta** e nunca entra no "Otimizar agora".

## Ainda não há versão para macOS e Linux

O motor de otimização é todo específico do Windows. Um instalador para as outras
plataformas hoje abriria um programa sem nenhuma função — preferimos não publicar
a publicar algo que não entrega.
