// Mapa de pastas: "meu disco tem 500 GB, cadê o espaço?". Não apaga nada: mostra do maior para o menor, com o
// caminho. Desvia de junction e link simbólico (contaria duas vezes ou entraria em laço), de pasta sem permissão
// (normal) e do tempo (prazo, avisando quando parou nele).

use serde::{Deserialize, Serialize};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

/// Trava contra estrutura patológica; quem manda na prática é o tempo. Baixa demais marcaria quase tudo como
/// "parcial" (`node_modules` passa de vinte níveis).
const PROFUNDIDADE_MAXIMA: u32 = 40;

/// Sem limite, um perfil real levou sete minutos.
const ORCAMENTO_SEGUNDOS: u64 = 45;

/// Dividido entre as pastas: com prazo único as primeiras em ordem alfabética gastavam tudo, e `.cache` (1,4 GB)
/// saía maior que `AppData` (151 GB, nem aberta). A sobra de quem termina passa para as seguintes.
fn fatia(restante: std::time::Duration, pastas_faltando: usize) -> std::time::Duration {
    restante / pastas_faltando.max(1) as u32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderEntry {
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub formatted: String,
    pub percent: f64,
    pub explanation: String,
    /// Parou na profundidade, no prazo ou em permissão negada num descendente: o número é um piso.
    pub partial: bool,
    pub natureza: Natureza,
}

/// Tipado, não frase (a guarda de `commands.rs`). Quatro estados: "não li" contado como zero faria o total mentir,
/// e "limpável" é diferente de "limpável AQUI".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tipo")]
pub enum Natureza {
    /// A ÚNICA que ganha botão: é a única em que o botão leva a algo que age.
    PodeLimpar,
    /// Sobra de sistema que este produto NÃO limpa (`Windows.old`, `servicing\LogFiles`). Como `PodeLimpar`, o mapa
    /// prometia "Limpar no liberador" e a outra tela não tinha botão, ou nem a pasta.
    SoOWindowsLimpa,
    Seu,
    /// Falha no próprio nível 1 (não num descendente, ver `nivel1_foi_lido`, nem no prazo, que é `partial`).
    NaoSei,
}

/// O destino junto do prefixo: o rótulo do mapa é uma promessa sobre a outra tela.
struct Destino {
    /// Caminho no Windows não diferencia maiúsculas: sem normalizar, `C:\WINDOWS\TEMP` passaria como do cliente.
    prefixo: &'static str,
    /// `Some(id)`: categoria do `diskspace.rs` que limpa isto (conferida por
    /// `todo_prefixo_com_botao_tem_categoria_que_limpa_de_verdade`). `None`: sobra de sistema que não limpamos.
    categoria: Option<&'static str>,
}

const LIMPAVEIS: &[Destino] = &[
    Destino {
        prefixo: r"\windows\temp",
        categoria: Some("temp"),
    },
    Destino {
        prefixo: r"\appdata\local\temp",
        categoria: Some("temp"),
    },
    Destino {
        prefixo: r"\windows\softwaredistribution\download",
        categoria: Some("update_cache"),
    },
    Destino {
        prefixo: r"\programdata\microsoft\windows\wer",
        categoria: Some("error_reports"),
    },
    // `update_logs` limpa `Windows\Logs\CBS`, que fica dentro desta pasta.
    Destino {
        prefixo: r"\windows\logs",
        categoria: Some("update_logs"),
    },
    // Existe no liberador com `cleanable: false` de propósito (TrustedInstaller).
    Destino {
        prefixo: r"\windows.old",
        categoria: None,
    },
    // Sem categoria nenhuma: era o beco sem saída.
    Destino {
        prefixo: r"\windows\servicing\logfiles",
        categoria: None,
    },
];

/// Por COMPONENTE inteiro: `contains` casava `\windows.old` dentro de `downloads\windows.old-backup`, e apagar
/// arquivo do cliente não tem desfazer.
fn contem_como_componente(caminho: &str, padrao: &str) -> bool {
    let componentes_caminho: Vec<&str> = caminho.split('\\').filter(|c| !c.is_empty()).collect();
    let componentes_padrao: Vec<&str> = padrao.split('\\').filter(|c| !c.is_empty()).collect();

    if componentes_padrao.is_empty() || componentes_padrao.len() > componentes_caminho.len() {
        return false;
    }

    componentes_caminho
        .windows(componentes_padrao.len())
        .any(|janela| janela == componentes_padrao.as_slice())
}

/// Pura. O padrão é o seguro: o que não for reconhecido como limpável é do cliente.
pub fn classificar(caminho: &str, leu: bool) -> Natureza {
    if !leu {
        return Natureza::NaoSei;
    }

    let minusculo = caminho.to_lowercase().replace('/', "\\");

    // O estado sai do destino: o rótulo vira botão.
    if let Some(destino) = LIMPAVEIS
        .iter()
        .find(|d| contem_como_componente(&minusculo, d.prefixo))
    {
        return match destino.categoria {
            Some(_) => Natureza::PodeLimpar,
            None => Natureza::SoOWindowsLimpa,
        };
    }

    Natureza::Seu
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderMap {
    pub root: String,
    /// Soma das raízes varridas, não o tamanho do disco: a tela precisa dizer o que a varredura cobriu.
    pub total_bytes: u64,
    pub total_formatted: String,
    pub folders: Vec<FolderEntry>,
    pub unreadable: usize,
    /// Só o RELÓGIO: `partial` também liga por profundidade e por descendente sem permissão. Antes, qualquer permissão
    /// negada fazia a tela dizer "não terminou dentro do tempo" (descendente ilegível tem `unreadable`).
    pub timed_out: bool,
}

pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;

    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.0} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// `symlink_metadata`, não `metadata`: o segundo iria ao destino e não veria a marca.
pub fn e_link(atributos: u32) -> bool {
    atributos & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

struct Varredura {
    ilegiveis: usize,
    prazo: std::time::Instant,
    estourou: bool,
}

impl Varredura {
    fn sem_tempo(&mut self) -> bool {
        if self.estourou {
            return true;
        }

        // Por pasta: consultar o relógio a cada arquivo custaria mais que ler o arquivo.
        if std::time::Instant::now() >= self.prazo {
            self.estourou = true;
        }

        self.estourou
    }
}

fn somar(dir: &Path, profundidade: u32, v: &mut Varredura) -> (u64, bool) {
    if profundidade >= PROFUNDIDADE_MAXIMA || v.sem_tempo() {
        return (0, true);
    }

    let Ok(entradas) = std::fs::read_dir(dir) else {
        // `cortado = true` também aqui: o total do ancestral virou piso, e `partial` é o único canal que avisa.
        v.ilegiveis += 1;
        return (0, true);
    };

    let mut total = 0u64;
    let mut cortado = false;

    for entrada in entradas.flatten() {
        // `entrada.metadata()` reaproveita o que a leitura da pasta trouxe: uma chamada de sistema a menos por arquivo.
        // Nenhum dos dois segue link.
        let Ok(meta) = entrada.metadata() else {
            continue;
        };

        if e_link(meta.file_attributes()) {
            continue;
        }

        if meta.is_dir() {
            let (bytes, parcial) = somar(&entrada.path(), profundidade + 1, v);
            total += bytes;
            cortado |= parcial;
        } else {
            total += meta.file_size();
        }
    }

    (total, cortado)
}

/// Só o NÍVEL 1, sem a `Varredura`: `v.ilegiveis` conta a subárvore inteira, e uma pasta de 150 GB lida a 99%
/// recebia o mesmo `NaoSei` que uma de zero bytes lidos. O `read_dir` a mais custa dezenas de chamadas, não milhões.
fn nivel1_foi_lido(caminho: &Path) -> bool {
    std::fs::read_dir(caminho).is_ok()
}

/// "AppData" é a maior pasta da maioria dos perfis e ninguém sabe o que tem lá.
pub fn explicar(nome: &str) -> &'static str {
    match nome.to_lowercase().as_str() {
        "appdata" => {
            "Dados dos programas instalados: cache de navegador, e-mail baixado, \
             projetos de editor. É quase sempre a maior pasta do perfil, e quase \
             nada aí dentro pode ser apagado na mão sem quebrar programa."
        }
        "downloads" => {
            "Downloads. Costuma ser o ganho mais fácil e mais seguro: instalador \
             velho, ISO, zip já extraído. Confira antes de apagar."
        }
        "documents" | "documentos" => "Seus documentos. Não apague sem olhar.",
        "desktop" | "área de trabalho" => "Arquivos da Área de Trabalho.",
        "pictures" | "imagens" => "Fotos e imagens.",
        "videos" | "vídeos" => {
            "Vídeos. Junto com jogos, é o que mais come disco sem ninguém perceber."
        }
        "music" | "músicas" => "Músicas.",
        "onedrive" => {
            "Pasta do OneDrive. Arquivo marcado como \"sempre disponível\" ocupa \
             espaço aqui mesmo estando na nuvem — dá para liberar pelo próprio \
             OneDrive sem perder nada."
        }
        "saved games" => "Jogos salvos.",
        "steamlibrary" | "steam" => {
            "Biblioteca da Steam. Jogo instalado e não jogado há anos costuma ser \
             a maior economia possível — e reinstalar depois é só baixar de novo."
        }
        _ => "",
    }
}

pub fn mapear(raiz: &Path, limite: usize) -> Result<FolderMap, String> {
    if !raiz.is_dir() {
        return Err(format!("`{}` não é uma pasta acessível.", raiz.display()));
    }

    let entradas =
        std::fs::read_dir(raiz).map_err(|e| format!("Não foi possível ler `{}`: {}", raiz.display(), e))?;

    // A lista vem antes: é ela que divide o prazo.
    let mut filhos: Vec<PathBuf> = Vec::new();
    let mut total = 0u64;

    for entrada in entradas.flatten() {
        let caminho = entrada.path();

        let Ok(meta) = caminho.symlink_metadata() else {
            continue;
        };

        if e_link(meta.file_attributes()) {
            continue;
        }

        if meta.is_dir() {
            filhos.push(caminho);
        } else {
            total += meta.file_size();
        }
    }

    let fim = std::time::Instant::now() + std::time::Duration::from_secs(ORCAMENTO_SEGUNDOS);
    let mut ilegiveis = 0usize;
    // SÓ o relógio: os cortes por profundidade e por `read_dir` voltam antes de `sem_tempo` e nunca tocam `estourou`.
    let mut algum_prazo_estourou = false;
    let mut pastas: Vec<FolderEntry> = Vec::new();

    for (indice, caminho) in filhos.iter().enumerate() {
        let restante = fim.saturating_duration_since(std::time::Instant::now());
        let minha_fatia = fatia(restante, filhos.len() - indice);

        let mut v = Varredura {
            ilegiveis: 0,
            prazo: std::time::Instant::now() + minha_fatia,
            estourou: false,
        };

        let (bytes, partial) = somar(caminho, 1, &mut v);
        let leu_com_sucesso = nivel1_foi_lido(caminho);

        total += bytes;
        ilegiveis += v.ilegiveis;
        algum_prazo_estourou |= v.estourou;

        let name = caminho
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let caminho_texto = caminho.to_string_lossy().to_string();

        pastas.push(FolderEntry {
            explanation: explicar(&name).to_string(),
            formatted: format_size(bytes),
            natureza: classificar(&caminho_texto, leu_com_sucesso),
            path: caminho_texto,
            percent: 0.0,
            name,
            bytes,
            partial,
        });
    }

    // Cortada antes de medida: uma é piso, a outra total (`Videos` 3,4 GB saía acima de `AppData` com 151 GB).
    // Quem não terminou é o candidato mais provável, e a tela diz "pelo menos".
    pastas.sort_by(|a, b| {
        b.partial
            .cmp(&a.partial)
            .then_with(|| b.bytes.cmp(&a.bytes))
    });
    pastas.truncate(limite);

    if total > 0 {
        for pasta in &mut pastas {
            pasta.percent = pasta.bytes as f64 / total as f64 * 100.0;
        }
    }

    Ok(FolderMap {
        root: raiz.to_string_lossy().to_string(),
        total_bytes: total,
        total_formatted: format_size(total),
        folders: pastas,
        unreadable: ilegiveis,
        timed_out: algum_prazo_estourou,
    })
}

pub fn perfil_do_usuario() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Users"))
}

/// Lista, não `C:\` (`System Volume Information` e lixeiras alheias só dão erro). Medido: AppData 176 GB, Steam
/// 122 GB, Downloads 48 GB, Windows 29 GB. Só entra o que existe.
pub fn raizes_do_disco() -> Vec<PathBuf> {
    let candidatos = [
        std::env::var("ProgramFiles").ok(),
        std::env::var("ProgramFiles(x86)").ok(),
        std::env::var("ProgramData").ok(),
        std::env::var("SystemRoot").ok(),
        Some(perfil_do_usuario().to_string_lossy().to_string()),
    ];

    let existentes: Vec<PathBuf> = candidatos
        .into_iter()
        .flatten()
        .map(PathBuf::from)
        .filter(|caminho| caminho.exists())
        .collect();

    deduplicar_por_caminho_normalizado(existentes)
}

/// Pura, para testar o mecanismo e não o acaso desta máquina. Em alguns Windows `ProgramFiles` e
/// `ProgramFiles(x86)` são o mesmo lugar; fica a PRIMEIRA, na ordem de peso.
fn deduplicar_por_caminho_normalizado(caminhos: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut vistos: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut resultado = Vec::new();

    for caminho in caminhos {
        let chave = caminho.to_string_lossy().to_lowercase();

        if vistos.insert(chave) {
            resultado.push(caminho);
        }
    }

    resultado
}

/// Primeira camada: uma linha por raiz (varrer 476 GB a fundo leva minutos); o detalhe é `mapear` ao clicar.
pub fn mapear_o_disco(limite: usize) -> Result<FolderMap, String> {
    mapear_o_disco_com_raizes(raizes_do_disco(), limite)
}

/// Separada para testar o ramo `Err(_)`: raiz ilegível vira `NaoSei`, não some. `raizes_do_disco()` só devolve o
/// que existe e nunca exercita esse ramo.
fn mapear_o_disco_com_raizes(raizes: Vec<PathBuf>, limite: usize) -> Result<FolderMap, String> {
    let mut folders: Vec<FolderEntry> = Vec::new();
    let mut ilegiveis = 0usize;
    let mut estourou = false;

    for raiz in raizes {
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
                    natureza: classificar(&caminho, true),
                    path: caminho,
                    bytes: parcial.total_bytes,
                    formatted: format_size(parcial.total_bytes),
                    percent: 0.0,
                    // `nome`, não o caminho: `explicar` casa por igualdade com chaves curtas ("steam"), e o caminho inteiro saía sem
                    // explicação.
                    explanation: explicar(&nome).to_string(),
                    partial: parcial.timed_out,
                    name: nome,
                });
            }
            Err(_) => {
                // Omitir a raiz ilegível faria o total mentir por baixo.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Sem `#[serde(tag = "tipo")]`, `linha.natureza.tipo` vira `undefined` na tela, com `cargo test` e `tsc` passando.
    /// O mesmo vale para o `Medida` do `diskspace.rs`.
    #[test]
    fn a_natureza_chega_na_tela_como_objeto_com_campo_tipo() {
        for (natureza, esperado) in [
            (Natureza::PodeLimpar, r#"{"tipo":"PodeLimpar"}"#),
            (
                Natureza::SoOWindowsLimpa,
                r#"{"tipo":"SoOWindowsLimpa"}"#,
            ),
            (Natureza::Seu, r#"{"tipo":"Seu"}"#),
            (Natureza::NaoSei, r#"{"tipo":"NaoSei"}"#),
        ] {
            assert_eq!(
                serde_json::to_string(&natureza).expect("Natureza serializa"),
                esperado,
                "{:?} não chega à tela com `tipo`",
                natureza
            );
        }
    }

    #[test]
    fn tamanho_sai_na_unidade_certa() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(2048), "2 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5 MB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn atributo_de_link_e_reconhecido() {
        const DIRETORIO: u32 = 0x10;

        assert!(e_link(FILE_ATTRIBUTE_REPARSE_POINT));
        assert!(e_link(DIRETORIO | FILE_ATTRIBUTE_REPARSE_POINT));
        assert!(!e_link(DIRETORIO));
        assert!(!e_link(0));
    }

    #[test]
    fn pastas_conhecidas_sao_explicadas_em_portugues() {
        assert!(explicar("AppData").contains("cache"));
        assert!(explicar("AppData").contains("quebrar programa"));

        assert!(explicar("Downloads").contains("instalador"));
        assert_eq!(explicar("downloads"), explicar("DOWNLOADS"));

        assert_eq!(explicar("Projetos do Cliente"), "");
    }

    #[test]
    fn varredura_respeita_o_prazo() {
        let inicio = std::time::Instant::now();
        let _ = mapear(&perfil_do_usuario(), 10).expect("o perfil precisa ser legível");
        let gasto = inicio.elapsed().as_secs();

        println!("varredura levou {} s", gasto);
        assert!(
            gasto <= ORCAMENTO_SEGUNDOS + 10,
            "a varredura levou {} s, muito além do prazo de {} s",
            gasto,
            ORCAMENTO_SEGUNDOS
        );
    }

    #[test]
    fn mapeia_o_perfil_desta_maquina() {
        let mapa = mapear(&perfil_do_usuario(), 10).expect("o perfil precisa ser legível");

        println!("{} — total {}{}", mapa.root, mapa.total_formatted,
                 if mapa.timed_out { " (parou no prazo)" } else { "" });
        println!("{} pastas sem permissão de leitura", mapa.unreadable);
        for f in &mapa.folders {
            println!(
                "  {:>9}  {:>5.1}%  {}{}",
                f.formatted,
                f.percent,
                f.name,
                if f.partial { " (parcial)" } else { "" }
            );
        }

        assert!(!mapa.folders.is_empty(), "todo perfil tem subpastas");
        assert!(mapa.folders.len() <= 10, "o limite precisa ser respeitado");

        let chave = |f: &FolderEntry| (!f.partial, std::cmp::Reverse(f.bytes));
        assert!(
            mapa.folders.windows(2).all(|p| chave(&p[0]) <= chave(&p[1])),
            "ordem quebrada"
        );

        // Os dois sintomas de link contado duas vezes.
        let soma: u64 = mapa.folders.iter().map(|f| f.bytes).sum();
        assert!(
            soma <= mapa.total_bytes,
            "as pastas somam {} num total de {} — link contado em dobro",
            soma,
            mapa.total_bytes
        );

        for f in &mapa.folders {
            assert!(f.percent >= 0.0 && f.percent <= 100.0, "{}%", f.percent);
        }
    }

    #[test]
    fn caminho_que_nao_e_pasta_e_recusado() {
        let erro = mapear(Path::new("C:\\Windows\\explorer.exe"), 5);
        assert!(erro.is_err());

        let inexistente = mapear(Path::new("C:\\pasta que nao existe 12345"), 5);
        assert!(inexistente.is_err());
    }

    #[test]
    fn pasta_que_nao_deu_para_ler_nunca_e_zero_nem_limpavel() {
        assert_eq!(classificar(r"C:\Windows\System32\config", false), Natureza::NaoSei);
    }

    #[test]
    fn arquivo_do_cliente_nunca_e_marcado_como_limpavel() {
        // O PIOR ERRO POSSÍVEL: Steam, Downloads e Documentos são do cliente, e apagar não tem desfazer.
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
        assert_eq!(classificar(r"C:\Windows\Temp", true), Natureza::PodeLimpar);
        assert_eq!(
            classificar(r"C:\Windows\SoftwareDistribution\Download", true),
            Natureza::PodeLimpar
        );
    }

    /// Só `PodeLimpar` ganha o botão: o `id` precisa existir no `diskspace.rs` com `cleanable: true`, senão o cliente
    /// chega numa categoria sem botão. O defeito volta como build vermelho, não como reclamação.
    #[test]
    fn todo_prefixo_com_botao_tem_categoria_que_limpa_de_verdade() {
        let limpa_de_verdade = super::super::diskspace::ids_que_o_liberador_limpa();

        for destino in LIMPAVEIS {
            let Some(id) = destino.categoria else {
                continue;
            };

            assert!(
                limpa_de_verdade.contains(&id),
                "o mapa oferece o botão para `{}` apontando para a categoria `{}`, \
                 mas o liberador não limpa essa categoria (ela não existe lá, ou está \
                 com `cleanable: false`). Ou o destino muda, ou o prefixo passa a ser \
                 `categoria: None` e a linha perde o botão.",
                destino.prefixo,
                id
            );
        }
    }

    #[test]
    fn o_que_o_produto_nao_limpa_nao_promete_botao() {
        assert_eq!(
            classificar(r"C:\Windows.old", true),
            Natureza::SoOWindowsLimpa,
            "Windows.old voltou a prometer o botão que o liberador não tem"
        );

        assert_eq!(
            classificar(r"C:\Windows\servicing\LogFiles", true),
            Natureza::SoOWindowsLimpa,
            "servicing\\LogFiles voltou a prometer um destino que não existe"
        );

        // Chamar de `Seu` esconderia 24 GB atrás de "seu arquivo".
        for caminho in [r"C:\Windows.old", r"C:\Windows\servicing\LogFiles"] {
            assert_ne!(
                classificar(caminho, true),
                Natureza::Seu,
                "{} não é arquivo do cliente",
                caminho
            );
        }
    }

    #[test]
    fn na_duvida_e_do_cliente_e_nao_limpavel() {
        // CANÁRIO: pasta desconhecida não pode cair em `PodeLimpar` pela ordem dos `if`.
        assert_eq!(classificar(r"C:\MinhaPastaEstranha", true), Natureza::Seu);
    }

    #[test]
    fn casamento_e_por_componente_e_nao_por_substring() {
        assert_eq!(
            classificar(r"C:\Windows.old", true),
            Natureza::SoOWindowsLimpa
        );
        assert_eq!(
            classificar(r"C:\Users\Fulano\Downloads\windows.old-backup", true),
            Natureza::Seu,
            "windows.old-backup nao e o mesmo componente que windows.old"
        );
        assert_eq!(
            classificar(r"C:\Users\Fulano\Documents\meus-windows-logs", true),
            Natureza::Seu,
            "meus-windows-logs nao pode casar com o padrao \\windows\\logs"
        );
    }

    #[test]
    fn read_dir_que_falha_em_qualquer_profundidade_vira_parcial() {
        // Caminho inexistente entra pelo mesmo ramo de `read_dir` que falha, sem depender de ACL.
        let mut v = Varredura {
            ilegiveis: 0,
            prazo: std::time::Instant::now() + std::time::Duration::from_secs(30),
            estourou: false,
        };
        let caminho_inexistente =
            std::env::temp_dir().join("otimiza_teste_caminho_que_nao_existe_12345");
        let _ = std::fs::remove_dir_all(&caminho_inexistente);

        let (bytes, cortado) = somar(&caminho_inexistente, 2, &mut v);

        assert_eq!(bytes, 0);
        assert!(cortado, "read_dir que falha precisa marcar `partial`, não desaparecer");
        assert_eq!(v.ilegiveis, 1);
    }

    #[test]
    fn nivel1_legivel_nao_depende_de_descendente_ilegivel() {
        // `nivel1_foi_lido` nem recebe a `Varredura`: não pode reagir a descendente ilegível.
        let pasta = std::env::temp_dir().join("otimiza_teste_nivel1_legivel");
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("criar pasta de teste");

        let v = Varredura {
            ilegiveis: 3,
            prazo: std::time::Instant::now() + std::time::Duration::from_secs(30),
            estourou: false,
        };

        let leu = nivel1_foi_lido(&pasta);
        assert!(
            leu,
            "read_dir do próprio nível 1 funcionou; a ilegibilidade é só dos descendentes"
        );
        assert_ne!(
            classificar(&pasta.to_string_lossy(), leu),
            Natureza::NaoSei,
            "descendente ilegível não pode reclassificar a pasta inteira como NaoSei"
        );

        drop(v);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn nivel1_ilegivel_e_reconhecido_como_tal() {
        // O outro lado: um `true` incondicional passaria por todos os outros testes.
        let inexistente =
            std::env::temp_dir().join("otimiza_teste_nivel1_que_nao_existe_98765");
        let _ = std::fs::remove_dir_all(&inexistente);

        assert!(!nivel1_foi_lido(&inexistente));
    }

    // O teste por ACL real (`icacls /deny`) foi removido: com `SeBackupPrivilege` (e o runner do GitHub é
    // administrativo) ele se autopulava em toda parte. Fica o canário de fiação: `mapear` chama
    // `nivel1_foi_lido(caminho)` e não voltou a `v.ilegiveis == 0`.
    #[test]
    fn mapear_liga_leu_com_sucesso_a_nivel1_foi_lido_e_nao_a_ilegiveis() {
        let fonte = codigo_fonte_deste_arquivo();

        // A mutação exata do bug original.
        assert!(
            !fonte.contains("let leu_com_sucesso = v.ilegiveis"),
            "mapear voltou a decidir `leu_com_sucesso` por `v.ilegiveis`, que soma \
             permissão negada em QUALQUER profundidade — exatamente o bug que \
             `nivel1_foi_lido` existe para evitar"
        );

        // A fiação certa precisa estar presente, não só a errada ausente.
        assert!(
            fonte.contains("let leu_com_sucesso = nivel1_foi_lido(caminho);"),
            "mapear precisa decidir `leu_com_sucesso` chamando `nivel1_foi_lido(caminho)`"
        );
    }

    /// Só fora de `mod tests`: os canários citam os padrões proibidos em string, e se achariam. `CARGO_MANIFEST_DIR`
    /// aponta sempre para `src-tauri`.
    fn codigo_fonte_deste_arquivo() -> String {
        let caminho = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("modules")
            .join("windows")
            .join("foldermap.rs");
        let fonte = std::fs::read_to_string(&caminho)
            .unwrap_or_else(|e| panic!("não consegui ler {:?}: {}", caminho, e));
        fonte
            .split_once("#[cfg(test)]")
            .map(|(codigo, _testes)| codigo.to_string())
            .unwrap_or(fonte)
    }

    #[test]
    fn permissao_negada_no_descendente_nao_liga_o_relogio() {
        // O ramo de `read_dir` que falha marca `partial` sem chamar `sem_tempo`: `estourou` continua `false`.
        let mut v = Varredura {
            ilegiveis: 0,
            prazo: std::time::Instant::now() + std::time::Duration::from_secs(30),
            estourou: false,
        };
        let caminho_inexistente = std::env::temp_dir()
            .join("otimiza_teste_permissao_nao_e_relogio_12345");
        let _ = std::fs::remove_dir_all(&caminho_inexistente);

        let (_, cortado) = somar(&caminho_inexistente, 2, &mut v);

        assert!(cortado, "read_dir que falha precisa marcar partial");
        assert!(
            !v.estourou,
            "descendente sem permissão não pode ligar o relógio — só o prazo de verdade liga isso"
        );
    }

    #[test]
    fn varredura_dentro_do_prazo_com_corte_por_profundidade_nao_e_timed_out() {
        // De ponta a ponta e determinístico: corte por profundidade, bem dentro do prazo, não pode dar `timed_out`.
        let raiz = std::env::temp_dir().join("otimiza_teste_timed_out_nao_e_profundidade");
        let _ = std::fs::remove_dir_all(&raiz);
        let mut fundo = raiz.join("pasta_funda");
        for i in 0..(PROFUNDIDADE_MAXIMA as usize + 10) {
            fundo = fundo.join(format!("nivel{}", i));
        }
        std::fs::create_dir_all(&fundo).expect("criar árvore funda de teste");

        let inicio = std::time::Instant::now();
        let mapa = mapear(&raiz, 5).expect("a raiz do teste é legível");
        let gasto = inicio.elapsed().as_secs();

        let _ = std::fs::remove_dir_all(&raiz);

        assert!(
            gasto < ORCAMENTO_SEGUNDOS,
            "o teste levou {} s — não serve para provar 'terminou dentro do prazo'",
            gasto
        );

        let entrada = mapa
            .folders
            .iter()
            .find(|f| f.name == "pasta_funda")
            .expect("pasta_funda precisa aparecer no mapa");
        assert!(
            entrada.partial,
            "a árvore é mais funda que PROFUNDIDADE_MAXIMA; a pasta tem que ficar parcial"
        );

        assert!(
            !mapa.timed_out,
            "corte por profundidade não é corte por prazo — timed_out mentiu de novo"
        );
    }

    #[test]
    fn mapear_liga_timed_out_a_estourou_e_nao_a_partial() {
        // `timed_out` vem de `v.estourou`, não de `algum_cortado |= partial`.
        let fonte = codigo_fonte_deste_arquivo();

        assert!(
            !fonte.contains("algum_cortado |= partial"),
            "mapear voltou a agregar `partial` (profundidade + prazo + permissão, os \
             três misturados) para decidir timed_out — é exatamente a diluição que \
             este achado corrigiu"
        );
        assert!(
            !fonte.contains("timed_out: algum_cortado"),
            "timed_out voltou a vir de `algum_cortado`, que inclui corte por permissão"
        );
        assert!(
            fonte.contains("algum_prazo_estourou |= v.estourou"),
            "timed_out precisa vir só de `Varredura::estourou`, o único sinal que é \
             de fato o relógio"
        );
    }

    #[test]
    fn pasta_vazia_nao_gera_divisao_por_zero() {
        let temporaria = std::env::temp_dir().join("otimiza_teste_mapa_vazio");
        std::fs::create_dir_all(&temporaria).expect("criar pasta de teste");

        let mapa = mapear(&temporaria, 5).expect("pasta vazia é legível");

        assert_eq!(mapa.total_bytes, 0);
        assert!(mapa.folders.is_empty());

        let _ = std::fs::remove_dir(&temporaria);
    }

    #[test]
    fn as_raizes_incluem_onde_os_jogos_ficam() {
        // O mapa só varria o perfil e era cego aos 122 GB de Steam em `Program Files (x86)`.
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
        // `Program Files` é prefixo de `Program Files (x86)`: um `contains` somaria a pasta duas vezes.
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
    fn deduplicar_por_caminho_normalizado_remove_a_repetida() {
        // Nesta máquina as duas já são strings diferentes; só a função pura, com candidatos que diferem em maiúsculas,
        // expõe o mecanismo.
        let candidatos = vec![
            PathBuf::from(r"C:\Program Files"),
            PathBuf::from(r"c:\program files"),
            PathBuf::from(r"C:\Program Files (x86)"),
        ];

        let resultado = deduplicar_por_caminho_normalizado(candidatos);

        assert_eq!(resultado.len(), 2, "esperava 2 raizes unicas, veio {:?}", resultado);
        assert_eq!(resultado[0], PathBuf::from(r"C:\Program Files"),
            "a PRIMEIRA ocorrencia deve sobreviver, e nao a ultima");
    }

    #[test]
    fn raiz_ilegivel_aparece_como_naosei_e_nao_e_pulada() {
        // Raiz inexistente aciona o `Err(_)` de `mapear()` sem ACL.
        let raiz_inexistente = std::env::temp_dir().join("otimiza_raiz_que_nao_existe_de_verdade_xyz789");
        assert!(!raiz_inexistente.exists(), "a raiz de teste precisa MESMO nao existir");

        let mapa = mapear_o_disco_com_raizes(vec![raiz_inexistente.clone()], 12)
            .expect("raiz ilegivel nao pode derrubar o mapa do disco inteiro");

        assert_eq!(mapa.folders.len(), 1, "a raiz ilegivel sumiu do mapa em vez de virar NaoSei");
        assert_eq!(mapa.folders[0].natureza, Natureza::NaoSei);
        assert_eq!(mapa.folders[0].bytes, 0);
        assert!(mapa.folders[0].partial);
        assert_eq!(mapa.unreadable, 1, "raiz ilegivel precisa contar em unreadable");
    }

    #[test]
    fn explicar_recebe_o_nome_e_nao_o_caminho_inteiro() {
        // O último componente do caminho precisa ser "steam": com o caminho inteiro a explicação saía vazia.
        let raiz_steam = std::env::temp_dir().join("otimiza_teste_explicar").join("steam");
        std::fs::create_dir_all(&raiz_steam).expect("criar pasta de teste");

        let mapa = mapear_o_disco_com_raizes(vec![raiz_steam.clone()], 12)
            .expect("pasta de teste e legivel");

        assert_eq!(mapa.folders.len(), 1);
        assert!(
            !mapa.folders[0].explanation.is_empty(),
            "explicacao saiu vazia -- explicar() recebeu o caminho inteiro, e nao o nome"
        );

        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("otimiza_teste_explicar"));
    }

    #[test]
    fn o_mapa_do_disco_roda_nesta_maquina() {
        // Medição de verdade: pode voltar vazio, não pode quebrar, e os totais precisam ser coerentes.
        let mapa = mapear_o_disco(12).expect("o mapa do disco nao pode falhar");

        for pasta in &mapa.folders {
            assert!(pasta.percent >= 0.0 && pasta.percent <= 100.0,
                "{} tem porcentagem impossivel: {}", pasta.name, pasta.percent);
        }

        if let Some(maior) = mapa.folders.iter().map(|f| f.bytes).max() {
            assert!(mapa.total_bytes >= maior, "o total e menor que a maior pasta");
        }
    }
}
