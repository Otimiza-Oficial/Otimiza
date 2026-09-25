// Programas de fábrica. Aqui o risco é invertido: errar é marcar o driver de vídeo como lixo e o cliente
// desinstalar. Por isso PRIMEIRO o que nunca pode ser marcado, depois o resto. Nada é desinstalado escondido:
// programa comum abre o desinstalador do fabricante; app da Loja sai pela chamada do Windows e volta pela Loja.

use super::{registry, shell};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BloatKind {
    OemUtility,
    TrialSecurity,
    Sponsored,
    StoreApp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloatItem {
    pub name: String,
    pub publisher: String,
    pub kind: BloatKind,
    /// `None` para apps da Loja (pasta protegida): "0 MB" daria a impressão de que não ocupam espaço.
    pub size_mb: Option<f64>,
    /// Sempre presente: marcar sem explicar é o que faz o cliente desinstalar coisa errada.
    pub reason: String,
    pub package: Option<String>,
    pub removable_here: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloatReport {
    pub items: Vec<BloatItem>,
    pub total_mb: f64,
    pub unmeasured: usize,
    pub programs_scanned: usize,
    /// Com o serviço da Loja desligado, a lista saía sem nenhum app da Loja e sem uma palavra sobre isso.
    pub lacunas: Vec<String>,
}

/// Consultada ANTES de qualquer regra: driver, runtime e biblioteca de sistema jamais aparecem como lixo.
const NUNCA_MARCAR: [&str; 18] = [
    "driver",
    "chipset",
    "audio",
    "realtek",
    "graphics",
    "geforce",
    "radeon",
    "runtime",
    "redistributable",
    "redistributável",
    "framework",
    ".net",
    "visual c++",
    "directx",
    "update for",
    "security update",
    "hotfix",
    "service pack",
];

pub fn protegido(nome: &str, editor: &str) -> bool {
    let alvo = format!("{} {}", nome, editor).to_lowercase();
    NUNCA_MARCAR.iter().any(|termo| alvo.contains(termo))
}

const PADROES: [(&str, BloatKind, &str); 22] = [
    ("hp support assistant", BloatKind::OemUtility, "Utilitário da HP que roda em segundo plano procurando atualizações."),
    ("hp jumpstart", BloatKind::OemUtility, "Assistente de configuração da HP, útil só na primeira semana de uso."),
    ("hp documentation", BloatKind::OemUtility, "Manuais da HP, disponíveis no site da fabricante."),
    ("dell supportassist", BloatKind::OemUtility, "Utilitário da Dell que roda em segundo plano verificando o computador."),
    ("dell digital delivery", BloatKind::OemUtility, "Entregador de software da Dell, usado só na primeira configuração."),
    ("dell customer connect", BloatKind::OemUtility, "Canal de comunicação comercial da Dell."),
    ("lenovo vantage", BloatKind::OemUtility, "Painel da Lenovo com atualizações e propaganda de acessórios."),
    ("lenovo now", BloatKind::OemUtility, "Ofertas da Lenovo apresentadas dentro do Windows."),
    ("acer care center", BloatKind::OemUtility, "Painel da Acer que roda em segundo plano."),
    ("acer jumpstart", BloatKind::OemUtility, "Assistente inicial da Acer."),
    ("asus giftbox", BloatKind::OemUtility, "Vitrine de aplicativos patrocinados da ASUS."),
    ("asus webstorage", BloatKind::OemUtility, "Armazenamento em nuvem da ASUS, em teste."),
    ("mcafee", BloatKind::TrialSecurity, "Antivírus em teste que veio de fábrica. Expira e passa a pedir renovação — e brigar com o Defender enquanto isso."),
    ("norton security", BloatKind::TrialSecurity, "Antivírus em teste que veio de fábrica."),
    ("norton 360", BloatKind::TrialSecurity, "Antivírus em teste que veio de fábrica."),
    ("avast free", BloatKind::TrialSecurity, "Antivírus de terceiro rodando junto com o Defender."),
    ("avg antivirus", BloatKind::TrialSecurity, "Antivírus de terceiro rodando junto com o Defender."),
    ("wildtangent", BloatKind::Sponsored, "Plataforma de jogos patrocinada, pré-instalada de fábrica."),
    ("wild tangent", BloatKind::Sponsored, "Plataforma de jogos patrocinada, pré-instalada de fábrica."),
    ("booking.com", BloatKind::Sponsored, "Atalho comercial pré-instalado."),
    ("keeper password", BloatKind::Sponsored, "Gerenciador de senhas em teste, pré-instalado de fábrica."),
    ("expressvpn", BloatKind::Sponsored, "VPN em teste, pré-instalada de fábrica."),
];

/// Curta e conservadora: nada que alguém use de verdade. Spotify, Netflix e afins ficam de fora.
const APPS_DA_LOJA: [(&str, &str); 10] = [
    ("Microsoft.BingNews", "Notícias da Microsoft, pré-instalado."),
    ("Microsoft.BingWeather", "Previsão do tempo da Microsoft, pré-instalado."),
    ("Microsoft.GetHelp", "Assistente de suporte da Microsoft."),
    ("Microsoft.Getstarted", "Tour de boas-vindas do Windows."),
    ("Microsoft.MicrosoftSolitaireCollection", "Coleção de paciência, com anúncios."),
    ("Microsoft.MicrosoftOfficeHub", "Atalho para comprar o Office."),
    ("Microsoft.MixedReality.Portal", "Portal de realidade mista, sem uso sem o óculos."),
    ("Microsoft.People", "Agenda de contatos, substituída pelo Outlook."),
    ("Microsoft.SkypeApp", "Skype pré-instalado, descontinuado pela Microsoft."),
    ("Clipchamp.Clipchamp", "Editor de vídeo pré-instalado."),
];

const UNINSTALL_KEYS: [(&str, &str); 3] = [
    ("HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
    ("HKLM", r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
    ("HKCU", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
];

struct ProgramaInstalado {
    nome: String,
    editor: String,
    tamanho_kb: u32,
}

/// `Err` quando uma chave não abre; entrada com `DisplayName` ilegível fica de fora sozinha e não conta.
fn ler_programas() -> Result<Vec<ProgramaInstalado>, String> {
    let mut programas = Vec::new();

    for (hive, base) in UNINSTALL_KEYS {
        for entrada in registry::subkeys(hive, base)? {
            let caminho = format!("{}\\{}", base, entrada);

            let Ok(Some(nome)) = registry::read_text(hive, &caminho, "DisplayName") else {
                continue;
            };
            if nome.trim().is_empty() {
                continue;
            }

            let tamanho_kb = match registry::read(hive, &caminho, "EstimatedSize") {
                Ok(crate::modules::changelog::PreviousValue::Dword(kb)) => kb,
                _ => 0,
            };

            programas.push(ProgramaInstalado {
                nome: nome.trim().to_string(),
                editor: registry::read_text(hive, &caminho, "Publisher")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                tamanho_kb,
            });
        }
    }

    Ok(programas)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawAppx {
    name: Option<String>,
    package_full_name: Option<String>,
}

fn ler_apps_da_loja() -> Result<Vec<RawAppx>, String> {
    let script = "ConvertTo-Json -Compress -Depth 2 -InputObject @(Get-AppxPackage \
                  -ErrorAction Stop | Select-Object Name,PackageFullName)";

    shell::json_da_saida(shell::powershell(script), "os aplicativos da Microsoft Store")
}

pub fn classificar(nome: &str, editor: &str) -> Option<(BloatKind, String)> {
    // A proteção vem primeiro: com "driver" no nome, nenhuma regra seguinte avalia.
    if protegido(nome, editor) {
        return None;
    }

    let alvo = format!("{} {}", nome, editor).to_lowercase();

    PADROES
        .iter()
        .find(|(termo, _, _)| alvo.contains(termo))
        .map(|(_, kind, motivo)| (*kind, motivo.to_string()))
}

/// `Err` sem os programas instalados; os apps da Loja ilegíveis viram lacuna.
pub fn analyze() -> Result<BloatReport, String> {
    let programas = ler_programas()?;
    let programs_scanned = programas.len();
    let mut items = Vec::new();
    let mut lacunas = Vec::new();

    for p in &programas {
        if let Some((kind, reason)) = classificar(&p.nome, &p.editor) {
            items.push(BloatItem {
                name: p.nome.clone(),
                publisher: p.editor.clone(),
                kind,
                size_mb: if p.tamanho_kb > 0 {
                    Some(p.tamanho_kb as f64 / 1024.0)
                } else {
                    None
                },
                reason,
                package: None,
                // Abrir o desinstalador oficial é mais seguro que imitá-lo.
                removable_here: false,
            });
        }
    }

    let apps = match ler_apps_da_loja() {
        Ok(apps) => apps,
        Err(erro) => {
            lacunas.push(erro);
            Vec::new()
        }
    };

    for app in apps {
        let Some(nome) = app.name else { continue };

        if let Some((_, motivo)) = APPS_DA_LOJA.iter().find(|(id, _)| *id == nome) {
            items.push(BloatItem {
                name: nome.clone(),
                publisher: "Microsoft Store".to_string(),
                kind: BloatKind::StoreApp,
                size_mb: None,
                reason: format!("{} Pode ser reinstalado pela Loja quando quiser.", motivo),
                package: app.package_full_name,
                removable_here: true,
            });
        }
    }

    // Os de tamanho desconhecido vão para o fim: não dá para afirmar que são pequenos.
    items.sort_by(|a, b| match (a.size_mb, b.size_mb) {
        (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    let total_mb = items.iter().filter_map(|i| i.size_mb).sum();
    let unmeasured = items.iter().filter(|i| i.size_mb.is_none()).count();

    Ok(BloatReport {
        items,
        total_mb,
        unmeasured,
        programs_scanned,
        lacunas,
    })
}

/// Só apps da Loja passam por aqui. Programa comum nunca é desinstalado por nós.
pub fn remover_app_da_loja(package: &str) -> Result<String, String> {
    // O nome entra num comando do PowerShell: só passam os caracteres que um pacote pode ter.
    if package.is_empty()
        || !package
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err("Identificador de pacote inválido.".to_string());
    }

    let script = format!(
        "Remove-AppxPackage -Package '{}' -ErrorAction Stop",
        package
    );

    shell::powershell_checked(&script)
        .map_err(|e| format!("Não foi possível remover: {}", e))?;

    Ok("Aplicativo removido. Ele pode ser reinstalado pela Microsoft Store.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_nunca_e_marcado_como_lixo() {
        let drivers = [
            ("NVIDIA Graphics Driver 566.36", "NVIDIA Corporation"),
            ("Realtek High Definition Audio Driver", "Realtek"),
            ("Intel Chipset Device Software", "Intel"),
            ("AMD Radeon Software", "Advanced Micro Devices"),
            ("Microsoft Visual C++ 2015-2022 Redistributable", "Microsoft"),
            (".NET Runtime 8.0.11", "Microsoft"),
            ("DirectX Runtime", "Microsoft"),
        ];

        for (nome, editor) in drivers {
            assert!(
                classificar(nome, editor).is_none(),
                "marcou como lixo o que é essencial: {}",
                nome
            );
        }
    }

    #[test]
    fn utilitario_de_fabricante_e_marcado() {
        for (nome, editor) in [
            ("HP Support Assistant", "HP Inc."),
            ("Dell SupportAssist", "Dell Inc."),
            ("Lenovo Vantage", "Lenovo"),
        ] {
            let achado = classificar(nome, editor);
            assert!(achado.is_some(), "não marcou {}", nome);
            assert_eq!(achado.unwrap().0, BloatKind::OemUtility);
        }
    }

    #[test]
    fn antivirus_de_fabrica_e_marcado_como_teste() {
        let (kind, motivo) = classificar("McAfee LiveSafe", "McAfee, LLC").unwrap();
        assert_eq!(kind, BloatKind::TrialSecurity);
        assert!(motivo.to_lowercase().contains("defender"));
    }

    #[test]
    fn a_protecao_vence_o_padrao() {
        // Um driver da McAfee casaria com "mcafee": a proteção precisa vencer.
        assert!(classificar("McAfee Firewall Driver", "McAfee").is_none());
    }

    #[test]
    fn todo_padrao_tem_motivo_escrito() {
        for (termo, _, motivo) in PADROES {
            assert!(
                !motivo.trim().is_empty(),
                "padrão `{}` marca sem explicar o porquê",
                termo
            );
        }
        for (id, motivo) in APPS_DA_LOJA {
            assert!(!motivo.trim().is_empty(), "app `{}` sem motivo", id);
        }
    }

    #[test]
    fn recusa_identificador_de_pacote_com_comando_embutido() {
        assert!(remover_app_da_loja("").is_err());
        assert!(remover_app_da_loja("App'; Remove-Item C:\\ -Recurse; '").is_err());
        assert!(remover_app_da_loja("App Nome Com Espaco").is_err());
    }

    #[test]
    fn nada_de_programa_comum_e_removido_por_aqui() {
        let r = analyze().expect("os programas instalados desta máquina precisam ser legíveis");
        for item in r.items.iter().filter(|i| i.kind != BloatKind::StoreApp) {
            assert!(
                !item.removable_here,
                "{} não pode ser removido pelo Otimiza",
                item.name
            );
        }
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze().expect("os programas instalados desta máquina precisam ser legíveis");
        println!(
            "{} programas examinados, {} marcados, {:.0} MB",
            r.programs_scanned,
            r.items.len(),
            r.total_mb
        );
        for lacuna in &r.lacunas {
            println!("  não li: {}", lacuna);
        }

        for i in &r.items {
            let tamanho = match i.size_mb {
                Some(mb) => format!("{:.0} MB", mb),
                None => "tamanho não informado".to_string(),
            };
            println!("  [{:?}] {:<45} {}", i.kind, i.name, tamanho);
        }

        assert!(r.items.iter().all(|i| !i.reason.trim().is_empty()));
        let medidos: Vec<f64> = r.items.iter().filter_map(|i| i.size_mb).collect();
        assert!(medidos.windows(2).all(|p| p[0] >= p[1]));

        let primeiro_sem_tamanho = r.items.iter().position(|i| i.size_mb.is_none());
        if let Some(i) = primeiro_sem_tamanho {
            assert!(r.items[i..].iter().all(|x| x.size_mb.is_none()));
        }

        assert_eq!(r.unmeasured, r.items.iter().filter(|i| i.size_mb.is_none()).count());
    }
}
