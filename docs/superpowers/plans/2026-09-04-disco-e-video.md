# O disco inteiro, e a placa de vídeo — Plano de Implementação

> **Para agentes executores:** SUB-SKILL OBRIGATÓRIA: use
> `superpowers:subagent-driven-development` (recomendado) ou
> `superpowers:executing-plans` para implementar tarefa a tarefa. Os passos
> usam caixas (`- [ ]`) para acompanhamento.

**Objetivo:** o mapa de pastas passa a enxergar o disco inteiro (hoje só vê a
pasta do usuário, e por isso é cego para os 122 GB de Steam), e o produto passa
a detectar Resizable BAR e a ajustar o driver NVIDIA de forma reversível.

**Arquitetura:** `foldermap.rs` já resolve junction, permissão e prazo — muda só
para onde aponta e como classifica cada linha. `diskspace.rs` ganha as
categorias que são grandes de verdade. Dois módulos novos, `rbar.rs` (só
leitura, via `nvidia-smi`) e `nvdriver.rs` (escrita, via `nvapi64.dll` carregada
em tempo de execução).

**Tecnologias:** Rust (Tauri 2), TypeScript sem framework, PowerShell para
consulta ao Windows, `nvidia-smi` e `nvapi64.dll` (ambos acompanham o driver).

**Spec:** `docs/superpowers/specs/2026-09-04-disco-e-video-design.md`

## Restrições globais

- **Português do Brasil** em código, comentários e em tudo que o cliente lê.
- **Todo commit é da conta do dono:**
  `git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" commit`
- **Nunca mostrar número que não foi medido.** "Não consegui verificar" nunca
  vira "está tudo bem", e pasta ilegível nunca é contada como zero.
- **A interface NUNCA decide comparando prosa vinda do backend.** Existe uma
  guarda em `commands.rs` que reprova o build se `main.ts` fizer isso — ela já
  pegou o mesmo defeito três vezes. Toda decisão de tela vem de campo tipado.
- **Só se apaga o que o Windows recria sozinho.** Ponto de restauração,
  hibernação e arquivo do usuário estão FORA, por decisão do dono.
- **Arquivo marcado como do usuário nunca é apagado**, por nenhum caminho.
- **Toda escrita é reversível com o valor anterior registrado**, e entra no
  histórico que o "Desfazer tudo" percorre.
- Módulo novo entra em `.github/workflows/release.yml`, senão a guarda
  `ci_coverage::todo_modulo_com_teste_roda_na_esteira` reprova.
- Rodar `cargo test --lib` em `pc-optimizer/src-tauri` e `npx tsc --noEmit` em
  `pc-optimizer`. Hoje: **543 testes passando, tsc limpo.**

---

### Task 1: Cada linha do mapa diz o que ela é

Hoje `FolderEntry` não distingue "isto eu posso limpar" de "isto é seu" de "não
consegui ler". Sem isso, a tela do Pilar 1 teria que adivinhar — e adivinhar
olhando texto é o defeito que a guarda de prosa existe para impedir.

**Arquivos:**
- Modificar: `pc-optimizer/src-tauri/src/modules/windows/foldermap.rs`
- Testar: o módulo `#[cfg(test)]` do próprio arquivo

**Interfaces:**
- Consome: `FolderEntry` e `FolderMap`, que já existem.
- Produz:
  - `pub enum Natureza { PodeLimpar, Seu, NaoSei }`, com
    `#[derive(Serialize, Deserialize)]` e `#[serde(tag = "tipo")]`
  - campo `pub natureza: Natureza` em `FolderEntry`
  - `pub fn classificar(caminho: &str, leu: bool) -> Natureza`

- [ ] **Passo 1: Escrever o teste que falha**

No `mod tests` de `foldermap.rs`:

```rust
#[test]
fn pasta_que_nao_deu_para_ler_nunca_e_zero_nem_limpavel() {
    // "NAO SEI" E UM TERCEIRO ESTADO, e nao um zero.
    //
    // Uma pasta sem permissao aparecendo como 0 GB faria o total mentir, e o
    // cliente concluiria que o espaco sumiu no nada. E marca-la como
    // limpavel seria oferecer apagar o que nem foi possivel olhar.
    assert_eq!(classificar(r"C:\Windows\System32\config", false), Natureza::NaoSei);
}

#[test]
fn arquivo_do_cliente_nunca_e_marcado_como_limpavel() {
    // O PIOR ERRO POSSIVEL DESTE PROGRAMA. Apagar jogo ou download de quem
    // pagou nao tem desfazer. Steam, Downloads e Documentos sao DELE.
    for caminho in [
        r"C:\Program Files (x86)\Steam",
        r"C:\Users\Fulano\Downloads",
        r"C:\Users\Fulano\Documents",
        r"C:\Users\Fulano\Videos",
    ] {
        assert_eq!(
            classificar(caminho, true),
            Natureza::Seu,
            "{} foi marcado como limpavel", caminho
        );
    }
}

#[test]
fn categoria_conhecida_de_limpeza_e_marcada_como_limpavel() {
    // O que o `diskspace.rs` ja sabe limpar continua limpavel aqui, para as
    // duas telas nao discordarem uma da outra.
    assert_eq!(classificar(r"C:\Windows\Temp", true), Natureza::PodeLimpar);
    assert_eq!(
        classificar(r"C:\Windows\SoftwareDistribution\Download", true),
        Natureza::PodeLimpar
    );
}

#[test]
fn na_duvida_e_do_cliente_e_nao_limpavel() {
    // CANARIO. Uma pasta que ninguem reconhece nao pode cair em `PodeLimpar`
    // por descuido de ordem dos `if`. O padrao seguro e "e do cliente".
    assert_eq!(classificar(r"C:\MinhaPastaEstranha", true), Natureza::Seu);
}
```

- [ ] **Passo 2: Rodar e ver falhar**

Rodar: `cd pc-optimizer/src-tauri && cargo test --lib foldermap`
Esperado: FALHA — `classificar` e `Natureza` não existem.

- [ ] **Passo 3: Implementar**

Em `foldermap.rs`:

```rust
/// O que cada linha do mapa É, para a tela não precisar adivinhar.
///
/// CAMPO TIPADO, E NÃO FRASE. Este projeto reprova o build quando a interface
/// decide comparando prosa vinda do backend — já aconteceu três vezes, e a
/// guarda em `commands.rs` existe por causa disso.
///
/// E são TRÊS estados, não dois. "Não consegui ler" precisa ser distinto de
/// "não há nada aqui": uma pasta sem permissão contada como zero faria o total
/// mentir, e o cliente concluiria que o espaço sumiu no nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tipo")]
pub enum Natureza {
    /// Categoria que o `diskspace.rs` sabe limpar. Ganha botão na tela.
    PodeLimpar,
    /// Arquivo do cliente. O produto MOSTRA o caminho e não faz mais nada.
    Seu,
    /// Não deu para ler — permissão, ou o prazo da varredura estourou.
    NaoSei,
}

/// Prefixos que o `diskspace.rs` já sabe limpar.
///
/// A lista mora aqui em minúsculas porque caminho no Windows não diferencia
/// maiúscula de minúscula, e comparar sem normalizar deixaria `C:\WINDOWS\TEMP`
/// passar como pasta do cliente.
const LIMPAVEIS: &[&str] = &[
    r"\windows\temp",
    r"\appdata\local\temp",
    r"\windows\softwaredistribution\download",
    r"\windows.old",
    r"\programdata\microsoft\windows\wer",
    r"\windows\servicing\logfiles",
    r"\windows\logs",
];

/// Decide o que uma pasta é. Pura, testável sem disco.
///
/// A ORDEM DOS TESTES IMPORTA, e o padrão é o seguro: o que não for
/// reconhecido como limpável é do cliente. Inverter isso ofereceria apagar
/// pasta desconhecida, e apagar arquivo de quem pagou não tem desfazer.
pub fn classificar(caminho: &str, leu: bool) -> Natureza {
    if !leu {
        return Natureza::NaoSei;
    }

    let minusculo = caminho.to_lowercase().replace('/', "\\");

    if LIMPAVEIS.iter().any(|p| minusculo.contains(p)) {
        return Natureza::PodeLimpar;
    }

    Natureza::Seu
}
```

Acrescentar o campo em `FolderEntry` (linha ~64), depois de `partial`:

```rust
    /// O que esta linha é: limpável, do cliente, ou ilegível. Ver `Natureza`.
    pub natureza: Natureza,
```

E preencher em `mapear`, onde cada `FolderEntry` é construído: `natureza:
classificar(&caminho_texto, leu_com_sucesso)`, usando a variável que a função
já tem para saber se a leitura deu certo.

- [ ] **Passo 4: Rodar e ver passar**

Rodar: `cargo test --lib foldermap`
Esperado: PASSA.

- [ ] **Passo 5: Provar por mutação**

Trocar o padrão de `Natureza::Seu` para `Natureza::PodeLimpar` na última linha
de `classificar`, rodar, e confirmar que `na_duvida_e_do_cliente_e_nao_limpavel`
REPROVA. Depois desfazer. **Teste que não reprova não é teste** — e neste
projeto já houve três testes que afirmavam algo que já era verdade antes da
correção.

- [ ] **Passo 6: Commitar**

```bash
git add pc-optimizer/src-tauri/src/modules/windows/foldermap.rs
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" \
  commit -m "Cada linha do mapa diz se e limpavel, do cliente, ou ilegivel"
```

---

### Task 2: O mapa alcança fora da pasta do usuário

É o furo que motivou o plano: 122 GB de Steam em `Program Files (x86)`, fora do
alcance do mapa.

**Arquivos:**
- Modificar: `pc-optimizer/src-tauri/src/modules/windows/foldermap.rs`
- Modificar: `pc-optimizer/src-tauri/src/commands.rs:406-407`
- Testar: o módulo `#[cfg(test)]` de `foldermap.rs`

**Interfaces:**
- Consome: `mapear(raiz: &Path, limite: usize) -> Result<FolderMap, String>` e
  `Natureza` (Task 1).
- Produz: `pub fn raizes_do_disco() -> Vec<PathBuf>` e
  `pub fn mapear_o_disco(limite: usize) -> Result<FolderMap, String>`.

- [ ] **Passo 1: Escrever o teste que falha**

```rust
#[test]
fn as_raizes_incluem_onde_os_jogos_ficam() {
    // O FURO QUE ESTE PLANO EXISTE PARA FECHAR.
    //
    // Medido na maquina do dono: 122 GB de Steam em `Program Files (x86)`,
    // contra 176 GB em AppData. O mapa so varria o perfil do usuario, entao
    // era cego para a SEGUNDA MAIOR coisa da maquina -- num produto cujo
    // publico inteiro e jogador.
    let raizes: Vec<String> = raizes_do_disco()
        .iter()
        .map(|p| p.to_string_lossy().to_lowercase())
        .collect();

    let juntas = raizes.join(" | ");

    assert!(juntas.contains("program files (x86)"), "faltou: {}", juntas);
    assert!(juntas.contains("program files"), "faltou: {}", juntas);
    assert!(juntas.contains("users") || juntas.contains("usuários"), "faltou: {}", juntas);
}

#[test]
fn nenhuma_raiz_repetida() {
    // `Program Files` e prefixo de `Program Files (x86)` -- montar a lista com
    // um `contains` descuidado somaria a mesma pasta duas vezes, e o total do
    // mapa passaria do tamanho do disco.
    let raizes = raizes_do_disco();
    let mut vistos = std::collections::BTreeSet::new();

    for r in &raizes {
        assert!(
            vistos.insert(r.to_string_lossy().to_lowercase()),
            "raiz repetida: {:?}", r
        );
    }
}

#[test]
fn o_mapa_do_disco_roda_nesta_maquina() {
    // MEDICAO DE VERDADE, na maquina que roda o teste. O mapa pode voltar
    // vazio (maquina sem nada), mas nao pode QUEBRAR -- e se voltar totais,
    // eles precisam ser coerentes.
    let mapa = mapear_o_disco(12).expect("o mapa do disco nao pode falhar");

    for pasta in &mapa.folders {
        assert!(pasta.percent >= 0.0 && pasta.percent <= 100.0,
            "{} tem porcentagem impossivel: {}", pasta.name, pasta.percent);
    }

    // O total nunca pode ser menor que a maior pasta.
    if let Some(maior) = mapa.folders.iter().map(|f| f.bytes).max() {
        assert!(mapa.total_bytes >= maior, "o total e menor que a maior pasta");
    }
}
```

- [ ] **Passo 2: Rodar e ver falhar**

Rodar: `cargo test --lib foldermap`
Esperado: FALHA — `raizes_do_disco` e `mapear_o_disco` não existem.

- [ ] **Passo 3: Implementar**

```rust
/// As pastas de primeiro nível que valem varrer, na ordem em que costumam
/// pesar.
///
/// POR QUE UMA LISTA E NÃO `C:\` INTEIRO. Varrer a raiz do disco entra em
/// `System Volume Information` e no `$Recycle.Bin` de outros usuários — pastas
/// que devolvem erro de permissão e não acrescentam nada. A lista cobre onde o
/// espaço realmente vai, medido na máquina que motivou este trabalho:
/// AppData 176 GB, Steam 122 GB, Downloads 48 GB, Windows 29 GB.
///
/// Só entra o que existe: máquina sem `Program Files (x86)` (Windows ARM, por
/// exemplo) simplesmente não tem essa linha, em vez de mostrar uma pasta vazia.
pub fn raizes_do_disco() -> Vec<PathBuf> {
    let mut raizes: Vec<PathBuf> = Vec::new();

    let candidatos = [
        std::env::var("ProgramFiles").ok(),
        std::env::var("ProgramFiles(x86)").ok(),
        std::env::var("ProgramData").ok(),
        std::env::var("SystemRoot").ok(),
        Some(perfil_do_usuario().to_string_lossy().to_string()),
    ];

    for candidato in candidatos.into_iter().flatten() {
        let caminho = PathBuf::from(&candidato);

        // DEDUPLICAÇÃO POR TEXTO NORMALIZADO. Em alguns Windows,
        // `ProgramFiles` e `ProgramFiles(x86)` apontam para o mesmo lugar —
        // somar as duas passaria o total do tamanho do disco.
        let chave = caminho.to_string_lossy().to_lowercase();
        let repetida = raizes
            .iter()
            .any(|r| r.to_string_lossy().to_lowercase() == chave);

        if caminho.exists() && !repetida {
            raizes.push(caminho);
        }
    }

    raizes
}

/// O mapa do disco: uma linha por raiz, sem descer nelas.
///
/// DUAS CAMADAS, E ESTA É A PRIMEIRA. Varrer 476 GB a fundo leva minutos —
/// medido: sete, na máquina onde `mapear` foi escrito. Ninguém espera olhando
/// tela parada. Aqui cada raiz vira uma linha; o detalhe só é varrido quando o
/// cliente clica numa delas, chamando `mapear` como já se faz hoje.
pub fn mapear_o_disco(limite: usize) -> Result<FolderMap, String> {
    let mut folders: Vec<FolderEntry> = Vec::new();
    let mut ilegiveis = 0usize;
    let mut estourou = false;

    for raiz in raizes_do_disco() {
        match mapear(&raiz, limite) {
            Ok(parcial) => {
                ilegiveis += parcial.unreadable;
                estourou = estourou || parcial.timed_out;

                let nome = raiz
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| raiz.to_string_lossy().to_string());
                let caminho = raiz.to_string_lossy().to_string();

                folders.push(FolderEntry {
                    name: nome,
                    natureza: classificar(&caminho, true),
                    path: caminho,
                    bytes: parcial.total_bytes,
                    formatted: format_size(parcial.total_bytes),
                    // Preenchido no fim, quando o total de todas for conhecido.
                    percent: 0.0,
                    explanation: explicar(&raiz.to_string_lossy()).to_string(),
                    partial: parcial.timed_out,
                });
            }
            Err(_) => {
                // A RAIZ QUE NÃO DEU PARA LER APARECE MESMO ASSIM, como
                // `NaoSei`. Omiti-la faria o total mentir por baixo, e o
                // cliente veria o espaço sumir no nada.
                let caminho = raiz.to_string_lossy().to_string();
                ilegiveis += 1;

                folders.push(FolderEntry {
                    name: raiz
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| caminho.clone()),
                    natureza: Natureza::NaoSei,
                    path: caminho,
                    bytes: 0,
                    formatted: format_size(0),
                    percent: 0.0,
                    explanation: String::new(),
                    partial: true,
                });
            }
        }
    }

    let total: u64 = folders.iter().map(|f| f.bytes).sum();

    for pasta in &mut folders {
        pasta.percent = if total == 0 {
            0.0
        } else {
            (pasta.bytes as f64 / total as f64) * 100.0
        };
    }

    folders.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    Ok(FolderMap {
        root: "C:\\".to_string(),
        total_bytes: total,
        total_formatted: format_size(total),
        folders,
        unreadable: ilegiveis,
        timed_out: estourou,
    })
}
```

Em `commands.rs:406-407`, trocar a chamada:

```rust
            use crate::modules::windows::foldermap;
            foldermap::mapear_o_disco(12)
```

- [ ] **Passo 4: Rodar e ver passar**

Rodar: `cargo test --lib foldermap`
Esperado: PASSA.

- [ ] **Passo 5: Provar por mutação**

Trocar `raizes_do_disco()` por `vec![perfil_do_usuario()]` dentro de
`mapear_o_disco`, rodar, e confirmar que
`as_raizes_incluem_onde_os_jogos_ficam` REPROVA. Desfazer.

- [ ] **Passo 6: Commitar**

```bash
git add pc-optimizer/src-tauri/src/modules/windows/foldermap.rs \
        pc-optimizer/src-tauri/src/commands.rs
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" \
  commit -m "O mapa passa a enxergar o disco inteiro, e nao so a pasta do usuario"
```

---

### Task 3: As categorias de limpeza que são grandes de verdade

**Arquivos:**
- Modificar: `pc-optimizer/src-tauri/src/modules/windows/diskspace.rs`
- Testar: o módulo `#[cfg(test)]` do próprio arquivo

**Interfaces:**
- Consome: `SpaceFinding`, `DiskReport`, `scan()`, `clean(id: &str)`, que já
  existem.
- Produz: quatro categorias novas com os `id` `winsxs`, `browser_cache`,
  `crash_dumps`, `store_cache`; e
  `pub fn estimativa_do_winsxs(saida_do_dism: &str) -> Option<u64>`.

- [ ] **Passo 1: Escrever o teste que falha**

```rust
#[test]
fn o_winsxs_usa_a_estimativa_do_dism_e_nunca_o_tamanho_da_pasta() {
    // O WINSXS MENTE SOBRE O TAMANHO, e o produto nao pode repetir a mentira.
    //
    // A pasta aparece com 11,5 GB na maquina do dono, mas usa hard links:
    // boa parte daquilo sao os MESMOS arquivos de `C:\Windows`, contados de
    // novo. O que o DISM libera de verdade costuma ser 1 a 5 GB. Mostrar
    // 11,5 GB e prometer o que nao vai acontecer.
    let saida = "\
Versão: 10.0.26100.1

Imagem : C:\\

Tamanho do Repositório de Componentes Reportável ao Windows Explorer : 11.49 GB
Tamanho Real do Repositório de Componentes : 8.21 GB
Recuperável : 2.34 GB
Limpeza do Repositório de Componentes Recomendada : Sim
A operação foi concluída com êxito.";

    let bytes = estimativa_do_winsxs(saida).expect("a linha Recuperavel existe nesta saida");
    // 2,34 GB, com folga de arredondamento.
    assert!(bytes > 2_400_000_000 && bytes < 2_600_000_000, "veio {}", bytes);
}

#[test]
fn winsxs_sem_a_linha_de_recuperavel_vira_nao_sei_e_nao_zero() {
    // "NAO CONSEGUI ESTIMAR" e diferente de "nao ha nada para recuperar". A
    // primeira e honesta; a segunda seria o produto afirmando o que nao mediu.
    assert_eq!(estimativa_do_winsxs("A operação falhou. Erro: 0x800f0954"), None);
    assert_eq!(estimativa_do_winsxs(""), None);
}

#[test]
fn a_estimativa_entende_o_dism_em_ingles_tambem() {
    // O Windows do cliente pode estar em ingles, e o DISM responde no idioma
    // do sistema. Ler so o portugues faria a categoria sumir para esse
    // cliente, sem explicacao.
    let saida = "Reclaimable Packages : 12\nReclaimable : 2.34 GB\n";
    assert!(estimativa_do_winsxs(saida).is_some(), "nao leu a saida em ingles");
}

#[test]
fn toda_categoria_nova_avisa_o_que_se_perde_quando_ha_o_que_perder() {
    // O cache do navegador apagado desloga de nada, mas faz o primeiro
    // carregamento de cada site ficar mais lento uma vez. O cliente precisa
    // saber ANTES de clicar -- e nao descobrir depois achando que quebrou.
    let relatorio = scan();

    for id in ["browser_cache", "store_cache"] {
        if let Some(f) = relatorio.findings.iter().find(|f| f.id == id) {
            assert!(f.warning.is_some(), "{} nao diz o que se perde", id);
        }
    }
}
```

- [ ] **Passo 2: Rodar e ver falhar**

Rodar: `cargo test --lib diskspace`
Esperado: FALHA — `estimativa_do_winsxs` não existe.

- [ ] **Passo 3: Implementar**

```rust
/// Quanto o DISM diz que dá para recuperar do repositório de componentes.
///
/// PURA, PARA SER TESTÁVEL SEM RODAR O DISM. A análise real
/// (`DISM /Online /Cleanup-Image /AnalyzeComponentStore`) leva minutos e exige
/// administrador; a leitura da resposta não precisa de nenhum dos dois.
///
/// `None` quando a linha não veio — e `None` NÃO é zero. "Não consegui
/// estimar" e "não há nada para recuperar" são coisas diferentes, e confundir
/// as duas já foi o defeito deste produto em quatro módulos.
///
/// Lê português e inglês porque o DISM responde no idioma do sistema, e um
/// cliente com Windows em inglês veria a categoria sumir sem explicação.
pub fn estimativa_do_winsxs(saida_do_dism: &str) -> Option<u64> {
    for linha in saida_do_dism.lines() {
        let minuscula = linha.to_lowercase();

        if !(minuscula.contains("recuperável")
            || minuscula.contains("recuperavel")
            || minuscula.contains("reclaimable"))
        {
            continue;
        }

        let (_, depois) = linha.split_once(':')?;
        let bruto = depois.trim();

        // "2.34 GB" — o número e a unidade. Um contador de pacotes
        // ("Reclaimable Packages : 12") não tem unidade e é descartado aqui.
        let (numero, unidade) = bruto.split_once(' ')?;
        let valor: f64 = numero.replace(',', ".").parse().ok()?;

        let multiplicador: u64 = match unidade.trim().to_uppercase().as_str() {
            "KB" => 1024,
            "MB" => 1024 * 1024,
            "GB" => 1024 * 1024 * 1024,
            "TB" => 1024u64 * 1024 * 1024 * 1024,
            _ => continue,
        };

        return Some((valor * multiplicador as f64) as u64);
    }

    None
}
```

Acrescentar as quatro categorias na tabela `Category` que o arquivo já tem,
seguindo exatamente o formato das seis existentes:

| `id` | `name` | onde | `warning` |
|---|---|---|---|
| `winsxs` | "Componentes antigos do Windows" | via DISM | `None` — o Windows não precisa deles |
| `browser_cache` | "Cache dos navegadores" | `Chrome/Edge/Firefox` em `%LOCALAPPDATA%` | "Os sites vão carregar uma vez mais devagar." |
| `crash_dumps` | "Despejos de memória" | `%SystemRoot%\MEMORY.DMP` e `Minidump\` | `None` |
| `store_cache` | "Cache da Microsoft Store" | `wsreset` | "A Store vai demorar um pouco mais para abrir na primeira vez." |

O `winsxs` usa `estimativa_do_winsxs` para o campo `bytes`. Quando ela devolver
`None`, a categoria entra com `cleanable: false` e `explanation` dizendo que
não foi possível estimar — **nunca com o tamanho da pasta**.

- [ ] **Passo 4: Rodar e ver passar**

Rodar: `cargo test --lib diskspace`
Esperado: PASSA.

- [ ] **Passo 5: Provar por mutação**

Fazer `estimativa_do_winsxs` devolver `Some(0)` em vez de `None` no caminho de
falha, rodar, e confirmar que
`winsxs_sem_a_linha_de_recuperavel_vira_nao_sei_e_nao_zero` REPROVA. Desfazer.

- [ ] **Passo 6: Commitar**

```bash
git add pc-optimizer/src-tauri/src/modules/windows/diskspace.rs
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" \
  commit -m "Limpeza: WinSxS, cache de navegador, despejos de memoria e Store"
```

---

### Task 4: Resizable BAR — duas perguntas, não uma

**Arquivos:**
- Criar: `pc-optimizer/src-tauri/src/modules/windows/rbar.rs`
- Modificar: `pc-optimizer/src-tauri/src/modules/windows/mod.rs` (declarar `pub mod rbar;`)
- Modificar: `.github/workflows/release.yml` (acrescentar `modules::windows::rbar::` a um passo)
- Testar: o módulo `#[cfg(test)]` do próprio arquivo

**Interfaces:**
- Consome: `super::shell` para rodar comando.
- Produz:
  - `pub enum EstadoDoRbar { Ligado, DesligadoESuportado, DesligadoSemSuporte, NaoSei }`
  - `pub fn avaliar(bar1_mib: Option<u64>, vram_mib: Option<u64>, modelo: &str) -> EstadoDoRbar`
  - `pub fn suporta_rbar(modelo: &str) -> bool`
  - `pub fn analyze() -> RelatorioDoRbar` com campos `estado`, `modelo`, `nota`

- [ ] **Passo 1: Escrever o teste que falha**

```rust
#[test]
fn placa_sem_suporte_nao_vira_conselho_de_bios() {
    // O DEFEITO QUE A MAQUINA DO DONO REVELOU.
    //
    // Medido la: GTX 1650, 4096 MiB de VRAM, BAR1 de 256 MiB. Desligado. Mas
    // a NVIDIA so habilitou Resizable BAR da RTX 3000 para cima -- a 1650 NAO
    // TEM esse recurso. Um produto que so olhasse "esta desligado" mandaria o
    // cliente entrar na BIOS procurar uma opcao que nao vai adiantar nada.
    //
    // Detectar sem entender e pior do que nao detectar.
    assert_eq!(
        avaliar(Some(256), Some(4096), "NVIDIA GeForce GTX 1650"),
        EstadoDoRbar::DesligadoSemSuporte
    );
}

#[test]
fn placa_com_suporte_e_desligada_vira_conselho() {
    assert_eq!(
        avaliar(Some(256), Some(8192), "NVIDIA GeForce RTX 3060 Ti"),
        EstadoDoRbar::DesligadoESuportado
    );
}

#[test]
fn bar1_do_tamanho_da_vram_e_ligado() {
    // Ligado, o BAR1 passa a cobrir toda a VRAM. A folga de 10% cobre o que o
    // firmware reserva para si.
    assert_eq!(
        avaliar(Some(8192), Some(8192), "NVIDIA GeForce RTX 3060 Ti"),
        EstadoDoRbar::Ligado
    );
    assert_eq!(
        avaliar(Some(7800), Some(8192), "NVIDIA GeForce RTX 3060 Ti"),
        EstadoDoRbar::Ligado
    );
}

#[test]
fn sem_leitura_e_nao_sei_e_nunca_esta_tudo_bem() {
    // A regra do produto. Nao conseguir ler o BAR1 -- driver antigo, placa
    // AMD, `nvidia-smi` ausente -- nao pode virar "esta ligado, nada a fazer".
    assert_eq!(avaliar(None, Some(8192), "NVIDIA GeForce RTX 3060 Ti"), EstadoDoRbar::NaoSei);
    assert_eq!(avaliar(Some(256), None, "NVIDIA GeForce RTX 3060 Ti"), EstadoDoRbar::NaoSei);
    assert_eq!(avaliar(Some(256), Some(8192), ""), EstadoDoRbar::NaoSei);
}

#[test]
fn a_familia_da_placa_decide_o_suporte() {
    assert!(suporta_rbar("NVIDIA GeForce RTX 3060"));
    assert!(suporta_rbar("NVIDIA GeForce RTX 4070 Ti"));
    assert!(suporta_rbar("NVIDIA GeForce RTX 5080"));

    assert!(!suporta_rbar("NVIDIA GeForce GTX 1650"));
    assert!(!suporta_rbar("NVIDIA GeForce GTX 1080 Ti"));
    // RTX 2000 nao recebeu rBAR.
    assert!(!suporta_rbar("NVIDIA GeForce RTX 2060"));
}

#[test]
fn a_nota_nunca_manda_mexer_na_bios_sem_suporte() {
    // CANARIO DE TELA. A frase e o que o cliente le, e mandar alguem procurar
    // na BIOS uma opcao que nao existe para a placa dele e o erro que este
    // modulo existe para nao cometer.
    let nota = nota_de(EstadoDoRbar::DesligadoSemSuporte);
    assert!(!nota.to_lowercase().contains("bios"), "mandou na BIOS a toa: {}", nota);

    let com_suporte = nota_de(EstadoDoRbar::DesligadoESuportado);
    assert!(com_suporte.to_lowercase().contains("bios"), "faltou dizer onde ativar");
}
```

- [ ] **Passo 2: Rodar e ver falhar**

Rodar: `cargo test --lib rbar`
Esperado: FALHA — o módulo não existe.

- [ ] **Passo 3: Implementar**

Criar `rbar.rs` com o cabeçalho explicando o porquê (no tom dos módulos
vizinhos, especialmente `firmware.rs`), e:

```rust
/// Quanto o BAR1 pode ficar abaixo da VRAM e ainda contar como ligado.
///
/// Dez por cento. Com rBAR ligado o BAR1 cobre a VRAM inteira, mas o firmware
/// reserva uma fatia para si e o número não bate na vírgula. Exigir igualdade
/// exata leria "ligado" como "desligado" em placa perfeitamente configurada.
const FOLGA: f64 = 0.90;

/// Se a família da placa tem Resizable BAR.
///
/// A NVIDIA habilitou rBAR a partir da série RTX 3000. GTX de qualquer geração
/// e RTX 2000 não têm — e é por isso que "está desligado" não basta como
/// resposta: numa GTX 1650 não há nada para ligar.
pub fn suporta_rbar(modelo: &str) -> bool {
    let m = modelo.to_uppercase();

    if !m.contains("RTX") {
        return false;
    }

    // O primeiro dígito da série: RTX 3060 -> 3. RTX 2060 -> 2.
    m.split_whitespace()
        .find_map(|palavra| {
            let digitos: String = palavra.chars().filter(|c| c.is_ascii_digit()).collect();
            digitos.chars().next()
        })
        .and_then(|d| d.to_digit(10))
        .map(|serie| serie >= 3)
        .unwrap_or(false)
}

/// Decide o estado. PURA — é o que permite provar os quatro casos sem placa de
/// vídeo nenhuma.
pub fn avaliar(bar1_mib: Option<u64>, vram_mib: Option<u64>, modelo: &str) -> EstadoDoRbar {
    let (Some(bar1), Some(vram)) = (bar1_mib, vram_mib) else {
        return EstadoDoRbar::NaoSei;
    };

    if modelo.trim().is_empty() || vram == 0 {
        return EstadoDoRbar::NaoSei;
    }

    if (bar1 as f64) >= (vram as f64) * FOLGA {
        return EstadoDoRbar::Ligado;
    }

    if suporta_rbar(modelo) {
        EstadoDoRbar::DesligadoESuportado
    } else {
        EstadoDoRbar::DesligadoSemSuporte
    }
}

/// A frase que o cliente lê.
pub fn nota_de(estado: EstadoDoRbar) -> String {
    match estado {
        EstadoDoRbar::Ligado =>
            "O Resizable BAR está ligado. Nada a fazer aqui.".to_string(),
        EstadoDoRbar::DesligadoESuportado =>
            "O Resizable BAR está desligado, e a sua placa aceita. Ele deixa o \
             processador enxergar a memória da placa de vídeo inteira de uma vez, \
             e costuma render alguns quadros. Para ativar, entre na BIOS e procure \
             por \"Resizable BAR\" ou \"Above 4G Decoding\" — não dá para ligar por \
             programa nenhum."
                .to_string(),
        // SEM A PALAVRA "BIOS", DE PROPÓSITO. Mandar procurar uma opção que não
        // existe para esta placa faria o cliente perder tempo e concluir que o
        // produto não sabe do que fala.
        EstadoDoRbar::DesligadoSemSuporte =>
            "Esta placa de vídeo não tem Resizable BAR — a NVIDIA só passou a \
             oferecer a partir da série RTX 3000. Não há nada para ativar, e isso \
             não é problema no seu PC."
                .to_string(),
        EstadoDoRbar::NaoSei =>
            "Não consegui verificar o Resizable BAR nesta máquina. Isso acontece \
             com placa AMD e com driver antigo — e não quer dizer que esteja certo \
             nem errado."
                .to_string(),
    }
}
```

A leitura real usa `nvidia-smi`, verificado na máquina do dono:

```rust
/// Lê o BAR1 e a VRAM pelo `nvidia-smi`.
///
/// O `nvidia-smi` acompanha o driver, então está em toda máquina com placa
/// NVIDIA — mesma escolha do `nvapi64.dll`, e a mesma razão pela qual o
/// relatório em PDF usa o Edge: não acrescentar dependência ao instalador.
///
/// Máquina com placa AMD simplesmente não tem o programa, e isso vira
/// `NaoSei` — que a tela mostra como "ainda não cobrimos sua placa".
///
/// A saída de `nvidia-smi -q` é fixa e em inglês, independente do idioma do
/// Windows — ao contrário do DISM. Por isso aqui basta um padrão.
fn ler_do_nvidia_smi() -> (Option<u64>, Option<u64>, String) {
    // `--query-gpu` devolve CSV enxuto; `-q` devolve o relatório inteiro. São
    // duas chamadas porque o BAR1 só aparece no relatório inteiro, e o modelo
    // e a VRAM saem muito mais limpos do CSV.
    let identidade = shell::run(
        "nvidia-smi",
        &["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"],
    );

    let (modelo, vram) = match identidade {
        Ok(saida) if saida.success => {
            let linha = saida.stdout.lines().next().unwrap_or_default().to_string();
            match linha.split_once(',') {
                Some((nome, mib)) => (
                    nome.trim().to_string(),
                    mib.trim().parse::<u64>().ok(),
                ),
                None => (String::new(), None),
            }
        }
        // SEM `nvidia-smi` NÃO HÁ PLACA NVIDIA VISÍVEL, e isso não é erro: é o
        // caso do cliente com AMD, que a tela trata dizendo que ainda não é
        // coberto.
        _ => return (None, None, String::new()),
    };

    let bar1 = shell::run("nvidia-smi", &["-q"])
        .ok()
        .filter(|s| s.success)
        .and_then(|s| bar1_da_saida(&s.stdout));

    (bar1, vram, modelo)
}

/// Tira o `BAR1 Total` do relatório do `nvidia-smi`. PURA, para ser testável
/// sem placa de vídeo.
///
/// O relatório tem VÁRIAS linhas `Total` — memória da placa, BAR1, memória
/// protegida. Pegar a primeira devolveria a VRAM no lugar do BAR1, e o produto
/// concluiria que o rBAR está sempre ligado. Por isso a leitura só começa
/// DEPOIS de encontrar o cabeçalho `BAR1 Memory Usage`.
pub fn bar1_da_saida(saida: &str) -> Option<u64> {
    let mut dentro = false;

    for linha in saida.lines() {
        let limpa = linha.trim();

        if limpa.starts_with("BAR1 Memory Usage") {
            dentro = true;
            continue;
        }

        if !dentro {
            continue;
        }

        if let Some((chave, valor)) = limpa.split_once(':') {
            if chave.trim() == "Total" {
                return valor
                    .trim()
                    .trim_end_matches(" MiB")
                    .trim()
                    .parse::<u64>()
                    .ok();
            }
        }
    }

    None
}
```

E o teste que prende essa leitura, no mesmo `mod tests`:

```rust
#[test]
fn a_leitura_do_bar1_nao_pega_a_memoria_da_placa_por_engano() {
    // O relatorio do `nvidia-smi` tem VARIAS linhas "Total". Pegar a primeira
    // devolveria a VRAM no lugar do BAR1 -- e o produto concluiria que o
    // Resizable BAR esta SEMPRE ligado, em toda maquina.
    //
    // Este trecho e a saida real da maquina do dono, encurtada.
    let saida = "\
    FB Memory Usage
        Total                             : 4096 MiB
        Used                              : 900 MiB
    BAR1 Memory Usage
        Total                             : 256 MiB
        Used                              : 2 MiB
        Free                              : 254 MiB
    Conf Compute Protected Memory Usage
        Total                             : 0 MiB";

    assert_eq!(bar1_da_saida(saida), Some(256), "leu a linha Total errada");
}

#[test]
fn saida_sem_bar1_e_nao_sei() {
    assert_eq!(bar1_da_saida("FB Memory Usage\n    Total : 4096 MiB"), None);
    assert_eq!(bar1_da_saida(""), None);
}
```

- [ ] **Passo 4: Rodar e ver passar**

Rodar: `cargo test --lib rbar`
Esperado: PASSA.

- [ ] **Passo 5: Conferir contra a máquina de verdade**

Rodar `nvidia-smi -q | Select-String BAR1 -Context 0,2` e comparar com o que
`analyze()` devolve. Na máquina onde este plano foi escrito o esperado é
`DesligadoSemSuporte` (GTX 1650, BAR1 256 MiB, VRAM 4096 MiB).

- [ ] **Passo 6: Registrar na esteira e commitar**

Acrescentar `modules::windows::rbar::` a um passo de
`.github/workflows/release.yml`, senão a guarda `ci_coverage` reprova.

```bash
git add pc-optimizer/src-tauri/src/modules/windows/rbar.rs \
        pc-optimizer/src-tauri/src/modules/windows/mod.rs \
        .github/workflows/release.yml
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" \
  commit -m "Resizable BAR: duas perguntas, esta ligado e a placa suporta"
```

---

### Task 5: A tela do mapa e do Resizable BAR

**Arquivos:**
- Modificar: `pc-optimizer/src/main.ts`
- Modificar: `pc-optimizer/index.html`
- Modificar: `pc-optimizer/src-tauri/src/commands.rs` (comando de rBAR + registro em `lib.rs`)

**Interfaces:**
- Consome: `FolderMap` com `natureza` (Tasks 1 e 2), `RelatorioDoRbar` (Task 4).
- Produz: comando `analyze_rbar` registrado em `lib.rs` e classificado em
  `LIVRES` (é leitura).

- [ ] **Passo 1: Escrever o teste que falha**

Em `commands.rs`, junto das guardas que já existem:

```rust
#[test]
fn a_tela_do_mapa_decide_por_natureza_e_nao_por_texto() {
    // A GUARDA QUE JA PEGOU O MESMO DEFEITO TRES VEZES NESTE PRODUTO.
    //
    // `natureza` e campo tipado justamente para a tela nao precisar
    // interpretar frase. Este teste confere que `main.ts` le o campo.
    let ts = std::fs::read_to_string("../src/main.ts").expect("main.ts");

    assert!(
        ts.contains("natureza.tipo") || ts.contains(r#"natureza"#),
        "a tela do mapa precisa decidir pelo campo `natureza`"
    );
}

#[test]
fn o_comando_do_rbar_esta_registrado() {
    // Comando que existe e nao esta no `generate_handler!` falha em tempo de
    // execucao, na maquina do cliente, e nunca no build.
    let lib = std::fs::read_to_string("src/lib.rs").expect("lib.rs");
    assert!(lib.contains("analyze_rbar"), "analyze_rbar fora do generate_handler!");
}
```

- [ ] **Passo 2: Rodar e ver falhar**

Rodar: `cargo test --lib commands`
Esperado: FALHA — nem o comando nem a tela existem.

- [ ] **Passo 3: Implementar**

Em `commands.rs`:

```rust
/// O Resizable BAR desta máquina. Só leitura — entra em `LIVRES`.
#[tauri::command(async)]
pub fn analyze_rbar() -> Result<crate::modules::windows::rbar::RelatorioDoRbar, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::rbar::analyze())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Só no Windows.".to_string())
    }
}
```

Registrar em `lib.rs`, no `generate_handler!`.

Em `main.ts`, a tela do mapa desenha cada linha decidindo por
`pasta.natureza.tipo`:

```ts
type Natureza =
  | { tipo: "PodeLimpar" }
  | { tipo: "Seu" }
  | { tipo: "NaoSei" };

// A COR E O BOTAO SAEM DO CAMPO, e nunca de comparar a explicacao. Este
// projeto reprova o build quando a tela decide por prosa do backend.
const rotulo = (n: Natureza): string => {
  if (n.tipo === "PodeLimpar") return "dá para limpar";
  if (n.tipo === "NaoSei") return "não consegui ler";
  return "seu arquivo";
};
```

E a linha `Seu` **não recebe botão de limpar** — só o caminho, para o cliente
decidir.

- [ ] **Passo 4: Rodar e ver passar**

Rodar: `cargo test --lib commands && cd .. && npx tsc --noEmit`
Esperado: PASSA nos dois.

- [ ] **Passo 5: Commitar**

```bash
git add pc-optimizer/src/main.ts pc-optimizer/index.html \
        pc-optimizer/src-tauri/src/commands.rs pc-optimizer/src-tauri/src/lib.rs
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" \
  commit -m "A tela do mapa e do Resizable BAR"
```

---

### Task 6: Ajustes de driver NVIDIA pela NVAPI

Última tarefa, e a única que ESCREVE. Depende de todas as anteriores estarem
verdes.

**Arquivos:**
- Criar: `pc-optimizer/src-tauri/src/modules/windows/nvdriver.rs`
- Modificar: `pc-optimizer/src-tauri/src/modules/windows/mod.rs`
- Modificar: `.github/workflows/release.yml`
- Testar: o módulo `#[cfg(test)]` do próprio arquivo

**Interfaces:**
- Consome: o histórico de mudanças (`ChangeRecord` em `windows/mod.rs`) e
  `revert_changes()`, que já existem.
- Produz:
  - `pub enum Nvapi { Disponivel, SemPlacaNvidia, NaoCarregou }`
  - `pub fn estado() -> Nvapi`
  - `pub fn aplicar(opcao: &str) -> Result<String, String>` — devolve o valor
    ANTERIOR, para o histórico
  - `pub fn restaurar_padrao(opcao: &str) -> Result<(), String>`

- [ ] **Passo 1: Escrever o teste que falha**

```rust
#[test]
fn sem_nvapi_o_produto_diz_que_nao_cobre_em_vez_de_tela_vazia() {
    // A RECOMENDACAO LITERAL DO VEREDITO DA INVESTIGACAO ANTERIOR:
    // "entrar com a NVIDIA e DIZER NA TELA que a AMD ainda nao e coberta, em
    // vez de fazer meia coisa nas duas".
    //
    // Cliente com AMD lendo "ainda nao cobrimos sua placa" entende. Vendo tela
    // vazia, acha que o programa quebrou.
    let nota = nota_do_estado(Nvapi::SemPlacaNvidia);

    assert!(nota.to_lowercase().contains("amd") || nota.to_lowercase().contains("não cobrimos"),
        "a tela precisa dizer que a placa nao e coberta: {}", nota);
    assert!(!nota.is_empty());
}

#[test]
fn nao_carregar_a_dll_e_diferente_de_nao_ter_placa_nvidia() {
    // Dois "nao deu" com causas diferentes e conselhos diferentes: um pede
    // atualizar o driver, o outro diz que a placa nao e coberta. Colapsar os
    // dois daria o conselho errado para metade dos casos.
    assert_ne!(nota_do_estado(Nvapi::NaoCarregou), nota_do_estado(Nvapi::SemPlacaNvidia));
    assert!(nota_do_estado(Nvapi::NaoCarregou).to_lowercase().contains("driver"));
}

#[test]
fn toda_opcao_conhecida_tem_como_restaurar_o_padrao() {
    // O QUE DECIDIU ESTE PILAR, segundo o veredito: a NVAPI tem chamada para
    // restaurar o padrao de uma opcao. "Reversivel de verdade, nao reversivel
    // se a gente anotar direitinho."
    //
    // Uma opcao sem restauracao nao pode entrar no catalogo: quebraria a regra
    // que sustenta o produto inteiro.
    for opcao in OPCOES {
        assert!(opcao.id_do_padrao.is_some(),
            "{} nao tem como voltar ao padrao e nao pode entrar", opcao.id);
    }
}
```

- [ ] **Passo 2: Rodar e ver falhar**

Rodar: `cargo test --lib nvdriver`
Esperado: FALHA — o módulo não existe.

- [ ] **Passo 3: Implementar**

Criar `nvdriver.rs` com o cabeçalho citando o veredito, a carga da
`nvapi64.dll` em tempo de execução (`libloading`, ou `LoadLibraryW` direto se o
projeto já usar `windows-sys`), e a tabela `OPCOES` com as cinco: gerenciamento
de energia, low latency mode, filtragem de textura, VSync, cache de shader.

Cada aplicação devolve o valor anterior e grava um `ChangeRecord` novo,
`DriverNvidia { opcao, valor_anterior }`, com o ramo correspondente em
`revert_changes()` chamando `restaurar_padrao`.

- [ ] **Passo 4: Rodar e ver passar**

Rodar: `cargo test --lib` (a suíte inteira) e `npx tsc --noEmit`.
Esperado: PASSA.

- [ ] **Passo 5: Provar o desfazer na máquina de verdade**

Aplicar um ajuste, conferir no Painel de Controle da NVIDIA que mudou, rodar
"Desfazer tudo", e conferir que voltou. **É o teste que diz se o pilar pode
ser vendido** — a promessa do produto é que toda mudança volta.

- [ ] **Passo 6: Registrar na esteira e commitar**

```bash
git add pc-optimizer/src-tauri/src/modules/windows/nvdriver.rs \
        pc-optimizer/src-tauri/src/modules/windows/mod.rs \
        .github/workflows/release.yml
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" \
  commit -m "Ajustes de driver NVIDIA pela NVAPI, com restauracao de padrao"
```

---

## Depois das seis tarefas

1. **Revisão da branch inteira**, com `superpowers:requesting-code-review`, no
   modelo mais capaz. Nas três versões anteriores ela achou defeito que revisão
   de tarefa não vê — inclusive um em que o produto cobrava o dinheiro e
   marcava a compra como vencida.
2. **Bump de versão nos quatro arquivos** (`Cargo.toml`, `tauri.conf.json`,
   `package.json`, `package-lock.json`). A trava
   `as_quatro_declaracoes_de_versao_concordam` reprova se discordarem, e a
   esteira reprova se a tag não bater.
3. **Notas de versão citando o número**, senão a publicação é recusada.
4. **Conferir na máquina do dono**: o mapa mostra Steam com 122 GB, o rBAR diz
   "sua placa não tem", e o desfazer do driver volta.
