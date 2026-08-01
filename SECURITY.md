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

O instalador é assinado digitalmente. Veja [`pc-optimizer/docs/ASSINATURA.md`](pc-optimizer/docs/ASSINATURA.md) para detalhes.

## Dados do Usuário

### O Que é Coletado

- Nenhum dado é enviado para servidores externos
- Estado das mudanças é salvo localmente em `%APPDATA%\pc-optimizer\changes.json`
- Nenhuma telemetria ou analytics

### O Que é Alterado

- Configurações do Windows (registro, serviços)
- Configurações de hardware (quando seguro e reversível)
- Nada é enviado para fora da máquina local

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

Sim, o instalador é assinado digitalmente. Verifique a assinatura antes de instalar.

## Contato de Segurança

Para relatar vulnerabilidades ou questões de segurança, use os canais privados mencionados acima em "Relatando Vulnerabilidades".

Não use issues públicas para relatar vulnerabilidades de segurança.
