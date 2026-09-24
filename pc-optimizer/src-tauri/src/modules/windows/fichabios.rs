// A ficha do firmware, para a aba BIOS (3.0)
//
// Só lê. O que o Windows deixa ver sem tocar na NVRAM: processador e
// microcódigo, Integridade de Memória, e quanto a placa-mãe demorou no último
// boot. Os defeitos conhecidos saem de regra pura sobre essa leitura.
//
// A regra que manda aqui é a de `bios.rs`: o Otimiza não grava na BIOS. Um
// defeito de BIOS vira um aviso com a página oficial do fabricante, nunca uma
// atualização feita por nós.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Leitura {
    pub cpu: Option<String>,
    /// A revisão do microcódigo carregada agora (Intel: a palavra alta do
    /// "Update Revision" do registro).
    pub microcodigo: Option<u32>,
    /// A Integridade de Memória (HVCI) está rodando.
    pub integridade_de_memoria: Option<bool>,
    /// Quanto o firmware levou no último boot, em segundos ("Último tempo do
    /// BIOS" do Gerenciador de Tarefas).
    pub tempo_da_bios_s: Option<f64>,
    pub lacunas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "tipo")]
pub enum Defeito {
    /// Intel de mesa da 13ª ou 14ª geração sem o microcódigo 0x12F.
    MicrocodigoIntelAntigo { atual: u32 },
    /// A placa-mãe demora no boot, e é AMD de mesa da série 7000 ou 9000 (AM5).
    BootLentoAm5 { segundos: u32 },
    /// Placa lenta no boot, sem causa conhecida pela leitura.
    BootLento { segundos: u32 },
}

/// A primeira revisão que a Intel publicou como a correção completa da
/// instabilidade Vmin (13ª e 14ª geração de mesa).
pub const MICROCODIGO_INTEL_CORRIGIDO: u32 = 0x12F;

/// Acima disto o firmware está gastando tempo que se nota ao ligar o PC.
const BOOT_LENTO_S: f64 = 15.0;

/// A revisão do microcódigo a partir dos oito bytes do registro. **Função pura.**
///
/// No Intel a revisão fica nos quatro bytes de cima, em little-endian. No AMD o
/// campo segue outro formato, e por isso a função só é chamada com Intel.
pub fn microcodigo_intel(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 8 {
        return None;
    }
    let r = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    (r != 0).then_some(r)
}

/// O sufixo de letras logo depois do número do modelo (`i9-14900HX` → `hx`,
/// `7800X3D` → `x`). **Função pura.**
fn sufixo_do_modelo(nome: &str) -> Option<String> {
    nome.to_lowercase().split_whitespace().find_map(|p| {
        let p = ["i3-", "i5-", "i7-", "i9-"].iter().find_map(|m| p.strip_prefix(m)).unwrap_or(p);
        let digitos = p.chars().take_while(|c| c.is_ascii_digit()).count();
        (4..=5).contains(&digitos).then(|| p[digitos..].chars().take_while(|c| c.is_ascii_alphabetic()).collect())
    })
}

/// Processador de notebook, pelo sufixo do modelo. **Função pura.**
pub fn e_de_notebook(nome: &str) -> bool {
    sufixo_do_modelo(nome).is_some_and(|s| ["h", "hx", "hk", "hs", "u", "p", "y"].contains(&s.as_str()))
}

/// Os defeitos conhecidos desta leitura. **Função pura.**
pub fn defeitos(l: &Leitura) -> Vec<Defeito> {
    let mut v = Vec::new();
    let cpu = l.cpu.as_deref().unwrap_or("");
    let geracao = super::cpugeracao::geracao_intel(cpu);
    if matches!(geracao, Some(13) | Some(14)) && !e_de_notebook(cpu) {
        if let Some(atual) = l.microcodigo {
            if atual < MICROCODIGO_INTEL_CORRIGIDO {
                v.push(Defeito::MicrocodigoIntelAntigo { atual });
            }
        }
    }
    if let Some(s) = l.tempo_da_bios_s.filter(|s| *s > BOOT_LENTO_S) {
        let am5 = matches!(super::cpugeracao::serie_ryzen(cpu), Some(7) | Some(9)) && !e_de_notebook(cpu);
        let segundos = s.round() as u32;
        v.push(if am5 { Defeito::BootLentoAm5 { segundos } } else { Defeito::BootLento { segundos } });
    }
    v
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ficha {
    pub leitura: Leitura,
    pub defeitos: Vec<Defeito>,
}

/// Lê a máquina. Cada campo que não se lê vira lacuna, e não zero.
#[cfg(windows)]
pub fn ler() -> Ficha {
    let script = r#"$p = Get-ItemProperty 'HKLM:\HARDWARE\DESCRIPTION\System\CentralProcessor\0' -ErrorAction SilentlyContinue
$fw = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Power' -Name FwPOSTTime -ErrorAction SilentlyContinue).FwPOSTTime
$dg = Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard -ErrorAction SilentlyContinue
$hvci = $null
if ($dg) { $hvci = [bool](@($dg.SecurityServicesRunning) -contains 2) }
$rev = $null
if ($p -and $p.'Update Revision') { $rev = [int[]]$p.'Update Revision' }
ConvertTo-Json -Compress -InputObject ([ordered]@{ Cpu = [string]$p.ProcessorNameString; Revisao = $rev; FwMs = $fw; Hvci = $hvci })"#;

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Bruto {
        cpu: Option<String>,
        revisao: Option<Vec<u8>>,
        fw_ms: Option<u64>,
        hvci: Option<bool>,
    }

    let mut l = Leitura::default();
    let bruto: Option<Bruto> = super::shell::powershell(script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(s.stdout.trim()).ok());
    let Some(b) = bruto else {
        l.lacunas.push("Não deu para ler o processador, o microcódigo nem o tempo de boot desta máquina.".to_string());
        return Ficha { leitura: l, defeitos: Vec::new() };
    };

    l.cpu = b.cpu.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
    let intel = l.cpu.as_deref().is_some_and(|c| c.to_lowercase().contains("intel"));
    if intel {
        l.microcodigo = b.revisao.as_deref().and_then(microcodigo_intel);
        if l.microcodigo.is_none() {
            l.lacunas.push("Não deu para ler a revisão do microcódigo.".to_string());
        }
    }
    l.tempo_da_bios_s = b.fw_ms.filter(|ms| *ms > 0).map(|ms| ms as f64 / 1000.0);
    if l.tempo_da_bios_s.is_none() {
        l.lacunas.push("O Windows não registrou quanto a placa-mãe levou no último boot.".to_string());
    }
    l.integridade_de_memoria = b.hvci;
    if l.integridade_de_memoria.is_none() {
        l.lacunas.push("Não deu para saber se a Integridade de Memória está ligada.".to_string());
    }

    let defeitos = defeitos(&l);
    Ficha { leitura: l, defeitos }
}

#[cfg(not(windows))]
pub fn ler() -> Ficha {
    Ficha::default()
}

/// Só o defeito de microcódigo, lido direto do registro: barato o bastante
/// para o diagnóstico da abertura (sem PowerShell).
#[cfg(windows)]
pub fn defeito_de_microcodigo() -> Option<Defeito> {
    use crate::modules::changelog::PreviousValue;
    const CPU: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";
    let cpu = super::registry::read_text("HKLM", CPU, "ProcessorNameString").ok().flatten()?;
    let microcodigo = match super::registry::read("HKLM", CPU, "Update Revision") {
        Ok(PreviousValue::Binary(b)) => microcodigo_intel(&b),
        _ => None,
    };
    let l = Leitura { cpu: Some(cpu.trim().to_string()), microcodigo, ..Default::default() };
    defeitos(&l).into_iter().find(|d| matches!(d, Defeito::MicrocodigoIntelAntigo { .. }))
}

/// Reinicia direto na tela de configuração da BIOS (UEFI).
#[cfg(windows)]
pub fn reiniciar_na_bios() -> Result<(), String> {
    let saida = super::shell::run("shutdown", &["/r", "/fw", "/t", "5"])?;
    if saida.success {
        Ok(())
    } else {
        Err(format!(
            "O Windows não aceitou reiniciar na BIOS{}. Reinicie e aperte Del ou F2 enquanto a marca da placa aparece.",
            if saida.stderr.trim().is_empty() { String::new() } else { format!(" ({})", saida.stderr.trim()) }
        ))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn leitura(cpu: &str, micro: Option<u32>, bios_s: Option<f64>) -> Leitura {
        Leitura { cpu: Some(cpu.into()), microcodigo: micro, tempo_da_bios_s: bios_s, ..Default::default() }
    }

    #[test]
    fn microcodigo_vem_da_palavra_de_cima() {
        assert_eq!(microcodigo_intel(&[0, 0, 0, 0, 0x2F, 0x01, 0, 0]), Some(0x12F));
        assert_eq!(microcodigo_intel(&[0, 0, 0, 0, 0, 0, 0, 0]), None);
        assert_eq!(microcodigo_intel(&[1, 2, 3]), None);
    }

    #[test]
    fn notebook_pelo_sufixo() {
        assert!(e_de_notebook("13th Gen Intel(R) Core(TM) i9-14900HX"));
        assert!(e_de_notebook("13th Gen Intel(R) Core(TM) i7-13700H"));
        assert!(!e_de_notebook("13th Gen Intel(R) Core(TM) i9-13900K"));
        assert!(!e_de_notebook("14th Gen Intel(R) Core(TM) i5-14400F"));
        assert!(!e_de_notebook("AMD Ryzen 7 7800X3D 8-Core Processor"));
        assert!(e_de_notebook("AMD Ryzen 7 7840HS w/ Radeon 780M Graphics"));
    }

    #[test]
    fn intel_de_mesa_com_microcodigo_antigo_e_defeito() {
        let d = defeitos(&leitura("13th Gen Intel(R) Core(TM) i9-13900K", Some(0x129), None));
        assert_eq!(d, vec![Defeito::MicrocodigoIntelAntigo { atual: 0x129 }]);
        assert!(defeitos(&leitura("13th Gen Intel(R) Core(TM) i9-13900K", Some(0x12F), None)).is_empty());
    }

    #[test]
    fn notebook_e_outras_geracoes_nao_acusam_microcodigo() {
        assert!(defeitos(&leitura("13th Gen Intel(R) Core(TM) i9-14900HX", Some(0x100), None)).is_empty());
        assert!(defeitos(&leitura("12th Gen Intel(R) Core(TM) i5-12400F", Some(0x10), None)).is_empty());
        assert!(defeitos(&leitura("13th Gen Intel(R) Core(TM) i9-13900K", None, None)).is_empty(), "sem leitura não acusa");
    }

    #[test]
    fn boot_lento_separa_am5_do_resto() {
        assert_eq!(
            defeitos(&leitura("AMD Ryzen 7 7800X3D 8-Core Processor", None, Some(38.2))),
            vec![Defeito::BootLentoAm5 { segundos: 38 }]
        );
        assert_eq!(
            defeitos(&leitura("AMD Ryzen 5 5600X 6-Core Processor", None, Some(20.0))),
            vec![Defeito::BootLento { segundos: 20 }]
        );
        assert!(defeitos(&leitura("AMD Ryzen 7 7800X3D 8-Core Processor", None, Some(9.0))).is_empty());
    }
}
