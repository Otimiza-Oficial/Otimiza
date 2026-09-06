// Perfil de hardware
//
// A diferença entre um otimizador honesto e uma lista de tweaks copiada da
// internet está aqui: saber COM QUAL máquina se está falando.
//
// Desativar o SysMain é bom em SSD e ruim em HD mecânico. Desativar a compressão
// de memória é bom com RAM sobrando e ruim com 8 GB. Um produto que oferece as
// duas coisas para todo mundo está chutando; este lê o hardware antes de abrir a
// boca.

use super::shell;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Ssd,
    Hdd,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct HardwareProfile {
    /// Tipo de mídia do disco onde o Windows está instalado.
    pub system_storage: StorageKind,
    pub total_ram_gb: f64,
    pub logical_cores: usize,
    pub cpu_name: String,
    pub gpu_name: String,
}

static PROFILE: OnceLock<HardwareProfile> = OnceLock::new();

/// Perfil da máquina, detectado uma vez e reaproveitado.
/// A detecção do disco chama o PowerShell e leva centenas de milissegundos —
/// repetir isso a cada listagem deixaria a interface lenta sem motivo.
pub fn profile() -> &'static HardwareProfile {
    PROFILE.get_or_init(detect)
}

fn detect() -> HardwareProfile {
    let mut system = sysinfo::System::new();
    system.refresh_memory();

    // NÃO É `refresh_cpu_all()`, e a diferença vale quase um segundo.
    //
    // Para reportar PERCENTUAL de uso, o `sysinfo` precisa de duas amostras
    // separadas por um intervalo — e paga esse intervalo dormindo. Medido nesta
    // máquina: 963 ms. Só que daqui sai apenas `cpu.brand()`, o NOME do
    // processador, que já vem na primeira leitura e não depende de amostra
    // nenhuma.
    //
    // Era quase um segundo na abertura do programa, na conta de todo cliente,
    // por um dado que não precisava disso. Há teste garantindo que a leitura
    // barata devolve o mesmo nome que a cara.
    system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing());

    HardwareProfile {
        system_storage: detect_system_storage(),
        total_ram_gb: system.total_memory() as f64 / 1_073_741_824.0,
        logical_cores: num_cpus::get(),
        cpu_name: system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|nome| !nome.is_empty())
            .unwrap_or_else(|| "processador não identificado".to_string()),
        gpu_name: detect_gpu(),
    }
}

/// Nome da placa de vídeo.
///
/// `Win32_VideoController` traz também adaptadores virtuais (área de trabalho
/// remota, captura de tela); o filtro por `AdapterRAM` maior que zero descarta
/// a maioria deles sem precisar de lista de nomes, que envelheceria.
fn detect_gpu() -> String {
    let script = "@(Get-CimInstance Win32_VideoController | \
                  Where-Object { $_.AdapterRAM -gt 0 } | \
                  Select-Object -ExpandProperty Name) -join ' + '";

    match shell::powershell(script) {
        Ok(output) if output.success && !output.stdout.trim().is_empty() => {
            output.stdout.trim().to_string()
        }
        _ => "placa de vídeo não identificada".to_string(),
    }
}

/// Descobre se o disco do sistema é SSD ou mecânico.
///
/// `MediaType` do PowerShell devolve as constantes "SSD" e "HDD" em qualquer
/// idioma do Windows — ao contrário do texto de quase todo comando do sistema.
fn detect_system_storage() -> StorageKind {
    // O rápido primeiro; o antigo continua aqui como reserva para a máquina
    // onde a consulta direta não responder.
    #[cfg(target_os = "windows")]
    if let Some(conhecido) = detect_system_storage_rapido() {
        return conhecido;
    }

    let script = "$n = (Get-Partition -DriveLetter C | Get-Disk).Number; \
                  (Get-PhysicalDisk | Where-Object DeviceId -eq $n).MediaType";

    let output = match shell::powershell(script) {
        Ok(output) if output.success => output.stdout,
        _ => return StorageKind::Unknown,
    };

    parse_media_type(&output)
}

/// O mesmo dado, sem carregar o módulo `Storage` do PowerShell.
///
/// Medido nesta máquina: **79 ms contra 1571 a 3581 ms** do caminho acima. A
/// diferença não está na consulta, está em carregar o módulo — o caminho antigo
/// aciona três cmdlets dele numa cadeia, e o custo varia tanto entre execuções
/// que dá para ver o carregamento acontecendo.
///
/// Consulta as MESMAS classes que os cmdlets consultariam, direto pelo WMI, e
/// aponta para o disco do SISTEMA — não para o primeiro da lista, que numa
/// máquina com pendrive espetado seria o pendrive.
///
/// Devolve `None` quando não conseguir responder, e aí o caminho antigo tenta.
/// Preferir o rápido e cair no lento é diferente de trocar um pelo outro: numa
/// máquina onde a consulta direta falhe, a resposta continua sendo a de antes.
#[cfg(target_os = "windows")]
fn detect_system_storage_rapido() -> Option<StorageKind> {
    let script = "$ns = 'root\\Microsoft\\Windows\\Storage'; \
                  $n = (Get-CimInstance -Namespace $ns -ClassName MSFT_Partition | \
                        Where-Object DriveLetter -eq 'C').DiskNumber; \
                  (Get-CimInstance -Namespace $ns -ClassName MSFT_PhysicalDisk | \
                   Where-Object DeviceId -eq \"$n\").MediaType";

    let saida = shell::powershell(script).ok()?;

    if !saida.success {
        return None;
    }

    match parse_media_type(&saida.stdout) {
        StorageKind::Unknown => None,
        conhecido => Some(conhecido),
    }
}

/// Exposto para teste: o parsing é a parte que pode quebrar em máquinas atípicas.
pub fn parse_media_type(output: &str) -> StorageKind {
    let value = output.trim().to_uppercase();

    // A consulta rápida devolve o `MediaType` cru da classe `MSFT_PhysicalDisk`,
    // que é um número — o "SSD" que aparece na tela é formatação do PowerShell.
    // O mapa é o da própria classe: 3 = HDD, 4 = SSD. Qualquer outro número é
    // desconhecido, e desconhecido não vira palpite.
    match value.as_str() {
        "3" => return StorageKind::Hdd,
        "4" => return StorageKind::Ssd,
        _ => {}
    }

    if value.contains("SSD") {
        StorageKind::Ssd
    } else if value.contains("HDD") {
        StorageKind::Hdd
    } else {
        // "Unspecified" é comum em NVMe atrás de certos controladores e em
        // máquinas virtuais. Fingir que é SSD seria um palpite; não é.
        StorageKind::Unknown
    }
}

#[cfg(test)]
mod tests_1_7 {
    use super::*;

    /// O nome do processador não precisa de amostragem de uso.
    ///
    /// `refresh_cpu_all()` custa ~960 ms medidos nesta máquina porque, para
    /// reportar PERCENTUAL de uso, o `sysinfo` precisa de duas amostras
    /// separadas por um intervalo. O perfil lê só `cpu.brand()` — uma string
    /// que já vem na primeira leitura. Era quase um segundo, na abertura do
    /// programa, pago por nada.
    ///
    /// Este teste prova que a leitura barata devolve o MESMO nome que a cara.
    /// Sem ele, trocar a chamada seria trocar um custo por um risco.
    #[test]
    fn o_nome_da_cpu_e_o_mesmo_sem_amostrar_uso() {
        use sysinfo::{CpuRefreshKind, System};

        let caro = {
            let mut s = System::new();
            s.refresh_cpu_all();
            s.cpus().first().map(|c| c.brand().trim().to_string())
        };

        let barato = {
            let mut s = System::new();
            s.refresh_cpu_specifics(CpuRefreshKind::nothing());
            s.cpus().first().map(|c| c.brand().trim().to_string())
        };

        assert_eq!(
            caro, barato,
            "a leitura barata mudou o nome do processador — a troca não é segura"
        );
    }

    /// O número que a classe do WMI devolve, e a armadilha que ele esconde.
    ///
    /// O caminho rápido consulta `MSFT_PhysicalDisk` direto, sem o módulo
    /// `Storage` do PowerShell — 79 ms contra 1571 a 3581 do caminho antigo,
    /// medido nesta máquina. Só que `MediaType` ali é um `UInt16`, não texto:
    /// o "SSD" que aparece na tela é formatação do PowerShell.
    ///
    /// Trocar sem tratar isso faria o parser receber `4`, não achar "SSD",
    /// devolver desconhecido — e o SysMain deixaria de ser oferecido em TODA
    /// máquina com SSD, que é justamente onde ele deve ser oferecido. Um
    /// segundo e meio economizado ao custo de uma otimização que some.
    ///
    /// O mapa é o da classe: 3 = HDD, 4 = SSD.
    #[test]
    fn o_numero_do_wmi_e_traduzido_como_o_texto() {
        assert_eq!(parse_media_type("4"), StorageKind::Ssd);
        assert_eq!(parse_media_type("3"), StorageKind::Hdd);
    }

    #[test]
    fn o_texto_antigo_continua_valendo() {
        // O caminho de reserva ainda devolve texto, e máquinas onde a consulta
        // rápida falhar vão passar por ele.
        assert_eq!(parse_media_type("SSD"), StorageKind::Ssd);
        assert_eq!(parse_media_type("HDD"), StorageKind::Hdd);
        assert_eq!(parse_media_type(" ssd \r\n"), StorageKind::Ssd);
    }

    #[test]
    fn numero_desconhecido_nao_vira_palpite() {
        // 0 é "Unspecified" e aparece em NVMe atrás de certos controladores e
        // em máquina virtual. Fingir que é SSD seria chute — e é sobre este
        // chute que o produto decide oferecer ou não o SysMain.
        assert_eq!(parse_media_type("0"), StorageKind::Unknown);
        assert_eq!(parse_media_type("5"), StorageKind::Unknown);
        assert_eq!(parse_media_type(""), StorageKind::Unknown);
        assert_eq!(parse_media_type("Unspecified"), StorageKind::Unknown);
    }

    /// A trava de sempre: o perfil precisa continuar identificando a máquina.
    #[test]
    fn o_perfil_continua_identificando_esta_maquina() {
        let p = profile();

        assert!(!p.cpu_name.is_empty());
        assert!(p.logical_cores >= 1);
        assert!(p.total_ram_gb > 0.0);
    }
}

#[cfg(test)]
mod medicao_de_tempo {
    //! Os 3,7 s da abertura moram aqui.
    //!
    //! A medição do veredito apontou a prontidão como 58% da tela inicial, mas
    //! a conta era emprestada: a prontidão só é a primeira a chamar
    //! `hardware::profile()`, e paga a detecção inteira. Com o cache quente, a
    //! prontidão inteira leva 307 ms.
    //!
    //! Este teste abre os 3,7 s por etapa, porque consertar sem saber qual
    //! delas custa seria o chute que o produto recusa.
    //!
    //! `cargo test --lib -- --ignored --nocapture onde_vai_o_tempo_do_perfil`

    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn onde_vai_o_tempo_do_perfil() {
        let cronometrar = |nome: &str, f: &dyn Fn()| {
            let inicio = Instant::now();
            f();
            println!("  {:<26} {:>6} ms", nome, inicio.elapsed().as_millis());
        };

        println!("\nETAPAS DO PERFIL DE HARDWARE\n");

        cronometrar("detect_system_storage", &|| {
            let _ = detect_system_storage();
        });

        cronometrar("detect_gpu", &|| {
            let _ = detect_gpu();
        });

        cronometrar("sysinfo: memória", &|| {
            let mut s = sysinfo::System::new();
            s.refresh_memory();
        });

        // Os dois lado a lado, porque medir só um não prova nada: a primeira
        // versão deste teste continuou cronometrando `refresh_cpu_all()`
        // depois de a produção já ter trocado, e o número não se mexeu —
        // parecia conserto que não funcionou, quando era medidor apontado para
        // o código velho.
        cronometrar("cpu: refresh_cpu_all (antigo)", &|| {
            let mut s = sysinfo::System::new();
            s.refresh_cpu_all();
        });

        cronometrar("cpu: seletivo (em uso)", &|| {
            let mut s = sysinfo::System::new();
            s.refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing());
        });

        cronometrar("num_cpus", &|| {
            let _ = num_cpus::get();
        });

        let inicio = Instant::now();
        let p = profile();
        println!(
            "\n  profile() com cache quente {:>6} ms",
            inicio.elapsed().as_millis()
        );
        println!("  ({} · {:.1} GB · {} núcleos)\n", p.cpu_name, p.total_ram_gb, p.logical_cores);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_media_type_constants() {
        assert_eq!(parse_media_type("SSD\r\n"), StorageKind::Ssd);
        assert_eq!(parse_media_type("HDD\r\n"), StorageKind::Hdd);
    }

    #[test]
    fn unknown_media_type_is_not_guessed_as_ssd() {
        assert_eq!(parse_media_type("Unspecified\r\n"), StorageKind::Unknown);
        assert_eq!(parse_media_type(""), StorageKind::Unknown);
    }

    #[test]
    fn detects_this_machine() {
        let profile = profile();
        println!("{:?}", profile);

        assert!(profile.total_ram_gb > 0.5);
        assert!(profile.logical_cores >= 1);
    }
}
