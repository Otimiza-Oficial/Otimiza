// Detector de programas de fábrica
//
// Notebook de loja chega com vários GB de utilitário do fabricante, antivírus em
// teste e joguinho patrocinado. É justamente a máquina mais fraca que vem com
// mais peso morto — e o dono nem sabe que aquilo está lá.
//
// AQUI O RISCO É INVERTIDO. Na maior parte deste projeto, errar significa não
// otimizar. Aqui, errar significa marcar o driver de vídeo como lixo e o cliente
// desinstalar. Por isso a ordem das regras é: PRIMEIRO o que nunca pode ser
// marcado, depois o que pode.
//
// E não desinstalamos nada de escondido. Programa comum abre o desinstalador
// oficial do próprio fabricante; aplicativo da Loja é removido pela chamada do
// Windows e pode ser reinstalado pela Loja a qualquer momento.

use super::{registry, shell};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BloatKind {
    /// Utilitário que veio com a marca do computador.
    OemUtility,
    /// Segurança em teste, que expira e fica pedindo renovação.
    TrialSecurity,
    /// Jogo ou aplicativo patrocinado, pré-instalado sem o dono pedir.
    Sponsored,
    /// Aplicativo da Microsoft Store que vem de fábrica no Windows.
    StoreApp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloatItem {
    pub name: String,
    pub publisher: String,
    pub kind: BloatKind,
    /// Tamanho em disco, quando dá para saber.
    ///
    /// `None` para aplicativos da Loja: eles ficam numa pasta protegida do
    /// Windows e o tamanho não é legível. Mostrar "0 MB" nesse caso passaria a
    /// impressão de que não ocupam espaço — e o cliente decidiria com base numa
    /// informação que nós inventamos.
    pub size_mb: Option<f64>,
    /// Por que este programa foi marcado. Sempre presente: marcar sem explicar
    /// é o que faz o cliente desinstalar coisa errada.
    pub reason: String,
    /// Identificador do pacote da Loja, quando for um.
    pub package: Option<String>,
    /// Se dá para remover pelo Otimiza. Programas comuns abrem o desinstalador
    /// do fabricante em vez disso.
    pub removable_here: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloatReport {
    pub items: Vec<BloatItem>,
    /// Soma só do que foi realmente medido.
    pub total_mb: f64,
    /// Quantos itens não têm tamanho conhecido, para a interface poder dizer
    /// que o total é parcial em vez de deixar parecer completo.
    pub unmeasured: usize,
    pub programs_scanned: usize,
}

// ------------------------------------------------------------ nunca marcar

/// Termos que impedem um programa de ser marcado, aconteça o que acontecer.
///
/// Esta lista é consultada ANTES de qualquer regra de detecção. Driver, runtime
/// e biblioteca de sistema jamais podem aparecer como "lixo de fábrica": o
/// cliente confiaria, desinstalaria, e o PC ficaria sem vídeo, sem som ou sem
/// conseguir abrir os programas dele.
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

/// Se este programa está protegido de ser marcado.
pub fn protegido(nome: &str, editor: &str) -> bool {
    let alvo = format!("{} {}", nome, editor).to_lowercase();
    NUNCA_MARCAR.iter().any(|termo| alvo.contains(termo))
}

// ----------------------------------------------------------- o que é marcado

/// (termo procurado, tipo, motivo mostrado ao cliente)
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

/// Aplicativos da Loja que o Windows instala sozinho.
///
/// Lista curta e conservadora de propósito: nada que alguém possa estar usando
/// de verdade como ferramenta. Spotify, Netflix e afins ficam de fora mesmo
/// vindo pré-instalados — quem usa, usa.
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

// ---------------------------------------------------- leitura dos programas

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

fn ler_programas() -> Vec<ProgramaInstalado> {
    let mut programas = Vec::new();

    for (hive, base) in UNINSTALL_KEYS {
        for entrada in registry::subkeys(hive, base).unwrap_or_default() {
            let caminho = format!("{}\\{}", base, entrada);

            let Some(nome) = registry::read_text(hive, &caminho, "DisplayName") else {
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
                editor: registry::read_text(hive, &caminho, "Publisher").unwrap_or_default(),
                tamanho_kb,
            });
        }
    }

    programas
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawAppx {
    name: Option<String>,
    package_full_name: Option<String>,
}

fn ler_apps_da_loja() -> Vec<RawAppx> {
    let script = "ConvertTo-Json -Compress -Depth 2 -InputObject @(Get-AppxPackage \
                  -ErrorAction SilentlyContinue | Select-Object Name,PackageFullName)";

    match shell::run("powershell", &["-NoProfile", "-Command", script]) {
        Ok(o) if o.success && !o.stdout.trim().is_empty() => {
            serde_json::from_str(&o.stdout).unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

/// Classifica um programa comum. `None` quando não é peso morto conhecido.
pub fn classificar(nome: &str, editor: &str) -> Option<(BloatKind, String)> {
    // A proteção vem primeiro, sempre. Um programa com "driver" no nome não é
    // avaliado por nenhuma regra seguinte.
    if protegido(nome, editor) {
        return None;
    }

    let alvo = format!("{} {}", nome, editor).to_lowercase();

    PADROES
        .iter()
        .find(|(termo, _, _)| alvo.contains(termo))
        .map(|(_, kind, motivo)| (*kind, motivo.to_string()))
}

pub fn analyze() -> BloatReport {
    let programas = ler_programas();
    let programs_scanned = programas.len();
    let mut items = Vec::new();

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
                // Programa comum tem desinstalador próprio, muitas vezes com
                // perguntas. Abrir o oficial é mais seguro que tentar imitar.
                removable_here: false,
            });
        }
    }

    for app in ler_apps_da_loja() {
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

    // Maior primeiro; os de tamanho desconhecido vão para o fim, porque não dá
    // para afirmar que são pequenos.
    items.sort_by(|a, b| match (a.size_mb, b.size_mb) {
        (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    let total_mb = items.iter().filter_map(|i| i.size_mb).sum();
    let unmeasured = items.iter().filter(|i| i.size_mb.is_none()).count();

    BloatReport {
        items,
        total_mb,
        unmeasured,
        programs_scanned,
    }
}

/// Remove um aplicativo da Loja.
///
/// Só aplicativos da Loja passam por aqui: eles têm remoção limpa pelo próprio
/// Windows e voltam pela Loja quando o usuário quiser. Programa comum nunca é
/// desinstalado por nós.
pub fn remover_app_da_loja(package: &str) -> Result<String, String> {
    // O nome do pacote vem do Windows, mas nunca se monta comando com texto de
    // fora sem checar: aqui só passam os caracteres que um pacote pode ter.
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

    shell::run_checked("powershell", &["-NoProfile", "-Command", &script])
        .map_err(|e| format!("Não foi possível remover: {}", e))?;

    Ok("Aplicativo removido. Ele pode ser reinstalado pela Microsoft Store.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_nunca_e_marcado_como_lixo() {
        // O teste mais importante deste módulo. Marcar driver como peso morto
        // faria o cliente desinstalar e ficar sem vídeo, som ou rede.
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
        // Precisa explicar o conflito com o Defender, que é o custo real.
        assert!(motivo.to_lowercase().contains("defender"));
    }

    #[test]
    fn a_protecao_vence_o_padrao() {
        // Um driver da McAfee (existe: drivers de firewall) casaria com o
        // padrão "mcafee". A proteção precisa vencer, e vem antes.
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
        // O nome do pacote entra num comando do PowerShell. Nunca se monta
        // comando com texto de fora sem checar o que ele contém.
        assert!(remover_app_da_loja("").is_err());
        assert!(remover_app_da_loja("App'; Remove-Item C:\\ -Recurse; '").is_err());
        assert!(remover_app_da_loja("App Nome Com Espaco").is_err());
    }

    #[test]
    fn nada_de_programa_comum_e_removido_por_aqui() {
        // Programa comum abre o desinstalador do fabricante; imitá-lo é o
        // caminho para deixar instalação pela metade.
        let r = analyze();
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
        let r = analyze();
        println!(
            "{} programas examinados, {} marcados, {:.0} MB",
            r.programs_scanned,
            r.items.len(),
            r.total_mb
        );

        for i in &r.items {
            let tamanho = match i.size_mb {
                Some(mb) => format!("{:.0} MB", mb),
                None => "tamanho não informado".to_string(),
            };
            println!("  [{:?}] {:<45} {}", i.kind, i.name, tamanho);
        }

        // Todo item marcado precisa dizer por quê.
        assert!(r.items.iter().all(|i| !i.reason.trim().is_empty()));
        // Os medidos vêm primeiro, do maior para o menor; os sem tamanho ficam
        // no fim, porque não dá para afirmar que são pequenos.
        let medidos: Vec<f64> = r.items.iter().filter_map(|i| i.size_mb).collect();
        assert!(medidos.windows(2).all(|p| p[0] >= p[1]));

        let primeiro_sem_tamanho = r.items.iter().position(|i| i.size_mb.is_none());
        if let Some(i) = primeiro_sem_tamanho {
            assert!(r.items[i..].iter().all(|x| x.size_mb.is_none()));
        }

        assert_eq!(r.unmeasured, r.items.iter().filter(|i| i.size_mb.is_none()).count());
    }
}
