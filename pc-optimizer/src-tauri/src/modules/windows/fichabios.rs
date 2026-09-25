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
    /// O modelo do chip no CPUID ("Intel64 Family 6 Model 183 Stepping 1" →
    /// 183). É ele, e não o nome comercial, que diz qual silício é.
    #[serde(default)]
    pub modelo: Option<u32>,
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

/// O chip Raptor Lake (CPUID família 6, modelo 0xB7). A instabilidade é dele.
/// Parte dos "13ª/14ª geração" (i3-13100/14100, i5-13400/14400 C0, 13500 e
/// 13600 sem K) usa o chip da 12ª (modelo 0x97 ou 0xBF), com outra numeração
/// de microcódigo e sem o defeito: pelo nome comercial eles seriam acusados.
pub const MODELO_RAPTOR_LAKE: u32 = 0xB7;

/// "Intel64 Family 6 Model 183 Stepping 1" → 183. **Função pura.**
pub fn modelo_do_identificador(id: &str) -> Option<u32> {
    let mut partes = id.split_whitespace();
    while let Some(p) = partes.next() {
        if p.eq_ignore_ascii_case("model") {
            return partes.next()?.parse().ok();
        }
    }
    None
}

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
    let intel = cpu.to_lowercase().contains("intel");
    if intel && l.modelo == Some(MODELO_RAPTOR_LAKE) && !e_de_notebook(cpu) {
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
ConvertTo-Json -Compress -InputObject ([ordered]@{ Cpu = [string]$p.ProcessorNameString; Id = [string]$p.Identifier; Revisao = $rev; FwMs = $fw; Hvci = $hvci })"#;

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Bruto {
        cpu: Option<String>,
        id: Option<String>,
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
    if l.cpu.is_none() {
        l.lacunas.push("Não deu para ler o processador: os defeitos conhecidos não foram conferidos.".to_string());
    }
    l.modelo = b.id.as_deref().and_then(modelo_do_identificador);
    let intel = l.cpu.as_deref().is_some_and(|c| c.to_lowercase().contains("intel"));
    if intel {
        l.microcodigo = b.revisao.as_deref().and_then(microcodigo_intel);
        if l.microcodigo.is_none() || l.modelo.is_none() {
            l.lacunas.push("Não deu para ler o modelo do chip ou a revisão do microcódigo.".to_string());
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
/// para o diagnóstico da abertura (sem PowerShell). `Err` quando a leitura
/// falhou num Intel: "não conferido" não pode virar "sem defeito".
#[cfg(windows)]
pub fn defeito_de_microcodigo() -> Result<Option<Defeito>, String> {
    use crate::modules::changelog::PreviousValue;
    const CPU: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";
    let cpu = super::registry::read_text("HKLM", CPU, "ProcessorNameString")?
        .ok_or("o registro não diz qual é o processador")?;
    if !cpu.to_lowercase().contains("intel") {
        return Ok(None);
    }
    let modelo = super::registry::read_text("HKLM", CPU, "Identifier")?.as_deref().and_then(modelo_do_identificador);
    let microcodigo = match super::registry::read("HKLM", CPU, "Update Revision")? {
        PreviousValue::Binary(b) => microcodigo_intel(&b),
        _ => None,
    };
    if modelo.is_none() || microcodigo.is_none() {
        return Err("não deu para ler o modelo do chip ou o microcódigo".to_string());
    }
    let l = Leitura { cpu: Some(cpu.trim().to_string()), modelo, microcodigo, ..Default::default() };
    Ok(defeitos(&l).into_iter().find(|d| matches!(d, Defeito::MicrocodigoIntelAntigo { .. })))
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
        Leitura { cpu: Some(cpu.into()), modelo: Some(MODELO_RAPTOR_LAKE), microcodigo: micro, tempo_da_bios_s: bios_s, ..Default::default() }
    }

    #[test]
    fn o_modelo_sai_do_identificador() {
        assert_eq!(modelo_do_identificador("Intel64 Family 6 Model 183 Stepping 1"), Some(183));
        assert_eq!(modelo_do_identificador("Intel64 Family 6 Model 165 Stepping 3"), Some(165));
        assert_eq!(modelo_do_identificador("AMD64 Family 25 Model 97 Stepping 2"), Some(97));
        assert_eq!(modelo_do_identificador("lixo"), None);
    }

    #[test]
    fn treze_e_quatorze_com_chip_da_doze_nao_sao_acusados() {
        // i5-14400F C0 e i3-13100: nome de 13ª/14ª, silício da 12ª (0x97/0xBF),
        // microcódigo na casa de 0x3x. Pela regra antiga, acusados à toa.
        for (cpu, modelo) in [
            ("14th Gen Intel(R) Core(TM) i5-14400F", 0xBF),
            ("13th Gen Intel(R) Core(TM) i3-13100F", 0xBF),
            ("13th Gen Intel(R) Core(TM) i5-13400", 0x97),
        ] {
            let l = Leitura { cpu: Some(cpu.into()), modelo: Some(modelo), microcodigo: Some(0x35), ..Default::default() };
            assert!(defeitos(&l).is_empty(), "{cpu} foi acusado");
        }
        let sem_modelo = Leitura { cpu: Some("13th Gen Intel(R) Core(TM) i9-13900K".into()), microcodigo: Some(0x100), ..Default::default() };
        assert!(defeitos(&sem_modelo).is_empty(), "sem o modelo não acusa");
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
        let doze = Leitura { cpu: Some("12th Gen Intel(R) Core(TM) i5-12400F".into()), modelo: Some(0x97), microcodigo: Some(0x10), ..Default::default() };
        assert!(defeitos(&doze).is_empty());
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
