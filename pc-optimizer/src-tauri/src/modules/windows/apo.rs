// Intel Application Optimization (3.0)
//
// A Intel valida o APO em uma lista fechada de processadores e jogos, no Windows 11. Ele chega pelo driver
// Intel Dynamic Tuning Technology (11405 ou mais novo), com a opção ligada na BIOS, e aparece no Gerenciador de
// Dispositivos como "Intel(R) Innovation Platform Framework" ou "Intel(R) Dynamic Tuning". Fonte:
// https://www.intel.com/content/www/us/en/support/articles/000095419/processors.html
//
// Só lê e aponta. Não instala driver: instalador de terceiros não tem desfazer confiável.

use super::achados::{FindingSeverity, FixLocation};

/// Modelos da lista "verified" da Intel (setembro de 2026), sem o prefixo de família.
const VALIDADOS: &[&str] = &[
    "14900ks", "14900k", "14900kf", "14700k", "14700kf", "14600k", "14600kf", "14900hx", "14700hx", "285k", "265k",
    "265kf", "245k", "245kf", "285hx", "275hx", "265hx", "255hx", "245hx", "235hx", "356h", "386h",
];

/// Os "Plus", validados só com a palavra no nome.
const VALIDADOS_PLUS: &[&str] = &["270k", "250k", "250kf", "290hx", "270hx"];

/// O modelo depois da família: "Intel(R) Core(TM) i9-14900K" → "14900k"; "Intel(R) Core(TM) Ultra 9 285K" → "285k".
/// **Função pura.**
fn modelo(cpu: &str) -> Option<String> {
    let n = cpu.to_lowercase();
    let depois = if let Some(i) = ["i9-", "i7-", "i5-"].iter().find_map(|p| n.find(p).map(|i| i + p.len())) {
        &n[i..]
    } else {
        let i = n.find("ultra ")? + "ultra ".len();
        n[i..].split_whitespace().nth(1)?
    };
    let m: String = depois.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
    (!m.is_empty()).then_some(m)
}

/// O processador está na lista validada da Intel. **Função pura.**
pub fn processador_validado(cpu: &str) -> bool {
    if !cpu.to_lowercase().contains("intel") {
        return false;
    }
    let Some(m) = modelo(cpu) else { return false };
    let plus = cpu.to_lowercase().contains("plus");
    if plus {
        VALIDADOS_PLUS.contains(&m.as_str())
    } else {
        VALIDADOS.contains(&m.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Achado {
    pub id: &'static str,
    pub titulo: String,
    pub medido: String,
    pub conselho: String,
    pub severidade: FindingSeverity,
    pub onde: FixLocation,
}

/// `driver_presente`: `None` é não deu para ler, e não vira acusação. **Função pura.**
pub fn achados(cpu: &str, windows_11: bool, driver_presente: Option<bool>) -> Vec<Achado> {
    if !windows_11 || !processador_validado(cpu) || driver_presente != Some(false) {
        return Vec::new();
    }
    vec![Achado {
        id: "apo_sem_driver",
        titulo: "Intel Application Optimization disponível e não instalado".to_string(),
        medido: format!(
            "O {} está na lista validada da Intel, e o driver Intel Dynamic Tuning não aparece no Gerenciador de Dispositivos.",
            cpu.trim()
        ),
        conselho: "O APO chega pelo driver Intel Dynamic Tuning Technology (11405 ou mais novo), baixado do site da sua \
                   placa-mãe, com a opção ligada na BIOS. A Intel diz que ele rende nos jogos da lista dela; nos \
                   outros não muda nada. O Otimiza não instala driver."
            .to_string(),
        severidade: FindingSeverity::Important,
        onde: FixLocation::Software,
    }]
}

/// Windows 11 começa no build 22000.
#[cfg(windows)]
pub fn windows_11() -> bool {
    super::registry::read_text("HKLM", r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuildNumber")
        .ok()
        .flatten()
        .and_then(|b| b.trim().parse::<u32>().ok())
        .is_some_and(|b| b >= 22000)
}

/// O driver aparece no Gerenciador de Dispositivos. `None`: a consulta falhou.
#[cfg(windows)]
pub fn driver_presente() -> Option<bool> {
    let script = "try { $d = @(Get-PnpDevice -PresentOnly -ErrorAction Stop | Where-Object { $_.FriendlyName -like '*Innovation Platform Framework*' -or $_.FriendlyName -like '*Dynamic Tuning*' }); \
                  if ($d.Count -gt 0) { 'sim' } else { 'nao' } } catch { 'erro' }";
    let saida = super::shell::powershell(script).ok().filter(|s| s.success)?;
    match saida.stdout.trim() {
        "sim" => Some(true),
        "nao" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn reconhece_a_lista_da_intel() {
        assert!(processador_validado("Intel(R) Core(TM) i9-14900K"));
        assert!(processador_validado("13th Gen Intel(R) Core(TM) i7-14700KF"));
        assert!(processador_validado("Intel(R) Core(TM) Ultra 9 285K"));
        assert!(processador_validado("Intel(R) Core(TM) Ultra 7 270K Plus"));
        assert!(!processador_validado("Intel(R) Core(TM) i5-14600"), "sem K não está na lista");
        assert!(!processador_validado("Intel(R) Core(TM) i5-14400F"));
        assert!(!processador_validado("Intel(R) Core(TM) Ultra 7 270K"), "270K só como Plus");
        assert!(!processador_validado("Intel(R) Core(TM) i3-10100F CPU @ 3.60GHz"));
        assert!(!processador_validado("AMD Ryzen 7 7800X3D 8-Core Processor"));
    }

    #[test]
    fn so_acusa_com_o_driver_lido_e_ausente_no_windows_11() {
        let cpu = "Intel(R) Core(TM) i9-14900K";
        assert_eq!(achados(cpu, true, Some(false)).len(), 1);
        assert!(achados(cpu, true, Some(true)).is_empty());
        assert!(achados(cpu, true, None).is_empty(), "leitura falha não vira acusação");
        assert!(achados(cpu, false, Some(false)).is_empty(), "o APO exige Windows 11");
    }
}
