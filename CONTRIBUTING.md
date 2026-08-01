# Contribuindo com o Otimiza

Obrigado pelo interesse em contribuir! Este documento guia como você pode participar do desenvolvimento do Otimiza.

## Visão Geral

O Otimiza é um otimizador de PC para Windows que se diferencia por:
- Medir antes e depois, admitindo quando não houve ganho
- Ser completamente reversível
- Recusar práticas perigosas (desativar proteções de segurança, etc.)
- Ler hardware antes de oferecer otimizações

## Stack Tecnológico

- **Backend:** Rust + Tauri 2
- **Frontend:** TypeScript + Vite (sem framework)
- **Leitura do sistema:** Registro do Windows e WMI (não parsing de comandos)

## Como Começar

### Pré-requisitos

- Node.js (veja `pc-optimizer/package.json` para versões compatíveis)
- Rust e Cargo
- Windows 10 ou superior (o programa é específico para Windows)

### Configuração do Ambiente

```bash
cd pc-optimizer
npm install
```

### Rodar em Desenvolvimento

```bash
npm run tauri dev
```

### Gerar Instaladores

```bash
npm run tauri build
```

Os instaladores saem em `pc-optimizer/src-tauri/target/release/bundle/`.

## Diretrizes de Contribuição

### Princípios Fundamentais

1. **Nunca comprometa a segurança**
   - Não desative proteções Spectre/Meltdown
   - Não desative Windows Update, Defender ou firewall
   - Não faça "limpeza de registro" que quebra programas instalados

2. **Sempre meça antes e depois**
   - Otimizações devem ter ganho verificável
   - Se não houver ganho, admita isso
   - Use limiares baseados em teste real, não chutados

3. **Seja reversível**
   - Cada mudança deve gravar o estado anterior
   - Desfazer deve restaurar byte a byte, não algo "equivalente"

4. **Leia o hardware antes de oferecer**
   - Detecte a configuração da máquina
   - Não ofereça otimizações que fariam mal àquele hardware específico

5. **Não parseie comandos**
   - Use registro e WMI para ler o sistema
   - Parsing de texto quebra em Windows em outros idiomas

### Código

- **Backend (Rust):** Siga as convenções do Rust. Use `cargo clippy` e `cargo fmt`.
- **Frontend (TypeScript):** Siga as convenções do TypeScript. Não use framework.
- **Commits:** Seja descritivo. Explique o "porquê", não só o "o quê".

### Testes

Antes de propor uma otimização nova:
1. Meça a mesma máquina três vezes sem alterar nada
2. Identifique o ruído natural
3. Só reporte ganho acima desse ruído
4. Documente em `pc-optimizer/PROGRESS.md` o que foi verificado

Funcionalidade que existe no código mas nunca foi executada aparece como pendente no `PROGRESS.md`.

### Documentação

- Atualize `pc-optimizer/PROGRESS.md` com o que foi verificado
- Documente novas otimizações com:
  - O que faz
  - Quando ajuda (hardware específico)
  - Quando atrapalha (hardware específico)
  - Como foi testado

## Processo de Pull Request

1. Fork o repositório
2. Crie uma branch para sua feature (`git checkout -b feature/nova-feature`)
3. Commit suas mudanças (`git commit -m 'Adiciona nova feature'`)
4. Push para a branch (`git push origin feature/nova-feature`)
5. Abra um Pull Request

Descreva:
- O que a mudança faz
- Como foi testada
- Por que é segura
- Qual hardware foi testado

## Licença

Código aberto à leitura, não ao uso. Veja [`LICENSE`](LICENSE).

O código está público porque um programa que altera configurações do seu sistema deveria poder ser auditado. Isso não é o mesmo que licença de uso: copiar, redistribuir ou usar comercialmente exige autorização por escrito.

## Contato

Para dúvidas sobre contribuição, abra uma issue no repositório.
