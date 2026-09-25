// Em que disco o JOGO mora (não o Windows): com SSD pequeno e HD grande, o Windows fica no SSD e o jogo no HD.
// Nasceu de um cliente com 15 a 20 FPS no FiveM, "depende de onde eu fico". Só lê: mover é decisão do dono.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Três estados: chamar de SSD um disco ilegível absolveria a causa procurada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Midia {
    Ssd,
    Mecanico,
    NaoDeuParaLer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OndeMora {
    pub jogo: String,
    pub caminho: String,
    /// `None` fora de letra de unidade (caminho de rede, por exemplo).
    pub unidade: Option<char>,
    pub midia: Midia,
    /// Vai para a tela mesmo sem a mídia: um modelo de disco a pessoa consegue pesquisar.
    pub modelo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnidadeLida {
    pub midia: Midia,
    pub modelo: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Leitura {
    pub unidades: HashMap<char, UnidadeLida>,
    /// Nunca omitido: numa imagem modificada, é a explicação de por que o produto não sabe.
    pub lacunas: Vec<String>,
}

/// Caminho de rede não é engano: com NAS, a resposta certa é não opinar.
pub fn unidade_do_caminho(caminho: &str) -> Option<char> {
    let mut chars = caminho.chars();

    let letra = chars.next()?.to_ascii_uppercase();
    if !letra.is_ascii_alphabetic() {
        return None;
    }

    if chars.next()? != ':' {
        return None;
    }

    Some(letra)
}

/// Uma consulta para todas as unidades. `MSFT_PhysicalDisk` vem VAZIA em Windows modificado (depende do serviço
/// de armazenamento); a reserva é a cadeia `Win32_DiskDrive` → `Partition` → `LogicalDisk`, que não diz se é SSD,
/// mas entrega o MODELO.
#[cfg(target_os = "windows")]
pub fn ler_unidades() -> Leitura {
    let rapida = "$ns = 'root\\Microsoft\\Windows\\Storage'; \
                  $discos = @{}; \
                  Get-CimInstance -Namespace $ns -ClassName MSFT_PhysicalDisk | \
                    ForEach-Object { $discos[[string]$_.DeviceId] = $_.MediaType }; \
                  Get-CimInstance -Namespace $ns -ClassName MSFT_Partition | \
                    Where-Object { $_.DriveLetter } | \
                    ForEach-Object { \
                      '{0}={1}' -f $_.DriveLetter, $discos[[string]$_.DiskNumber] \
                    }";

    if let Ok(saida) = super::shell::powershell(rapida) {
        if saida.success {
            let unidades = ler_pares(&saida.stdout);
            if !unidades.is_empty() {
                return Leitura { unidades, lacunas: Vec::new() };
            }
        }
    }

    // O `-join` evita que um modelo com espaço quebre a linha.
    let reserva = "Get-CimInstance Win32_DiskDrive | ForEach-Object { \
                     $d = $_; \
                     Get-CimAssociatedInstance -InputObject $_ \
                       -ResultClassName Win32_DiskPartition | ForEach-Object { \
                       Get-CimAssociatedInstance -InputObject $_ \
                         -ResultClassName Win32_LogicalDisk | ForEach-Object { \
                         '{0}|{1}' -f $_.DeviceID, $d.Model \
                       } \
                     } \
                   }";

    match super::shell::powershell(reserva) {
        Ok(saida) if saida.success && !saida.stdout.trim().is_empty() => Leitura {
            unidades: ler_modelos(&saida.stdout),
            lacunas: vec![
                "O Windows não respondeu o tipo de mídia dos discos (SSD ou mecânico). \
                 Isso acontece quando o serviço de armazenamento está desligado, o que é \
                 comum em Windows modificado. O modelo de cada disco foi lido pelo \
                 caminho antigo e aparece do lado do jogo."
                    .to_string(),
            ],
        },
        _ => Leitura {
            unidades: HashMap::new(),
            lacunas: vec![
                "Não deu para ler em que disco cada unidade está nesta máquina.".to_string(),
            ],
        },
    }
}

#[cfg(not(target_os = "windows"))]
pub fn ler_unidades() -> Leitura {
    Leitura::default()
}

/// A mídia só sai do nome quando ele diz ("SSD", "NVMe"): deduzir pela marca é afirmar sem ler.
pub fn ler_modelos(saida: &str) -> HashMap<char, UnidadeLida> {
    let mut mapa = HashMap::new();

    for linha in saida.lines() {
        let Some((letra, modelo)) = linha.trim().split_once('|') else {
            continue;
        };

        let Some(letra) = letra.trim().chars().next().map(|c| c.to_ascii_uppercase()) else {
            continue;
        };

        if !letra.is_ascii_alphabetic() {
            continue;
        }

        let modelo = modelo.trim().to_string();
        let alto = modelo.to_uppercase();

        let midia = if alto.contains("SSD") || alto.contains("NVME") {
            Midia::Ssd
        } else {
            Midia::NaoDeuParaLer
        };

        mapa.insert(
            letra,
            UnidadeLida {
                midia,
                modelo: (!modelo.is_empty()).then_some(modelo),
            },
        );
    }

    mapa
}

pub fn ler_pares(saida: &str) -> HashMap<char, UnidadeLida> {
    let mut mapa = HashMap::new();

    for linha in saida.lines() {
        let Some((letra, tipo)) = linha.trim().split_once('=') else {
            continue;
        };

        let Some(letra) = letra.trim().chars().next().map(|c| c.to_ascii_uppercase()) else {
            continue;
        };

        if !letra.is_ascii_alphabetic() {
            continue;
        }

        mapa.insert(
            letra,
            UnidadeLida { midia: classificar(tipo.trim()), modelo: None },
        );
    }

    mapa
}

/// 3 = mecânico, 4 = SSD. "Unspecified" e 0 (NVMe atrás de RAID) são `NaoDeuParaLer`: chutar SSD absolveria um
/// cartão SD.
pub fn classificar(tipo: &str) -> Midia {
    let t = tipo.trim().to_uppercase();

    match t.as_str() {
        "3" => return Midia::Mecanico,
        "4" => return Midia::Ssd,
        _ => {}
    }

    if t.contains("SSD") {
        Midia::Ssd
    } else if t.contains("HDD") {
        Midia::Mecanico
    } else {
        Midia::NaoDeuParaLer
    }
}

pub fn juntar(jogos: &[(String, String)], unidades: &HashMap<char, UnidadeLida>) -> Vec<OndeMora> {
    jogos
        .iter()
        .map(|(nome, caminho)| {
            let unidade = unidade_do_caminho(caminho);
            let lida = unidade.and_then(|u| unidades.get(&u));

            OndeMora {
                jogo: nome.clone(),
                caminho: caminho.clone(),
                unidade,
                midia: lida.map(|l| l.midia).unwrap_or(Midia::NaoDeuParaLer),
                modelo: lida.and_then(|l| l.modelo.clone()),
            }
        })
        .collect()
}

/// `NaoDeuParaLer` fica de fora: "seu jogo está no HD" sobre leitura falha é mentira.
pub fn em_disco_mecanico(onde: &[OndeMora]) -> Vec<&OndeMora> {
    onde.iter().filter(|o| o.midia == Midia::Mecanico).collect()
}

pub fn explicar(jogo: &str, unidade: char) -> String {
    format!(
        "O {jogo} está instalado na unidade {unidade}:, que é um disco mecânico. \
         Num servidor de RP o jogo lê textura e modelo o tempo todo enquanto você \
         anda pelo mapa, e um disco de prato não entrega isso na velocidade que o \
         jogo pede — é o que aparece como travada ao virar a esquina e como FPS \
         que muda conforme o lugar. Passar o jogo para o SSD costuma ser o maior \
         ganho isolado numa máquina com os dois discos."
    )
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Relatorio {
    pub jogos: Vec<OndeMora>,
    pub lacunas: Vec<String>,
}

#[cfg(target_os = "windows")]
pub fn analisar() -> Relatorio {
    let leitura = ler_unidades();

    let biblioteca = super::jogos::varrer();
    let jogos: Vec<(String, String)> = biblioteca
        .jogos
        .iter()
        .map(|j| (j.nome.clone(), j.pasta.to_string_lossy().to_string()))
        .collect();

    // Jogo não ENCONTRADO não pode virar "nenhum jogo está no disco errado".
    let mut lacunas = leitura.lacunas;
    lacunas.extend(biblioteca.lacunas);

    Relatorio { jogos: juntar(&jogos, &leitura.unidades), lacunas }
}

#[cfg(not(target_os = "windows"))]
pub fn analisar() -> Relatorio {
    Relatorio::default()
}

/// O jogo ABERTO agora, que não depende de estar numa biblioteca conhecida.
pub fn do_caminho(jogo: &str, caminho: &Path) -> OndeMora {
    let texto = caminho.to_string_lossy().to_string();
    let leitura = ler_unidades();
    let unidade = unidade_do_caminho(&texto);
    let lida = unidade.and_then(|u| leitura.unidades.get(&u));

    OndeMora {
        jogo: jogo.to_string(),
        caminho: texto,
        unidade,
        midia: lida.map(|l| l.midia).unwrap_or(Midia::NaoDeuParaLer),
        modelo: lida.and_then(|l| l.modelo.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_a_letra_da_unidade() {
        assert_eq!(unidade_do_caminho(r"D:\Games\GTAV"), Some('D'));
        assert_eq!(unidade_do_caminho(r"c:\jogos"), Some('C'));
    }

    #[test]
    fn caminho_de_rede_nao_vira_unidade() {
        assert_eq!(unidade_do_caminho(r"\\servidor\jogos\GTAV"), None);
        assert_eq!(unidade_do_caminho("/mnt/jogos"), None);
        assert_eq!(unidade_do_caminho(""), None);
    }

    #[test]
    fn le_os_pares_da_consulta() {
        let saida = "C=4\r\nD=3\r\nE=SSD\r\n";
        let mapa = ler_pares(saida);

        assert_eq!(mapa[&'C'].midia, Midia::Ssd);
        assert_eq!(mapa[&'D'].midia, Midia::Mecanico);
        assert_eq!(mapa[&'E'].midia, Midia::Ssd);
    }

    #[test]
    fn tipo_desconhecido_nao_vira_ssd() {
        assert_eq!(classificar("0"), Midia::NaoDeuParaLer);
        assert_eq!(classificar("Unspecified"), Midia::NaoDeuParaLer);
        assert_eq!(classificar(""), Midia::NaoDeuParaLer);
        assert_eq!(classificar("5"), Midia::NaoDeuParaLer);
    }

    #[test]
    fn junta_o_jogo_com_a_midia_da_unidade_dele() {
        let mut midias = HashMap::new();
        midias.insert('C', UnidadeLida { midia: Midia::Ssd, modelo: None });
        midias.insert('D', UnidadeLida { midia: Midia::Mecanico, modelo: None });

        let jogos = vec![
            ("Grand Theft Auto V".to_string(), r"D:\Games\GTAV".to_string()),
            ("Counter-Strike 2".to_string(), r"C:\Steam\cs2".to_string()),
        ];

        let onde = juntar(&jogos, &midias);
        let mecanicos = em_disco_mecanico(&onde);

        assert_eq!(mecanicos.len(), 1);
        assert_eq!(mecanicos[0].jogo, "Grand Theft Auto V");
        assert_eq!(mecanicos[0].unidade, Some('D'));
    }

    #[test]
    fn unidade_ilegivel_nao_vira_acusacao_de_disco_mecanico() {
        let midias = HashMap::new(); // nenhuma unidade pôde ser lida

        let jogos = vec![("GTA V".to_string(), r"D:\Games\GTAV".to_string())];
        let onde = juntar(&jogos, &midias);

        assert_eq!(onde[0].midia, Midia::NaoDeuParaLer);
        assert!(
            em_disco_mecanico(&onde).is_empty(),
            "um disco ilegível foi acusado de ser mecânico"
        );
    }

    #[test]
    fn a_frase_diz_o_jogo_e_a_unidade() {
        let frase = explicar("Grand Theft Auto V", 'D');

        assert!(frase.contains("Grand Theft Auto V"));
        assert!(frase.contains("D:"));
        assert!(frase.contains("SSD"), "a frase precisa dizer o que fazer");
    }

    /// Roda contra esta máquina: mostra o que a consulta devolve de verdade.
    #[test]
    #[ignore = "toca o Windows desta máquina"]
    fn as_unidades_desta_maquina() {
        let leitura = ler_unidades();

        for (letra, lida) in &leitura.unidades {
            println!("{letra}: {:?} — {:?}", lida.midia, lida.modelo);
        }
        for lacuna in &leitura.lacunas {
            println!("lacuna: {lacuna}");
        }

        let relatorio = analisar();

        for onde in &relatorio.jogos {
            println!(
                "{} -> {} ({:?}, {:?})",
                onde.jogo, onde.caminho, onde.midia, onde.modelo
            );
        }
        for lacuna in &relatorio.lacunas {
            println!("lacuna: {lacuna}");
        }
    }
}
