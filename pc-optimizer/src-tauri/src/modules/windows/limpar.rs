// Os caminhos e as chamadas da limpeza (o catálogo puro está em `modules::limpeza`). O que não se lê sai
// DESCONHECIDO, nunca zero: zero faria o técnico não tentar como administrador. Apagar nunca para no primeiro
// erro: arquivo em uso é o caso comum.

#![cfg(target_os = "windows")]

use std::path::{Path, PathBuf};

use crate::modules::limpeza::{alvo_por_id, AlvoMedido, ALVOS};

use super::shell;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Resultado {
    pub id: String,
    pub bytes_liberados: u64,
    pub arquivos_apagados: usize,
    /// NÃO é erro: é o caso comum.
    pub arquivos_pulados: usize,
    pub erro: Option<String>,
}

fn variavel(nome: &str) -> Option<PathBuf> {
    std::env::var(nome).ok().map(PathBuf::from)
}

/// Lista vazia = o alvo não existe aqui, diferente de pasta que existe e está vazia.
pub fn pastas_de(id: &str) -> Vec<PathBuf> {
    let windows = variavel("SystemRoot").unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let local = variavel("LOCALAPPDATA");

    match id {
        "temporarios" => [variavel("TEMP"), Some(windows.join("Temp"))]
            .into_iter()
            .flatten()
            .collect(),

        "windows_update" => vec![windows.join("SoftwareDistribution").join("Download")],

        "entregas_otimizadas" => vec![windows
            .join("SoftwareDistribution")
            .join("DeliveryOptimization")],

        // Arquivos soltos numa pasta que tem outras coisas: ver `apagar_miniaturas`.
        "miniaturas" => local
            .map(|l| vec![l.join("Microsoft").join("Windows").join("Explorer")])
            .unwrap_or_default(),

        // `ProgramData` exige administrador; sem ele os arquivos contam como pulados.
        "relatorios_de_erro" => [local, variavel("ProgramData")]
            .into_iter()
            .flatten()
            .flat_map(|raiz| {
                let wer = raiz.join("Microsoft").join("Windows").join("WER");
                [wer.join("ReportQueue"), wer.join("ReportArchive")]
            })
            .collect(),

        // Uma subpasta por usuário em cada disco: medir por varredura, apagar pelo Windows (`esvaziar_lixeira`).
        "lixeira" => discos()
            .into_iter()
            .map(|d| d.join("$Recycle.Bin"))
            .collect(),

        _ => Vec::new(),
    }
}

fn discos() -> Vec<PathBuf> {
    ('C'..='Z')
        .map(|letra| PathBuf::from(format!("{letra}:\\")))
        .filter(|d| d.is_dir())
        .collect()
}

/// `None` quando nem a pasta abriu; arquivo que falha no meio é pulado.
fn tamanho_de(pasta: &Path) -> Option<u64> {
    if !pasta.is_dir() {
        return None;
    }

    let mut total = 0u64;
    let mut pendentes = vec![pasta.to_path_buf()];
    let mut abriu_alguma = false;

    // Iterativo: instalador mal feito deixa milhares de níveis e estouraria a pilha.
    while let Some(atual) = pendentes.pop() {
        let Ok(entradas) = std::fs::read_dir(&atual) else {
            continue;
        };
        abriu_alguma = true;

        for entrada in entradas.flatten() {
            let Ok(tipo) = entrada.file_type() else {
                continue;
            };

            // Link NÃO é seguido: um atalho para `C:\` faria medir e apagar o disco inteiro.
            if tipo.is_symlink() {
                continue;
            }

            if tipo.is_dir() {
                pendentes.push(entrada.path());
            } else if let Ok(meta) = entrada.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }

    abriu_alguma.then_some(total)
}

pub fn medir() -> Vec<AlvoMedido> {
    ALVOS
        .iter()
        .map(|a| {
            let pastas = pastas_de(a.id);

            let medidas: Vec<u64> = pastas.iter().filter_map(|p| tamanho_de(p)).collect();

            AlvoMedido {
                id: a.id.to_string(),
                nome: a.nome.to_string(),
                o_que_e: a.o_que_e.to_string(),
                custo: a.custo.to_string(),
                padrao: a.padrao,
                bytes: (!medidas.is_empty()).then(|| medidas.iter().sum()),
            }
        })
        .collect()
}

fn esvaziar(pasta: &Path) -> (u64, usize, usize) {
    let (mut bytes, mut apagados, mut pulados) = (0u64, 0usize, 0usize);

    let Ok(entradas) = std::fs::read_dir(pasta) else {
        return (0, 0, 0);
    };

    for entrada in entradas.flatten() {
        let caminho = entrada.path();
        let Ok(tipo) = entrada.file_type() else {
            pulados += 1;
            continue;
        };

        // Mesma razão da medição: não seguir link.
        if tipo.is_symlink() {
            pulados += 1;
            continue;
        }

        if tipo.is_dir() {
            let tamanho = tamanho_de(&caminho).unwrap_or(0);
            match std::fs::remove_dir_all(&caminho) {
                Ok(()) => {
                    bytes += tamanho;
                    apagados += 1;
                }
                Err(_) => pulados += 1,
            }
        } else {
            let tamanho = entrada.metadata().map(|m| m.len()).unwrap_or(0);
            match std::fs::remove_file(&caminho) {
                Ok(()) => {
                    bytes += tamanho;
                    apagados += 1;
                }
                Err(_) => pulados += 1,
            }
        }
    }

    (bytes, apagados, pulados)
}

/// Apagar a pasta inteira levaria a configuração de exibição das pastas, que é do cliente.
fn apagar_miniaturas(pasta: &Path) -> (u64, usize, usize) {
    let (mut bytes, mut apagados, mut pulados) = (0u64, 0usize, 0usize);

    let Ok(entradas) = std::fs::read_dir(pasta) else {
        return (0, 0, 0);
    };

    for entrada in entradas.flatten() {
        let nome = entrada.file_name().to_string_lossy().to_lowercase();

        if !(nome.starts_with("thumbcache_") || nome.starts_with("iconcache_")) {
            continue;
        }

        let tamanho = entrada.metadata().map(|m| m.len()).unwrap_or(0);
        match std::fs::remove_file(entrada.path()) {
            Ok(()) => {
                bytes += tamanho;
                apagados += 1;
            }
            Err(_) => pulados += 1,
        }
    }

    (bytes, apagados, pulados)
}

/// Pelo Windows, e não varrendo `$Recycle.Bin`: por fora, a lixeira mostraria item que não existe mais.
fn esvaziar_lixeira() -> Result<(), String> {
    let saida = shell::powershell("Clear-RecycleBin -Force -ErrorAction Stop")?;

    if saida.success {
        Ok(())
    } else {
        let texto = saida.stderr.trim();
        // Lixeira já vazia devolve erro no PowerShell, e não é falha.
        if texto.contains("empty") || texto.contains("vazi") {
            Ok(())
        } else {
            Err(texto.to_string())
        }
    }
}

pub fn apagar(id: &str) -> Resultado {
    let mut resultado = Resultado {
        id: id.to_string(),
        bytes_liberados: 0,
        arquivos_apagados: 0,
        arquivos_pulados: 0,
        erro: None,
    };

    if alvo_por_id(id).is_none() {
        resultado.erro = Some("alvo fora do catálogo".to_string());
        return resultado;
    }

    if id == "lixeira" {
        let antes: u64 = pastas_de(id).iter().filter_map(|p| tamanho_de(p)).sum();

        match esvaziar_lixeira() {
            Ok(()) => resultado.bytes_liberados = antes,
            Err(e) => resultado.erro = Some(e),
        }

        return resultado;
    }

    // Com atualização em andamento, apagar o cache a deixa pela metade: os serviços donos param antes e voltam
    // depois (só os que estavam rodando).
    let servicos: &[&str] = match id {
        "windows_update" | "entregas_otimizadas" => &["wuauserv", "bits", "dosvc"],
        _ => &[],
    };
    let parados = parar_servicos(servicos);

    for pasta in pastas_de(id) {
        let (bytes, apagados, pulados) = if id == "miniaturas" {
            apagar_miniaturas(&pasta)
        } else {
            esvaziar(&pasta)
        };

        resultado.bytes_liberados += bytes;
        resultado.arquivos_apagados += apagados;
        resultado.arquivos_pulados += pulados;
    }

    religar_servicos(&parados);
    resultado
}

/// `None` na leitura é "não sei", e a dúvida pende para parar.
pub fn parar_servicos(servicos: &[&str]) -> Vec<String> {
    let mut parados = Vec::new();
    for s in servicos {
        if super::services::is_running(s) != Some(false) {
            let _ = super::services::stop(s);
            parados.push(s.to_string());
        }
    }
    parados
}

pub fn religar_servicos(parados: &[String]) {
    for s in parados {
        let _ = super::services::start(s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_alvo_do_catalogo_tem_caminho_ou_nao_existe_aqui() {
        for a in ALVOS {
            let pastas = pastas_de(a.id);
            println!("{}: {} pasta(s)", a.id, pastas.len());

            assert!(
                !pastas.is_empty() || a.id == "entregas_otimizadas",
                "{} não sabe onde mora",
                a.id
            );
        }
    }

    #[test]
    fn a_medicao_desta_maquina_responde_por_todos() {
        let medidos = medir();
        assert_eq!(medidos.len(), ALVOS.len());

        for m in &medidos {
            println!(
                "  {:22} {}",
                m.id,
                m.bytes
                    .map(|b| format!("{} MB", b / (1024 * 1024)))
                    .unwrap_or_else(|| "não deu para ler".to_string())
            );
        }

        for m in &medidos {
            if let Some(b) = m.bytes {
                assert!(
                    b < 1024_u64.pow(4),
                    "{}: {} bytes é leitura errada",
                    m.id,
                    b
                );
            }
        }
    }

    #[test]
    fn id_fora_do_catalogo_nao_tem_caminho() {
        assert!(pastas_de("prefetch").is_empty());
        assert!(pastas_de("inventado").is_empty());
    }

    #[test]
    fn apagar_o_que_nao_existe_nao_faz_nada() {
        let r = apagar("prefetch");

        assert_eq!(r.bytes_liberados, 0);
        assert_eq!(r.arquivos_apagados, 0);
        assert!(r.erro.is_some(), "tem de recusar, e dizer que recusou");
    }

    #[test]
    fn medir_nao_apaga_nada() {
        let pasta = std::env::temp_dir().join("otimiza-limpeza-medir");
        let _ = std::fs::create_dir_all(&pasta);
        let arquivo = pasta.join("teste.tmp");
        std::fs::write(&arquivo, vec![0u8; 4096]).expect("escrever");

        let medido = tamanho_de(&pasta).expect("tamanho");
        assert!(medido >= 4096, "{medido}");
        assert!(arquivo.is_file(), "medir não pode apagar");

        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn pasta_que_nao_existe_sai_como_desconhecida_e_nao_como_zero() {
        let fora = std::env::temp_dir().join("otimiza-limpeza-que-nao-existe");
        let _ = std::fs::remove_dir_all(&fora);

        assert_eq!(tamanho_de(&fora), None, "ausente não é zero");
    }

    #[test]
    fn esvaziar_apaga_o_conteudo_e_mantem_a_pasta() {
        let pasta = std::env::temp_dir().join("otimiza-limpeza-esvaziar");
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(pasta.join("dentro")).expect("criar");
        std::fs::write(pasta.join("a.tmp"), vec![0u8; 1024]).expect("escrever");
        std::fs::write(pasta.join("dentro").join("b.tmp"), vec![0u8; 2048]).expect("escrever");

        let (bytes, apagados, _) = esvaziar(&pasta);

        assert!(bytes >= 3072, "{bytes}");
        assert_eq!(apagados, 2, "o arquivo e a subpasta");
        assert!(pasta.is_dir(), "a pasta em si não pode sumir");
        assert_eq!(std::fs::read_dir(&pasta).unwrap().count(), 0);

        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn as_miniaturas_nao_levam_junto_o_resto_da_pasta() {
        let pasta = std::env::temp_dir().join("otimiza-limpeza-miniaturas");
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("criar");
        std::fs::write(pasta.join("thumbcache_96.db"), vec![0u8; 512]).expect("escrever");
        std::fs::write(pasta.join("iconcache_16.db"), vec![0u8; 256]).expect("escrever");
        std::fs::write(pasta.join("Settings.dat"), vec![0u8; 128]).expect("escrever");

        let (bytes, apagados, _) = apagar_miniaturas(&pasta);

        assert_eq!(apagados, 2);
        assert_eq!(bytes, 768);
        assert!(
            pasta.join("Settings.dat").is_file(),
            "a configuração do cliente não pode sair junto"
        );

        let _ = std::fs::remove_dir_all(&pasta);
    }
}
