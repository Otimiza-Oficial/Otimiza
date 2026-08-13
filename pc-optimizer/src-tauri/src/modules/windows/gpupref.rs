// Qual placa de vídeo cada jogo usa
//
// Este é o maior ganho de FPS que o Otimiza consegue entregar — e só existe em
// máquina com duas placas de vídeo, que é o caso de praticamente todo notebook.
//
// O DEFEITO QUE ELE CONSERTA
//
// Notebook de jogo tem duas placas: a integrada ao processador, que gasta pouco
// e desenha pouco, e a dedicada, que é a placa de verdade. O Windows escolhe
// qual usar por jogo, e às vezes escolhe errado — o jogo abre na integrada e
// roda a uma fração do que a máquina consegue.
//
// O cliente não tem como perceber isso. O jogo abre, roda mal, e ele conclui
// que o PC é fraco. Muitas vezes não é: é a placa boa parada do lado.
//
// Quando o Otimiza acerta esse caso, o ganho é de duas a cinco vezes — mais do
// que todo o resto do catálogo somado. Quando a máquina tem uma placa só, o
// ganho é EXATAMENTE ZERO, e o produto não pode oferecer nada.
//
// COMO ISSO É GRAVADO
//
// `HKCU\SOFTWARE\Microsoft\DirectX\UserGpuPreferences`, um valor de texto por
// jogo. O nome do valor é o caminho completo do executável; o conteúdo é
// `GpuPreference=N;` — e o ponto e vírgula faz parte, não é enfeite.
//
// É a mesma chave que a tela de Configurações do Windows usa. Não exige
// administrador, não exige reiniciar o PC, e é reversível: guardamos o valor
// anterior como qualquer outra mudança do produto. O jogo precisa ser reaberto
// para valer.

use crate::modules::changelog::{ChangeRecord, PreviousValue};
use serde::{Deserialize, Serialize};
use std::path::Path;

const CHAVE: &str = r"SOFTWARE\Microsoft\DirectX\UserGpuPreferences";

/// Qual placa o Windows deve usar para um programa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preferencia {
    /// O Windows decide. É o padrão de fábrica.
    Automatica,
    /// A placa que gasta menos — a integrada, num notebook.
    Economia,
    /// A placa de verdade.
    Desempenho,
}

impl Preferencia {
    fn codigo(self) -> u8 {
        match self {
            Preferencia::Automatica => 0,
            Preferencia::Economia => 1,
            Preferencia::Desempenho => 2,
        }
    }

    fn do_codigo(codigo: u8) -> Option<Self> {
        match codigo {
            0 => Some(Preferencia::Automatica),
            1 => Some(Preferencia::Economia),
            2 => Some(Preferencia::Desempenho),
            _ => None,
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            Preferencia::Automatica => "decidida pelo Windows",
            Preferencia::Economia => "placa de economia",
            Preferencia::Desempenho => "placa de desempenho",
        }
    }
}

/// Monta o texto que vai para o registro.
///
/// O formato é o do Windows, e o ponto e vírgula final faz parte dele.
pub fn texto_da_preferencia(preferencia: Preferencia) -> String {
    format!("GpuPreference={};", preferencia.codigo())
}

/// Lê a preferência de um texto do registro.
///
/// O valor pode carregar mais de um ajuste separado por ponto e vírgula, então
/// não dá para comparar a string inteira — é preciso procurar o campo.
pub fn preferencia_do_texto(bruto: &str) -> Option<Preferencia> {
    bruto
        .split(';')
        .filter_map(|campo| campo.split_once('='))
        .find(|(chave, _)| chave.trim().eq_ignore_ascii_case("GpuPreference"))
        .and_then(|(_, valor)| valor.trim().parse::<u8>().ok())
        .and_then(Preferencia::do_codigo)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuPrefFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: super::achados::FindingSeverity,
    pub fix_location: super::achados::FixLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuPrefReport {
    /// Quantas placas de vídeo a máquina tem. Com uma só, este módulo inteiro
    /// não tem o que fazer.
    pub placas: Vec<String>,
    pub tem_placa_dupla: bool,
    /// Jogos com preferência gravada, e qual.
    pub definidos: Vec<(String, Preferencia)>,
    pub findings: Vec<GpuPrefFinding>,
}

// ------------------------------------------------------------------- leitura

/// As placas de vídeo da máquina.
#[cfg(target_os = "windows")]
pub fn placas() -> Vec<String> {
    // `PNPDeviceID` começando com `PCI\` descarta adaptador virtual de área de
    // trabalho remota e software de captura, que aparecem como placa de vídeo
    // e fariam qualquer PC parecer ter duas.
    let script = "@(Get-CimInstance Win32_VideoController | \
                  Where-Object { $_.PNPDeviceID -like 'PCI\\*' } | \
                  Select-Object -ExpandProperty Name)";

    match super::shell::powershell(script) {
        Ok(saida) if saida.success => saida
            .stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn placas() -> Vec<String> {
    Vec::new()
}

/// O que já está gravado, jogo por jogo.
pub fn definidos() -> Vec<(String, Preferencia)> {
    super::registry::value_names("HKCU", CHAVE)
        .into_iter()
        .filter_map(|caminho| {
            let bruto = super::registry::read_text("HKCU", CHAVE, &caminho)?;
            let preferencia = preferencia_do_texto(&bruto)?;
            Some((caminho, preferencia))
        })
        .collect()
}

// --------------------------------------------------------------- diagnóstico

/// Regras puras.
///
/// Recebe tudo pronto para poder ser testada numa máquina de uma placa só —
/// que é justamente o caso em que a resposta certa é ficar calado.
pub fn diagnosticar(
    placas: &[String],
    definidos: &[(String, Preferencia)],
) -> Vec<GpuPrefFinding> {
    use super::achados::{FindingSeverity, FixLocation};

    // Com uma placa só, escolher placa não existe. Falar aqui seria inventar
    // uma otimização para vender — exatamente o que este produto não faz.
    if placas.len() < 2 {
        return Vec::new();
    }

    let na_economia: Vec<&String> = definidos
        .iter()
        .filter(|(_, p)| *p == Preferencia::Economia)
        .map(|(caminho, _)| caminho)
        .filter(|caminho| Path::new(caminho).exists())
        .collect();

    if na_economia.is_empty() {
        return Vec::new();
    }

    let nomes: Vec<String> = na_economia
        .iter()
        .take(3)
        .map(|caminho| {
            Path::new(caminho)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| (*caminho).clone())
        })
        .collect();

    vec![GpuPrefFinding {
        id: "jogo_na_placa_errada".to_string(),
        title: "Jogo configurado para a placa de vídeo mais fraca".to_string(),
        measured: format!(
            "{} programa(s) estão fixados na placa de economia, numa máquina com {} placas: {}.",
            na_economia.len(),
            placas.len(),
            nomes.join(", ")
        ),
        advice: "Num PC com duas placas de vídeo, jogo rodando na placa de economia entrega \
                 uma fração do que a máquina consegue — e é invisível para quem está jogando: \
                 o jogo abre normalmente e só roda mal. Trocar para a placa de desempenho é \
                 o maior ganho de FPS que existe nesse caso, e vale na próxima vez que o jogo \
                 abrir."
            .to_string(),
        severity: FindingSeverity::Critical,
        fix_location: FixLocation::Software,
    }]
}

pub fn analyze() -> GpuPrefReport {
    let placas = placas();
    let definidos = definidos();
    let findings = diagnosticar(&placas, &definidos);

    GpuPrefReport {
        tem_placa_dupla: placas.len() >= 2,
        placas,
        definidos,
        findings,
    }
}

// ------------------------------------------------------------------ escrita

/// Fixa qual placa um jogo deve usar.
///
/// Devolve o registro da mudança para o histórico: como toda alteração do
/// produto, esta volta atrás com o valor exato que existia antes.
pub fn definir(executavel: &Path, preferencia: Preferencia) -> Result<ChangeRecord, String> {
    // O nome do valor é o caminho completo, então ele precisa ser um caminho
    // completo de verdade — e de um arquivo que exista. Sem isso, esta função
    // viraria uma forma de escrever texto arbitrário no registro do cliente.
    if !executavel.is_absolute() {
        return Err("O caminho do jogo precisa ser completo.".to_string());
    }

    if !executavel.exists() {
        return Err(format!(
            "`{}` não existe. Só dá para escolher a placa de um jogo instalado.",
            executavel.display()
        ));
    }

    let chave_do_valor = executavel.to_string_lossy().to_string();

    let anterior = super::registry::read("HKCU", CHAVE, &chave_do_valor)
        .unwrap_or(PreviousValue::Absent);

    super::registry::set_string(
        "HKCU",
        CHAVE,
        &chave_do_valor,
        &texto_da_preferencia(preferencia),
    )?;

    Ok(ChangeRecord::RegistryValue {
        hive: "HKCU".to_string(),
        path: CHAVE.to_string(),
        name: chave_do_valor,
        previous: anterior,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_formato_do_windows_leva_ponto_e_virgula() {
        // Sem o ponto e vírgula o Windows ignora o valor em silêncio, e o
        // produto acharia que aplicou.
        assert_eq!(texto_da_preferencia(Preferencia::Desempenho), "GpuPreference=2;");
        assert_eq!(texto_da_preferencia(Preferencia::Economia), "GpuPreference=1;");
        assert_eq!(texto_da_preferencia(Preferencia::Automatica), "GpuPreference=0;");
    }

    #[test]
    fn le_a_preferencia_mesmo_com_outros_ajustes_na_mesma_linha() {
        // O valor pode carregar mais de um ajuste. Comparar a string inteira
        // faria o produto não reconhecer o que ele mesmo gravou.
        assert_eq!(
            preferencia_do_texto("GpuPreference=2;"),
            Some(Preferencia::Desempenho)
        );
        assert_eq!(
            preferencia_do_texto("AutoHDREnable=1;GpuPreference=1;"),
            Some(Preferencia::Economia)
        );
        assert_eq!(preferencia_do_texto("AutoHDREnable=1;"), None);
        assert_eq!(preferencia_do_texto(""), None);
        assert_eq!(preferencia_do_texto("GpuPreference=9;"), None);
    }

    #[test]
    fn maquina_de_uma_placa_nao_ganha_achado_nenhum() {
        // A regra mais importante deste arquivo. Num desktop com uma placa só,
        // escolher placa não existe — e oferecer isso seria vender uma
        // otimização que não pode entregar nada.
        let definidos = vec![(
            r"C:\Jogo\jogo.exe".to_string(),
            Preferencia::Economia,
        )];

        assert!(diagnosticar(&["NVIDIA GeForce GTX 1650".to_string()], &definidos).is_empty());
        assert!(diagnosticar(&[], &definidos).is_empty());
    }

    #[test]
    fn placa_dupla_sem_jogo_na_economia_tambem_fica_calado() {
        let placas = vec![
            "Intel UHD Graphics".to_string(),
            "NVIDIA GeForce RTX 4060".to_string(),
        ];

        assert!(diagnosticar(&placas, &[]).is_empty());
        assert!(diagnosticar(
            &placas,
            &[(r"C:\Jogo\jogo.exe".to_string(), Preferencia::Desempenho)]
        )
        .is_empty());
    }

    #[test]
    fn jogo_apagado_nao_conta() {
        // O Windows nunca limpa esta chave, então ela guarda jogo desinstalado
        // há anos. Contar aqueles faria o produto acusar um problema que já não
        // existe na máquina.
        let placas = vec![
            "Intel UHD Graphics".to_string(),
            "NVIDIA GeForce RTX 4060".to_string(),
        ];
        let definidos = vec![(
            r"C:\Jogo\Que\Nao\Existe\Mais\jogo.exe".to_string(),
            Preferencia::Economia,
        )];

        assert!(diagnosticar(&placas, &definidos).is_empty());
    }

    #[test]
    fn caminho_relativo_ou_inexistente_e_recusado() {
        // O nome do valor é o caminho, então esta função escreve texto vindo de
        // fora no registro do cliente. Sem estas travas ela viraria uma forma
        // de gravar qualquer coisa lá.
        assert!(definir(Path::new("jogo.exe"), Preferencia::Desempenho).is_err());
        assert!(definir(
            Path::new(r"C:\Nao\Existe\jogo.exe"),
            Preferencia::Desempenho
        )
        .is_err());
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();

        println!("  placas: {:?}", r.placas);
        println!("  placa dupla: {}", r.tem_placa_dupla);
        for (caminho, p) in r.definidos.iter().take(6) {
            println!("    {} → {}", caminho, p.nome());
        }
        for f in &r.findings {
            println!("  [{:?}] {}", f.severity, f.measured);
        }

        // Numa máquina de uma placa só o módulo tem que ficar calado.
        if !r.tem_placa_dupla {
            assert!(
                r.findings.is_empty(),
                "máquina de uma placa não pode receber achado de escolha de placa"
            );
        }
    }
}
