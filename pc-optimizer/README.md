# PC Performance Optimizer

Sistema multiplataforma de otimização de desempenho para Windows, Linux e MacOS construído com **Tauri + Rust**.

## 🚀 Características

- **Diagnóstico Inteligente**: Identifica gargalos de CPU, RAM, Disco e GPU
- **Otimização Segura**: Todas operações são reversíveis com restore points automáticos
- **Gaming Mode**: Otimizações específicas para aumentar FPS e reduzir input lag
- **Tweaks Avançados**: Registry tweaks, otimização de drivers e rede
- **Multiplataforma**: Suporte para Windows, Linux e MacOS
- **Interface Simples**: UI intuitiva com modo "1-clique"
- **Leve e Rápido**: Construído com Rust para máxima performance

## 📋 Pré-requisitos

### Windows
- [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
  - Durante instalação, selecione "Desktop development with C++"
- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/)

### Linux
```bash
sudo apt install libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

### MacOS
```bash
xcode-select --install
```

## 🛠️ Instalação

1. Clone o repositório
```bash
git clone <repo-url>
cd pc-optimizer
```

2. Instale as dependências
```bash
npm install
```

3. Execute em modo de desenvolvimento
```bash
npm run tauri dev
```

4. Compile para produção
```bash
npm run tauri build
```

## 📁 Estrutura do Projeto

```
pc-optimizer/
├── src/                    # Frontend (TypeScript + HTML/CSS)
│   ├── main.ts            # Lógica principal do frontend
│   └── styles.css         # Estilos
├── src-tauri/             # Backend (Rust)
│   ├── src/
│   │   ├── core/          # Motor principal
│   │   │   ├── engine.rs  # CoreEngine - coordenador central
│   │   │   ├── platform.rs # Detecção de plataforma
│   │   │   └── config.rs  # Gerenciamento de configuração
│   │   ├── modules/       # Módulos de funcionalidade
│   │   │   ├── diagnostic.rs  # Motor de diagnóstico
│   │   │   ├── optimizer.rs   # Módulo de otimização
│   │   │   ├── safety.rs      # Validador de segurança
│   │   │   └── monitor.rs     # Monitor de performance
│   │   ├── utils/         # Utilitários
│   │   │   └── logger.rs  # Sistema de logging
│   │   ├── commands.rs    # Comandos Tauri (IPC)
│   │   ├── lib.rs         # Biblioteca principal
│   │   └── main.rs        # Entry point
│   └── Cargo.toml         # Dependências Rust
└── package.json           # Dependências Node.js
```

## 🏗️ Arquitetura

### Core Engine
- **CoreEngine**: Coordenador central que orquestra todas operações
- **Platform Detector**: Detecta OS, versão e arquitetura
- **Configuration Manager**: Gerencia configurações e preferências

### Modules
- **Diagnostic Engine**: Analisa sistema e identifica gargalos
- **Optimization Module**: Aplica otimizações específicas por plataforma
- **Safety Validator**: Valida operações antes da execução
- **Backup Manager**: Cria restore points e backups
- **Performance Monitor**: Monitora métricas em tempo real

### Design Patterns
- **Strategy Pattern**: Módulos específicos por plataforma
- **Command Pattern**: Operações reversíveis
- **Observer Pattern**: Monitoramento em tempo real
- **Factory Pattern**: Criação de otimizadores baseado em plataforma

## 🧪 Testes

### Unit Tests
```bash
cd src-tauri
cargo test
```

### Property-Based Tests
```bash
cd src-tauri
cargo test --features property-tests
```

## 📝 Status de Desenvolvimento

✅ Task 1: Estrutura do projeto e infraestrutura core
- [x] Projeto Tauri configurado
- [x] Core Engine implementado
- [x] Platform Detection implementado
- [x] Configuration Management implementado
- [x] Diagnostic Engine (estrutura base)
- [x] Safety Validator implementado
- [x] Performance Monitor (estrutura base)
- [x] Frontend básico funcional

⏳ Próximas tarefas (conforme .kiro/specs/pc-performance-optimizer/tasks.md):
- Task 2: Implementar testes de plataforma
- Task 3: Completar Diagnostic Engine com análises reais
- Task 4: Checkpoint de diagnóstico
- Task 5-22: Otimizações, automação, UI completa, etc.

## 🔒 Segurança

Todas as otimizações passam por validação de segurança:
- Serviços críticos são protegidos
- Restore points automáticos antes de mudanças
- Validação contra whitelist/blacklist
- Rollback automático em caso de erro

## 📄 Licença

Ver arquivo LICENSE para detalhes.

## 🤝 Contribuindo

Contribuições são bem-vindas! Ver arquivo CONTRIBUTING.md para diretrizes.

## ⚠️ Aviso

Este software modifica configurações do sistema. Sempre crie backups antes de usar.
Use por sua própria conta e risco.
