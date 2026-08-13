// Que jogos existem nesta máquina
//
// O produto precisa saber isto por três motivos, em ordem de importância:
//
// 1. SEGURANÇA. A escrita em IFEO — a chave do registro que fixa prioridade de
//    processo — é o mesmo mecanismo usado para sequestrar a execução de um
//    programa. Até agora a trava era uma lista de nomes de jogo escrita à mão.
//    Com a detecção genérica, a trava passa a ser: o executável precisa estar
//    DENTRO de uma biblioteca de jogo de verdade. Este módulo é quem sabe onde
//    essas bibliotecas ficam.
//
// 2. Limpar o cache do jogo certo, medir o FPS do jogo certo.
//
// 3. Abrir dizendo o nome do que a pessoa joga, em vez de pedir que ela digite.
//
// A DIFERENÇA ENTRE "INSTALADO" E "JÁ JOGOU" — e por que ela importa
//
// `HKCU\SOFTWARE\Microsoft\DirectX\UserGpuPreferences` parece a fonte perfeita:
// o próprio Windows lista os jogos com caminho completo. Só que ele NUNCA
// limpa essa lista.
//
// Na máquina onde este módulo foi escrito, a chave lista o Fortnite num caminho
// dentro de `C:\Program Files\Epic Games` — e nem o Fortnite nem a Epic existem
// mais ali. Tratar aquilo como "jogo instalado" faria o produto afirmar que o
// cliente tem um jogo que ele desinstalou, e afirmação errada é o defeito que
// este produto existe para não ter.
//
// Então: a lista do Windows entra como HISTÓRICO, e cada caminho é conferido no
// disco antes de virar afirmação.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// De onde a informação veio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origem {
    Steam,
    Epic,
    /// A lista de preferência de GPU do próprio Windows, com o arquivo
    /// confirmado no disco.
    Windows,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JogoInstalado {
    pub nome: String,
    pub origem: Origem,
    /// Pasta onde o jogo está. É o que a trava do IFEO confere.
    pub pasta: PathBuf,
    /// Executável, quando a loja informa. A Steam não informa; a Epic sim.
    pub executavel: Option<PathBuf>,
    /// Quando foi jogado pela última vez, em segundos desde 1970. Zero quando
    /// a loja não guarda essa informação.
    pub ultima_vez: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Biblioteca {
    pub jogos: Vec<JogoInstalado>,
    /// Raízes de biblioteca encontradas. É esta lista que autoriza a escrita
    /// em IFEO — um executável fora de todas elas não é jogo instalado.
    pub raizes: Vec<PathBuf>,
    /// O que não deu para ler, dito em voz alta.
    pub lacunas: Vec<String>,
}

// ------------------------------------------------------------------ leitura

/// Extrai o valor de uma chave num arquivo no formato da Valve.
///
/// O formato é `"chave"<tab>"valor"`, um par por linha, com blocos entre
/// chaves. Não vale trazer uma biblioteca de VDF para o instalador do produto
/// por causa de duas dúzias de linhas — e um analisador completo teria mais
/// superfície de erro do que este, que só olha pares na mesma linha.
pub fn valor_vdf(conteudo: &str, chave: &str) -> Option<String> {
    pares_vdf(conteudo)
        .into_iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(chave))
        .map(|(_, v)| v)
}

/// Todos os pares `"chave" "valor"` do arquivo, na ordem em que aparecem.
pub fn pares_vdf(conteudo: &str) -> Vec<(String, String)> {
    let mut pares = Vec::new();

    for linha in conteudo.lines() {
        let mut partes = linha.split('"').skip(1).step_by(2);

        let (Some(chave), Some(valor)) = (partes.next(), partes.next()) else {
            continue;
        };

        // Terceiro campo na mesma linha significa que a linha não é um par
        // simples, e interpretá-la assim traria lixo.
        if partes.next().is_some() {
            continue;
        }

        pares.push((chave.to_string(), valor.to_string()));
    }

    pares
}

/// As pastas de biblioteca declaradas no `libraryfolders.vdf`.
///
/// A Steam permite instalar jogo em qualquer disco, e é comum o jogo pesado
/// estar num HD separado. Ler só a pasta da Steam perderia justamente esse.
pub fn raizes_steam(libraryfolders: &str) -> Vec<PathBuf> {
    pares_vdf(libraryfolders)
        .into_iter()
        .filter(|(chave, _)| chave.eq_ignore_ascii_case("path"))
        // No VDF a barra invertida vem escapada.
        .map(|(_, valor)| PathBuf::from(valor.replace("\\\\", "\\")))
        .collect()
}

/// Lê um `appmanifest_*.acf`.
pub fn jogo_do_manifest(conteudo: &str, raiz: &Path) -> Option<JogoInstalado> {
    let nome = valor_vdf(conteudo, "name")?;
    let pasta_relativa = valor_vdf(conteudo, "installdir")?;

    if nome.trim().is_empty() || pasta_relativa.trim().is_empty() {
        return None;
    }

    let numero = |chave: &str| {
        valor_vdf(conteudo, chave)
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(0)
    };

    Some(JogoInstalado {
        nome,
        origem: Origem::Steam,
        pasta: raiz.join("steamapps").join("common").join(pasta_relativa),
        // A Steam não guarda qual é o executável. Quem quiser o binário precisa
        // varrer a pasta — e isso é caro, então fica para quem realmente
        // precisar, não para a varredura.
        executavel: None,
        ultima_vez: numero("LastPlayed"),
        bytes: numero("SizeOnDisk"),
    })
}

#[cfg(target_os = "windows")]
fn pasta_da_steam() -> Option<PathBuf> {
    use crate::modules::changelog::PreviousValue;

    let PreviousValue::Text(caminho) =
        super::registry::read("HKCU", r"SOFTWARE\Valve\Steam", "SteamPath").ok()?
    else {
        return None;
    };

    // O valor vem com barras normais e em minúsculas — o Windows aceita, mas
    // fica feio na tela e atrapalha comparação de caminho.
    Some(PathBuf::from(caminho.replace('/', "\\")))
}

#[cfg(not(target_os = "windows"))]
fn pasta_da_steam() -> Option<PathBuf> {
    None
}

fn ler_steam(biblioteca: &mut Biblioteca) {
    let Some(steam) = pasta_da_steam() else {
        return;
    };

    let arquivo = steam.join("steamapps").join("libraryfolders.vdf");

    let Ok(conteudo) = std::fs::read_to_string(&arquivo) else {
        biblioteca.lacunas.push(
            "A Steam está instalada, mas a lista de bibliotecas dela não pôde ser lida."
                .to_string(),
        );
        return;
    };

    for raiz in raizes_steam(&conteudo) {
        if !raiz.exists() {
            // Disco removido, ou biblioteca em pendrive que não está plugado.
            // Não é erro: é uma raiz que não vale hoje.
            continue;
        }

        biblioteca.raizes.push(raiz.clone());

        let Ok(entradas) = std::fs::read_dir(raiz.join("steamapps")) else {
            continue;
        };

        for entrada in entradas.flatten() {
            let caminho = entrada.path();
            let nome_arquivo = caminho.file_name().unwrap_or_default().to_string_lossy();

            if !nome_arquivo.starts_with("appmanifest_") || !nome_arquivo.ends_with(".acf") {
                continue;
            }

            let Ok(conteudo) = std::fs::read_to_string(&caminho) else {
                continue;
            };

            if let Some(jogo) = jogo_do_manifest(&conteudo, &raiz) {
                // A Steam instala pacotes de bibliotecas de sistema como se
                // fossem jogos. Eles têm pasta, tamanho e manifest — e não são
                // jogo nenhum.
                if e_pacote_de_sistema(&jogo.nome) {
                    continue;
                }

                if jogo.pasta.exists() {
                    biblioteca.jogos.push(jogo);
                }
            }
        }
    }
}

/// Pacotes que a Steam instala como se fossem jogos.
fn e_pacote_de_sistema(nome: &str) -> bool {
    let minusculo = nome.to_lowercase();

    minusculo.contains("redistributable")
        || minusculo.contains("steamworks")
        || minusculo.starts_with("proton")
        || minusculo.contains("steam linux runtime")
}

fn ler_epic(biblioteca: &mut Biblioteca) {
    let Ok(dados) = std::env::var("PROGRAMDATA") else {
        return;
    };

    let pasta = PathBuf::from(dados)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");

    let Ok(entradas) = std::fs::read_dir(&pasta) else {
        // A Epic não estar instalada é o caso comum, e não é lacuna nenhuma.
        return;
    };

    for entrada in entradas.flatten() {
        let caminho = entrada.path();

        if caminho.extension().and_then(|e| e.to_str()) != Some("item") {
            continue;
        }

        let Ok(conteudo) = std::fs::read_to_string(&caminho) else {
            continue;
        };

        // Ao contrário da Steam, a Epic guarda o executável — e em JSON, que o
        // projeto já sabe ler.
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&conteudo) else {
            continue;
        };

        let (Some(nome), Some(local)) = (
            json.get("DisplayName").and_then(|v| v.as_str()),
            json.get("InstallLocation").and_then(|v| v.as_str()),
        ) else {
            continue;
        };

        let pasta_do_jogo = PathBuf::from(local);

        if !pasta_do_jogo.exists() {
            continue;
        }

        let executavel = json
            .get("LaunchExecutable")
            .and_then(|v| v.as_str())
            .map(|relativo| pasta_do_jogo.join(relativo.replace('/', "\\")))
            .filter(|caminho| caminho.exists());

        biblioteca.raizes.push(pasta_do_jogo.clone());
        biblioteca.jogos.push(JogoInstalado {
            nome: nome.to_string(),
            origem: Origem::Epic,
            pasta: pasta_do_jogo,
            executavel,
            ultima_vez: 0,
            bytes: 0,
        });
    }
}

/// Jogos que o próprio Windows registrou, conferidos no disco.
///
/// Esta é a fonte que exige mais cuidado. Ver a explicação no topo do arquivo:
/// o Windows nunca limpa a lista, então ela guarda jogo desinstalado há anos.
#[cfg(target_os = "windows")]
fn ler_windows(biblioteca: &mut Biblioteca) {
    const CHAVE: &str = r"SOFTWARE\Microsoft\DirectX\UserGpuPreferences";

    for caminho_texto in super::registry::value_names("HKCU", CHAVE) {
        // O nome do valor é o caminho completo do executável. Só entra se o
        // arquivo ainda existir: sem esta conferência o produto afirmaria que
        // o cliente tem um jogo que ele apagou.
        let executavel = PathBuf::from(&caminho_texto);

        if !executavel.exists() {
            continue;
        }

        let Some(pasta) = executavel.parent().map(PathBuf::from) else {
            continue;
        };

        // Se a Steam ou a Epic já contaram este jogo, a informação delas é
        // melhor: tem nome oficial e tamanho.
        if biblioteca
            .jogos
            .iter()
            .any(|j| executavel.starts_with(&j.pasta))
        {
            continue;
        }

        let nome = executavel
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "programa desconhecido".to_string());

        biblioteca.jogos.push(JogoInstalado {
            nome,
            origem: Origem::Windows,
            pasta,
            executavel: Some(executavel),
            ultima_vez: 0,
            bytes: 0,
        });
    }
}

#[cfg(not(target_os = "windows"))]
fn ler_windows(_biblioteca: &mut Biblioteca) {}

/// Varre as lojas e devolve o que existe de verdade no disco.
pub fn varrer() -> Biblioteca {
    let mut biblioteca = Biblioteca::default();

    ler_steam(&mut biblioteca);
    ler_epic(&mut biblioteca);
    ler_windows(&mut biblioteca);

    // O mais jogado primeiro; o que a loja não datou vai para o fim.
    biblioteca.jogos.sort_by(|a, b| b.ultima_vez.cmp(&a.ultima_vez));
    biblioteca.raizes.sort();
    biblioteca.raizes.dedup();

    biblioteca
}

/// Este executável está dentro de uma biblioteca de jogo?
///
/// **É a trava de segurança da escrita em IFEO.** Até a versão 0.13 quem
/// autorizava aquela escrita era uma lista de nomes de jogo escrita à mão; com
/// a detecção genérica essa lista deixa de servir, e passa a valer o caminho.
///
/// Função pura de propósito: a decisão de segurança do produto não pode
/// depender de ter Steam instalada na máquina de quem roda os testes.
pub fn dentro_de_biblioteca(executavel: &Path, raizes: &[PathBuf]) -> bool {
    // Caminho relativo, ou com `..`, não é caminho de jogo instalado — é
    // tentativa de escapar da pasta.
    if !executavel.is_absolute()
        || executavel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return false;
    }

    raizes.iter().any(|raiz| executavel.starts_with(raiz))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIBRARYFOLDERS: &str = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
		"apps"
		{
			"271590"		"129049488815"
		}
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}
"#;

    const MANIFEST: &str = r#"
"AppState"
{
	"appid"		"271590"
	"name"		"Grand Theft Auto V Legacy"
	"StateFlags"		"4"
	"installdir"		"Grand Theft Auto V"
	"LastPlayed"		"1785616013"
	"SizeOnDisk"		"129049488815"
}
"#;

    #[test]
    fn le_as_bibliotecas_de_todos_os_discos() {
        // O jogo pesado costuma estar num disco separado. Ler só a pasta da
        // Steam perderia justamente esse.
        let raizes = raizes_steam(LIBRARYFOLDERS);

        assert_eq!(raizes.len(), 2);
        assert_eq!(raizes[0], PathBuf::from(r"C:\Program Files (x86)\Steam"));
        assert_eq!(raizes[1], PathBuf::from(r"D:\SteamLibrary"));
    }

    #[test]
    fn le_o_manifest_com_nome_data_e_tamanho() {
        let raiz = PathBuf::from(r"C:\Program Files (x86)\Steam");
        let jogo = jogo_do_manifest(MANIFEST, &raiz).expect("manifest válido");

        assert_eq!(jogo.nome, "Grand Theft Auto V Legacy");
        assert_eq!(
            jogo.pasta,
            raiz.join("steamapps").join("common").join("Grand Theft Auto V")
        );
        assert_eq!(jogo.ultima_vez, 1785616013);
        assert_eq!(jogo.bytes, 129049488815);
    }

    #[test]
    fn manifest_sem_nome_ou_pasta_e_recusado() {
        assert!(jogo_do_manifest("\"AppState\"\n{\n}\n", Path::new("C:\\")).is_none());
        assert!(jogo_do_manifest("\"name\" \"\"\n\"installdir\" \"x\"", Path::new("C:\\")).is_none());
    }

    #[test]
    fn pacote_de_sistema_nao_e_jogo() {
        // A Steam instala isto como se fosse jogo, com pasta e tamanho.
        assert!(e_pacote_de_sistema("Steamworks Common Redistributables"));
        assert!(e_pacote_de_sistema("Proton 9.0"));
        assert!(!e_pacote_de_sistema("Grand Theft Auto V Legacy"));
    }

    #[test]
    fn a_trava_do_ifeo_so_aceita_caminho_dentro_da_biblioteca() {
        // Esta é a decisão de segurança mais importante deste arquivo: é ela
        // que substitui a lista de nomes como autorização para escrever numa
        // chave do registro usada por programas que sequestram execução.
        let raizes = vec![
            PathBuf::from(r"C:\Program Files (x86)\Steam"),
            PathBuf::from(r"D:\SteamLibrary"),
        ];

        assert!(dentro_de_biblioteca(
            Path::new(r"D:\SteamLibrary\steamapps\common\Jogo\jogo.exe"),
            &raizes
        ));

        // Fora de qualquer biblioteca.
        assert!(!dentro_de_biblioteca(
            Path::new(r"C:\Windows\System32\sethc.exe"),
            &raizes
        ));
        assert!(!dentro_de_biblioteca(
            Path::new(r"C:\Users\Cliente\Downloads\coisa.exe"),
            &raizes
        ));

        // Tentativa de escapar da pasta, e caminho relativo.
        assert!(!dentro_de_biblioteca(
            Path::new(r"D:\SteamLibrary\..\..\Windows\System32\cmd.exe"),
            &raizes
        ));
        assert!(!dentro_de_biblioteca(Path::new(r"jogo.exe"), &raizes));

        // Sem biblioteca nenhuma, nada é autorizado.
        assert!(!dentro_de_biblioteca(
            Path::new(r"D:\SteamLibrary\steamapps\common\Jogo\jogo.exe"),
            &[]
        ));
    }

    #[test]
    fn linha_com_tres_campos_nao_vira_par() {
        // Robustez do analisador: linha estranha não pode virar dado.
        let pares = pares_vdf("\"a\" \"b\" \"c\"\n\"d\" \"e\"");

        assert_eq!(pares.len(), 1);
        assert_eq!(pares[0], ("d".to_string(), "e".to_string()));
    }

    #[test]
    fn varre_esta_maquina() {
        let b = varrer();

        println!("  raízes: {:?}", b.raizes);
        for j in &b.jogos {
            println!(
                "  {:?} · {} · {:.1} GB · {}",
                j.origem,
                j.nome,
                j.bytes as f64 / 1_073_741_824.0,
                j.pasta.display()
            );
        }
        for l in &b.lacunas {
            println!("  não deu para ler: {}", l);
        }

        // Nada a exigir: uma máquina pode não ter loja nenhuma. O que dá para
        // exigir é que tudo que foi relatado exista de verdade no disco — a
        // regra que impede o produto de afirmar que o cliente tem um jogo que
        // ele desinstalou.
        for j in &b.jogos {
            assert!(j.pasta.exists(), "{} relatado e não existe", j.pasta.display());
        }
    }
}
