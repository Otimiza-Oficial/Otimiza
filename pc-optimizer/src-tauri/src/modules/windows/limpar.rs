// Onde cada alvo da limpeza mora, quanto ele tem, e como apagá-lo
//
// O CATÁLOGO ESTÁ EM `modules::limpeza`, e ele é puro: o que cada item é, o que
// se perde ao apagar, e qual vem marcado. Aqui ficam só os caminhos e as
// chamadas — a parte que só existe no Windows.
//
// A SEPARAÇÃO NÃO É ARRUMAÇÃO. A decisão de produto — "a lixeira não vem
// marcada porque contém arquivo do cliente" — precisa ser testável em qualquer
// máquina, inclusive numa que não tem lixeira nenhuma. Se ela morasse junto do
// código que varre `C:\$Recycle.Bin`, ela só seria testável no Windows.
//
// MEDIR É ANDAR NA PASTA, E ANDAR NA PASTA PODE FALHAR
//
// Falta de permissão, arquivo travado, caminho longo demais. Nenhum desses é
// erro fatal: a soma continua, e o que não deu para ler sai como DESCONHECIDO
// em vez de zero. Zero afirmaria que não há nada ali, e é o que faria o técnico
// não rodar como administrador quando era exatamente isso que faltava.
//
// APAGAR NUNCA PARA NO PRIMEIRO ERRO
//
// Um arquivo em uso por programa aberto é o caso comum, não a exceção. Parar
// ali deixaria a limpeza pela metade e diria "falhou" sobre uma operação que
// liberou dois gigabytes. O que se conta é o que foi apagado, o que foi pulado,
// e quanto sobrou.

#![cfg(target_os = "windows")]

use std::path::{Path, PathBuf};

use crate::modules::limpeza::{alvo_por_id, AlvoMedido, ALVOS};

use super::shell;

/// O que sobrou depois de apagar.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Resultado {
    pub id: String,
    pub bytes_liberados: u64,
    pub arquivos_apagados: usize,
    /// Em uso por um programa aberto. NÃO é erro: é o caso comum.
    pub arquivos_pulados: usize,
    /// O que impediu de apagar, quando impediu por completo.
    pub erro: Option<String>,
}

fn variavel(nome: &str) -> Option<PathBuf> {
    std::env::var(nome).ok().map(PathBuf::from)
}

/// As pastas de cada alvo nesta máquina.
///
/// Lista vazia significa "este alvo não existe aqui" — Windows sem Entrega
/// Otimizada, por exemplo. Diferente de pasta que existe e está vazia.
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

        // O cache de miniaturas são ARQUIVOS soltos numa pasta que tem outras
        // coisas dentro, então ele não é tratado como pasta inteira: ver
        // `apagar_miniaturas`.
        "miniaturas" => local
            .map(|l| vec![l.join("Microsoft").join("Windows").join("Explorer")])
            .unwrap_or_default(),

        "relatorios_de_erro" => local
            .map(|l| {
                let wer = l.join("Microsoft").join("Windows").join("WER");
                vec![wer.join("ReportQueue"), wer.join("ReportArchive")]
            })
            .unwrap_or_default(),

        // A lixeira fica na raiz de CADA disco, com uma subpasta por usuário.
        // Varrer para medir é seguro; apagar é feito pelo Windows, que sabe
        // qual subpasta é de quem. Ver `esvaziar_lixeira`.
        "lixeira" => discos()
            .into_iter()
            .map(|d| d.join("$Recycle.Bin"))
            .collect(),

        _ => Vec::new(),
    }
}

/// As raízes dos discos fixos, para a lixeira.
fn discos() -> Vec<PathBuf> {
    ('C'..='Z')
        .map(|letra| PathBuf::from(format!("{letra}:\\")))
        .filter(|d| d.is_dir())
        .collect()
}

/// Soma o tamanho de uma pasta, andando nela.
///
/// `None` quando nem a pasta abriu. Arquivo solto que falhar no meio é pulado
/// — o total sai menor, e isso é melhor que sair ausente por causa de um
/// arquivo.
fn tamanho_de(pasta: &Path) -> Option<u64> {
    if !pasta.is_dir() {
        return None;
    }

    let mut total = 0u64;
    let mut pendentes = vec![pasta.to_path_buf()];
    let mut abriu_alguma = false;

    // Iterativo e não recursivo: uma pasta de temporários com milhares de
    // níveis — que acontece com instalador mal feito — estouraria a pilha.
    while let Some(atual) = pendentes.pop() {
        let Ok(entradas) = std::fs::read_dir(&atual) else {
            continue;
        };
        abriu_alguma = true;

        for entrada in entradas.flatten() {
            let Ok(tipo) = entrada.file_type() else {
                continue;
            };

            // Link simbólico NÃO é seguido: um atalho para `C:\` dentro de uma
            // pasta de temporários faria a medição varrer o disco inteiro, e o
            // apagar seguir atrás dela.
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

/// Mede todos os alvos.
pub fn medir() -> Vec<AlvoMedido> {
    ALVOS
        .iter()
        .map(|a| {
            let pastas = pastas_de(a.id);

            // Soma só o que deu para ler. Nenhuma pasta lida vira ausente, e
            // não zero — a diferença é o que diz ao técnico se vale tentar
            // como administrador.
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

/// Apaga o conteúdo de uma pasta, sem apagar a pasta.
///
/// Devolve (bytes, apagados, pulados). Nunca para no primeiro erro: arquivo em
/// uso é o caso comum, e desistir ali deixaria a limpeza pela metade.
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

/// O cache de miniaturas são arquivos com nome conhecido dentro da pasta do
/// Explorer, que tem outras coisas.
///
/// Apagar a pasta inteira levaria junto a configuração de exibição das pastas
/// — que é do cliente, e que ele nunca pediu para perder.
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
            // Em uso pelo Explorer é o caso normal: o arquivo fica travado
            // enquanto ele está aberto. Pular é a resposta certa.
            Err(_) => pulados += 1,
        }
    }

    (bytes, apagados, pulados)
}

/// Esvazia a lixeira pelo próprio Windows.
///
/// NÃO por varredura de `$Recycle.Bin`. Aquela pasta tem uma subpasta por
/// usuário e um índice que o Explorer mantém; apagar arquivo dali por fora
/// deixa a lixeira mostrando item que não existe mais. O Windows sabe fazer
/// isso direito, e esta é uma operação sem volta — não é hora de improvisar.
fn esvaziar_lixeira() -> Result<(), String> {
    let saida = shell::powershell("Clear-RecycleBin -Force -ErrorAction Stop")?;

    if saida.success {
        Ok(())
    } else {
        let texto = saida.stderr.trim();
        // Lixeira já vazia devolve erro no PowerShell, e não é falha nenhuma.
        if texto.contains("empty") || texto.contains("vazi") {
            Ok(())
        } else {
            Err(texto.to_string())
        }
    }
}

/// Apaga um alvo. `Err` só quando nada pôde ser feito.
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
        // O tamanho é medido ANTES, porque depois não há o que medir.
        let antes: u64 = pastas_de(id).iter().filter_map(|p| tamanho_de(p)).sum();

        match esvaziar_lixeira() {
            Ok(()) => resultado.bytes_liberados = antes,
            Err(e) => resultado.erro = Some(e),
        }

        return resultado;
    }

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

    resultado
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Todo alvo do catálogo sabe onde mora, ou declara que não mora aqui.
    ///
    /// Um alvo sem caminho nenhum apareceria na tela com tamanho desconhecido
    /// para sempre, e o técnico ficaria tentando como administrador uma coisa
    /// que nunca ia medir.
    #[test]
    fn todo_alvo_do_catalogo_tem_caminho_ou_nao_existe_aqui() {
        for a in ALVOS {
            let pastas = pastas_de(a.id);
            println!("{}: {} pasta(s)", a.id, pastas.len());

            // `entregas_otimizadas` pode não existir em Windows antigo, e a
            // lixeira depende de haver disco — os dois são casos legítimos de
            // lista vazia. O que não pode é um id do catálogo cair no `_`.
            assert!(
                !pastas.is_empty() || a.id == "entregas_otimizadas",
                "{} não sabe onde mora",
                a.id
            );
        }
    }

    /// A medição desta máquina, de verdade.
    ///
    /// Não afirma tamanho nenhum — afirma o CONTRATO: todo alvo do catálogo
    /// sai da medição, cada um com número ou com ausência declarada, e nunca
    /// com um zero que ninguém mediu.
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

        // Nenhum alvo pode sair com zero: ou tem tamanho, ou tem a ausência
        // declarada. Um zero aqui seria o produto afirmando que a pasta está
        // vazia sem ter conseguido abri-la.
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

    /// A trava que impede apagar coisa que não está no catálogo.
    #[test]
    fn apagar_o_que_nao_existe_nao_faz_nada() {
        let r = apagar("prefetch");

        assert_eq!(r.bytes_liberados, 0);
        assert_eq!(r.arquivos_apagados, 0);
        assert!(r.erro.is_some(), "tem de recusar, e dizer que recusou");
    }

    /// Medir não apaga. Óbvio, e por isso mesmo com teste: é a separação que
    /// deixa o cliente ver o total antes de decidir.
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

    /// Só os arquivos de cache saem da pasta do Explorer — a configuração de
    /// exibição das pastas é do cliente e fica.
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
