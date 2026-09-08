# Política de Segurança do Otimiza

## Visão Geral

O Otimiza é um programa que altera configurações do sistema Windows. Como tal, a segurança é fundamental. Este documento descreve como abordamos segurança e como relatar vulnerabilidades.

## Princípios de Segurança

### O Que Nós Fazemos

- **Leitura segura:** Usamos registro do Windows e WMI, não parsing de comandos que pode falhar em diferentes idiomas
- **Reversibilidade completa:** Cada mudança grava o estado anterior antes de escrever
- **Detecção de hardware:** Leemos a configuração antes de oferecer otimizações
- **Transparência:** Mostramos exatamente o que está sendo feito enquanto fazemos

### O Que Nós Recusamos a Fazer

- Desativar proteções da CPU contra Spectre/Meltdown
- Desligar Windows Update, Defender ou firewall
- "Limpeza de registro" (não tem ganho medível e quebra programas)
- Liberar RAM à força (deixa o PC mais lento)
- Escrever na BIOS (errar ali inutiliza a placa-mãe)

## Relatando Vulnerabilidades

Se você encontrar uma vulnerabilidade de segurança no Otimiza, por favor:

1. **Não crie uma issue pública** — isso exporia usuários a risco
2. **Envie um email descrevendo:**
   - A vulnerabilidade
   - Como reproduzir
   - Impacto potencial
   - Se você tem uma correção proposta

3. **Nós responderemos em até 7 dias** com:
   - Confirmação de recebimento
   - Plano de correção
   - Timeline estimada

4. **Após correção:** Publicaremos creditando sua descoberta (se desejar)

### O Que Constitui Vulnerabilidade

- Escalonamento de privilégios
- Execução de código arbitrário
- Exposição de dados sensíveis
- Falha de reversibilidade que deixa o sistema em estado inconsistente
- Bypass de verificações de segurança

## Desenvolvimento Seguro

### Revisão de Código

- Mudanças que alteram configurações do sistema requerem revisão
- Novas otimizações devem ser testadas em múltiplos hardwares
- Verificação de reversibilidade é obrigatória

### Testes

- Otimizações são medidas antes/depois em hardware real
- Limiares de ruído vêm de teste, não de chute
- Funcionalidade não testada aparece como pendente em `PROGRESS.md`

### Assinatura Digital

**O instalador ainda não é assinado.** Na primeira execução o Windows mostra
"O Windows protegeu o seu PC — editor desconhecido", e o aviso está certo: sem
assinatura, o sistema não tem como saber quem publicou o arquivo.

O motivo não é descuido. Desde 2023 as autoridades certificadoras não emitem
mais certificado de assinatura de código como arquivo simples — a chave privada
precisa ficar em hardware certificado —, e essa compra ainda não foi feita. O
processo, o custo e o que falta estão em
[`pc-optimizer/docs/ASSINATURA.md`](pc-optimizer/docs/ASSINATURA.md).

Até lá, o que está no seu alcance: baixar apenas da página de versões deste
repositório, e ler o código, que é público.

## Dados do Usuário

### O Que é Coletado

- **Nenhum dado da sua máquina é enviado para lugar nenhum**
- Nenhuma telemetria, nenhum analytics, nenhuma conta
- Estado das mudanças é salvo localmente em `%APPDATA%\pc-optimizer\changes.json`
- **A única requisição que o programa faz** é uma pergunta ao GitHub — saiu
  versão nova? Ela é anônima, manda apenas um User-Agent com o número da versão
  instalada, e sem resposta o programa apenas não avisa e segue funcionando.
  O código está em `pc-optimizer/src-tauri/src/modules/atualizacao.rs`

### O Que é Alterado

- Configurações do Windows (registro, serviços)
- Configurações de hardware (quando seguro e reversível)
- Nenhuma dessas informações sai da máquina — a única coisa que sai é a
  pergunta de versão descrita acima

## Atualizações de Segurança

- Correções de segurança são prioridade máxima
- Serão lançadas o mais rápido possível
- Usuários serão notificados através dos canais oficiais

## Perguntas Frequentes

### O Otimiza é seguro?

Sim, quando usado conforme documentado. O programa:
- Só oferece otimizações apropriadas para seu hardware
- É completamente reversível
- Não desativa proteções de segurança
- Não envia dados para fora

### O Otimiza pode quebrar meu PC?

Não, porque:
- Cada mudança é reversível byte a byte
- Hardware é detectado antes de oferecer otimizações
- O programa recusa práticas perigosas
- Estado anterior é sempre salvo

### Posso confiar no instalador?

Ele **não é assinado**, então o Windows vai avisar que o editor é desconhecido —
e vai estar certo em avisar. Ver "Assinatura Digital", acima.

O que dá para fazer no lugar disso: baixar só da página de versões deste
repositório, e ler o código-fonte, que é público inteiro. Um programa que altera
configurações do seu sistema deveria poder ser auditado por quem instala.

## Contato de Segurança

Para relatar vulnerabilidades ou questões de segurança, use os canais privados mencionados acima em "Relatando Vulnerabilidades".

Não use issues públicas para relatar vulnerabilidades de segurança.
