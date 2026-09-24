# Setup Inicial - PC Performance Optimizer

## ⚠️ Pré-requisito Importante para Windows

Antes de continuar, você precisa instalar o **Visual Studio Build Tools** para compilar o projeto Rust no Windows.

### Instalação do Visual Studio Build Tools

1. **Baixe o instalador:**
   - Acesse: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
   - Clique em "Download" em "Build Tools for Visual Studio 2022"

2. **Execute o instalador:**
   - Quando o instalador abrir, selecione **"Desktop development with C++"**
   - Aguarde a instalação (pode levar 10-20 minutos)

3. **Reinicie o terminal/PowerShell** após a instalação

### Verificar Instalação

Após instalar, execute:
```powershell
cargo check
```

Se funcionar sem erro de linker, está tudo pronto! ✅

---

## 🚀 Primeiros Passos

### 1. Instalar Dependências

```bash
cd pc-optimizer
npm install
```

### 2. Executar em Modo Desenvolvimento

```bash
npm run tauri dev
```

Isso irá:
- Compilar o backend Rust
- Iniciar o servidor de desenvolvimento Vite
- Abrir a janela do aplicativo
- Habilitar hot-reload no frontend

### 3. Testar Funcionalidades

Na interface que abrir:

1. **Informações do Sistema**
   - Deve mostrar sua plataforma, OS, arquitetura
   - Clique em "Atualizar Informações" para recarregar

2. **Diagnóstico do Sistema**
   - Clique em "Executar Diagnóstico"
   - Verá um relatório com health score e informações do sistema
   - (Por enquanto é um placeholder - será implementado nas próximas tasks)

3. **Teste de Conexão**
   - Digite seu nome e clique em "Saudar"
   - Verifica se a comunicação IPC está funcionando

---

## 🏗️ Estrutura Criada

### Backend (Rust) - `src-tauri/src/`

✅ **Core Modules:**
- `core/engine.rs` - Motor principal (CoreEngine)
- `core/platform.rs` - Detecção de plataforma
- `core/config.rs` - Gerenciamento de configuração

✅ **Functional Modules:**
- `modules/diagnostic.rs` - Motor de diagnóstico
- `modules/optimizer.rs` - Sistema de otimização
- `modules/safety.rs` - Validação de segurança
- `modules/monitor.rs` - Monitor de performance

✅ **Infrastructure:**
- `commands.rs` - Comandos Tauri (IPC)
- `utils/logger.rs` - Sistema de logging

### Frontend - `src/`

✅ **Interface:**
- `main.ts` - Lógica principal TypeScript
- `styles.css` - Estilos modernos com gradientes
- `index.html` - Estrutura HTML

### Testes

✅ **Unit Tests:**
- Testes unitários em cada módulo Rust
- Execute: `cd src-tauri && cargo test`

---

## 📊 Status Atual

### ✅ Implementado (Task 1)

- [x] Projeto Tauri configurado com TypeScript + Rust
- [x] Estrutura modular completa
- [x] Core Engine com inicialização
- [x] Detecção de plataforma funcionando
- [x] Sistema de configuração
- [x] Safety Validator com lista de serviços críticos
- [x] Estrutura base do Diagnostic Engine
- [x] Estrutura base do Performance Monitor
- [x] Frontend funcional com 3 seções
- [x] Comandos IPC (get_platform_info, run_diagnostic, greet)
- [x] Documentação completa (README, COMMANDS, SETUP)

### ⏳ Próximos Passos (Task 2+)

- [ ] Task 2.1: Implementar testes de plataforma
- [ ] Task 2.3: Core Engine com factory pattern
- [ ] Task 3: Implementar análises reais de CPU/RAM/Disk/GPU
- [ ] Task 4: Checkpoint - Verificar diagnóstico
- [ ] Task 5: Safety Validator + Backup Manager
- [ ] Task 6-22: Otimizações, automação, UI completa, etc.

---

## 🔧 Comandos Úteis

### Desenvolvimento
```bash
npm run tauri dev          # Modo desenvolvimento
npm run build              # Build frontend
npm run tauri build        # Build completo (cria executável)
```

### Testes
```bash
cd src-tauri
cargo test                 # Todos os testes
cargo test --test integration_test  # Testes de integração
cargo check                # Verificar sem compilar
cargo clippy               # Linter Rust
```

### Debugging
```bash
# DevTools abre automaticamente em modo dev
# Logs Rust aparecem no terminal
```

---

## 🎯 Próximo Passo Recomendado

Execute o aplicativo e verifique se tudo funciona:

```bash
npm run tauri dev
```

Se abrir a janela e mostrar as informações da plataforma corretamente, a Task 1 está 100% completa! 🎉

---

## ❓ Problemas Comuns

### Erro "linker not found"
- **Solução:** Instale Visual Studio Build Tools (ver seção acima)

### Erro ao instalar npm dependencies
- **Solução:** Certifique-se que tem Node.js 18+ instalado
- Execute: `node --version`

### Janela não abre em dev mode
- **Solução:** Verifique se não tem firewall bloqueando
- Tente executar como administrador

### Erro de compilação Rust
- **Solução:** Atualize Rust: `rustup update`
- Limpe cache: `cargo clean`

---

## 📚 Documentação Adicional

- `README.md` - Visão geral e arquitetura
- `COMMANDS.md` - Documentação da API Tauri
- `.kiro/specs/pc-performance-optimizer/` - Especificações completas
  - `requirements.md` - Requisitos funcionais
  - `design.md` - Design da arquitetura
  - `tasks.md` - Plano de implementação

---

## 💡 Dica

O projeto está configurado para **hot-reload**. Faça mudanças no código e veja atualizações em tempo real:
- Frontend (TS/CSS/HTML): Reload automático instantâneo
- Backend (Rust): Recompila automaticamente (pode levar alguns segundos)

Bom desenvolvimento! 🚀
