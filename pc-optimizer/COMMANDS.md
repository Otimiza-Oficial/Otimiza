# Tauri Commands API

Documentação dos comandos IPC disponíveis para comunicação frontend-backend.

## Platform Commands

### `get_platform_info`

Retorna informações sobre a plataforma atual.

**Parâmetros:** Nenhum

**Retorno:**
```typescript
{
  platform: string,    // "Windows", "Linux", "MacOS", ou "Unknown"
  os_type: string,     // Tipo do OS (ex: "windows", "linux")
  arch: string,        // Arquitetura (ex: "x86_64", "aarch64")
  version: string      // Versão do OS
}
```

**Exemplo:**
```typescript
import { invoke } from "@tauri-apps/api/core";

const info = await invoke("get_platform_info");
console.log(info.platform); // "Windows"
```

---

## Engine Commands

### `initialize_engine`

Inicializa o motor principal do aplicativo.

**Parâmetros:** Nenhum

**Retorno:** `string` - Mensagem de sucesso

**Exemplo:**
```typescript
await invoke("initialize_engine");
```

---

## Diagnostic Commands

### `run_diagnostic`

Executa diagnóstico completo do sistema.

**Parâmetros:** Nenhum

**Retorno:**
```typescript
{
  timestamp: number,
  system_info: {
    os: string,
    os_version: string,
    cpu_name: string,
    cpu_cores: number,
    total_ram_gb: number,
    gpu_name: string
  },
  bottlenecks: Array<{
    component: "CPU" | "RAM" | "Disk" | "GPU" | "Network",
    severity: "Critical" | "High" | "Medium" | "Low",
    description: string,
    impact: string,
    suggested_fix: string
  }>,
  health_score: number,  // 0-100
  recommendations: string[]
}
```

**Exemplo:**
```typescript
const report = await invoke("run_diagnostic");
console.log(`Health Score: ${report.health_score}/100`);
console.log(`Bottlenecks: ${report.bottlenecks.length}`);
```

---

## Utility Commands

### `greet`

Comando de teste/exemplo para verificar comunicação IPC.

**Parâmetros:**
- `name: string` - Nome para saudação

**Retorno:** `string` - Mensagem de saudação

**Exemplo:**
```typescript
const greeting = await invoke("greet", { name: "João" });
console.log(greeting); // "Olá, João! Bem-vindo ao PC Performance Optimizer."
```

---

## Próximos Comandos (em desenvolvimento)

Os seguintes comandos serão adicionados nas próximas tasks:

### Optimization Commands
- `apply_optimization` - Aplica uma otimização específica
- `apply_all_optimizations` - Aplica todas otimizações seguras
- `get_available_optimizations` - Lista otimizações disponíveis
- `rollback_optimization` - Reverte uma otimização

### Safety Commands
- `validate_operation` - Valida segurança de uma operação
- `create_restore_point` - Cria ponto de restauração
- `list_restore_points` - Lista pontos de restauração
- `restore_from_point` - Restaura sistema de um ponto

### Monitor Commands
- `start_monitoring` - Inicia monitoramento em tempo real
- `stop_monitoring` - Para monitoramento
- `get_current_metrics` - Obtém métricas atuais
- `get_metrics_history` - Obtém histórico de métricas

### Gaming Commands
- `enable_gaming_mode` - Ativa modo gaming
- `disable_gaming_mode` - Desativa modo gaming
- `get_current_fps` - Obtém FPS atual

---

## Tratamento de Erros

Todos os comandos podem lançar erros. Use try-catch:

```typescript
try {
  const result = await invoke("run_diagnostic");
  // Processar resultado
} catch (error) {
  console.error("Erro ao executar diagnóstico:", error);
  // Exibir mensagem ao usuário
}
```

---

## Estado Global

O aplicativo mantém estado global acessível via `AppState`:
- `engine: CoreEngine` - Motor principal (com Mutex para thread-safety)

Este estado é compartilhado entre todos os comandos e garante consistência.
