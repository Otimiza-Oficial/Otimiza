// Analisador de gargalo
//
// A pergunta que todo cliente faz e que nenhum otimizador responde: "por que
// meu FPS é baixo?". A resposta honesta quase nunca é "falta otimizar" — é uma
// peça específica que chegou no limite enquanto as outras estão sobrando.
//
// Este módulo mede as cinco durante alguns segundos e diz qual travou. Ele não
// otimiza nada: só explica. E explicar é o que permite a próxima decisão ser
// certa, seja ela mexer numa configuração, trocar uma peça, ou não fazer nada.
//
// O CASO QUE MAIS APARECE, E QUE QUASE NINGUÉM SABE EXPLICAR
//
// O FiveM, como o GTA V, depende muito de um único núcleo. Numa máquina de oito
// núcleos é comum ver um deles a 100% e os outros sete quase parados, com a
// placa de vídeo a 40%. O dono olha o Gerenciador de Tarefas, vê "CPU 25%",
// conclui que sobra máquina, e não entende por que trava.
//
// Sobra máquina — só que na parte errada. Comprar mais núcleos não resolve
// nada; um processador com núcleo mais rápido resolve. Essa frase, dita com o
// número na mão, vale mais que qualquer ajuste que este programa possa aplicar.
//
// A REGRA QUE IMPEDE O CHUTE
//
// Sem carga não há gargalo a encontrar. Uma máquina parada não tem limite
// nenhum, e apontar um seria inventar. Quando nada está perto do limite, o
// veredito é "não identificamos", nunca o palpite mais vendável.

use super::shell;
use serde::{Deserialize, Serialize};

/// A partir de quanto um recurso é considerado no limite.
///
/// Não é 100 porque nenhum contador fica cravado no topo: oscila. E não é
/// menos que isto porque um recurso a 80% ainda tem folga — chamar aquilo de
/// gargalo mandaria o cliente trocar peça à toa.
pub const SATURADO: f64 = 92.0;

/// Abaixo disto o recurso está claramente sobrando.
///
/// Serve para o contraste: um núcleo a 99% ao lado de placa de vídeo a 35% é
/// uma história diferente de tudo a 95%.
pub const FOLGADO: f64 = 60.0;

/// Carga mínima para valer a pena julgar.
pub const CARGA_MINIMA: f64 = 15.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Limite {
    /// Um núcleo no talo e os outros sobrando. O caso clássico de jogo.
    CpuUmNucleo,
    /// Todos os núcleos no limite.
    CpuTodos,
    /// A placa de vídeo é o teto. Em jogo isso costuma ser boa notícia.
    Gpu,
    /// A memória de vídeo acabou.
    MemoriaVideo,
    /// A memória do sistema acabou.
    MemoriaRam,
    /// O disco não dá conta.
    Disco,
    /// Nada está perto do limite.
    NaoIdentificado,
    /// A máquina está parada; não há o que julgar.
    SemCarga,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckReport {
    pub limite: Limite,
    pub summary: String,
    pub advice: String,
    /// Média do uso total do processador na janela medida.
    pub cpu_total: f64,
    /// O núcleo mais carregado. É este que denuncia o gargalo de um núcleo só.
    pub cpu_max_core: f64,
    pub gpu_percent: f64,
    pub vram_used_mb: f64,
    /// `None` quando não foi possível ler o total da placa.
    pub vram_total_mb: Option<f64>,
    pub ram_available_gb: f64,
    pub ram_total_gb: f64,
    pub disk_percent: f64,
    pub samples: usize,
    pub seconds: f64,
}

// ------------------------------------------------------------ leituras

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawContadores {
    gpu: Option<f64>,
    vram_mb: Option<f64>,
    disco: Option<f64>,
}

/// Uma amostra dos contadores que não vêm do sysinfo.
///
/// Placa de vídeo e disco em uma só consulta: cada chamada ao WMI custa mais de
/// um segundo, e duas por amostra dobrariam o tempo da análise.
fn amostrar_wmi() -> RawContadores {
    // O uso da placa é somado sobre os motores 3D. Uma placa expõe vários
    // motores — 3D, cópia, vídeo — e só o 3D representa o que um jogo pede.
    let script = "\
        $g = @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine \
               -ErrorAction SilentlyContinue | Where-Object Name -like '*engtype_3D*'); \
        $m = @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUAdapterMemory \
               -ErrorAction SilentlyContinue); \
        $d = Get-CimInstance Win32_PerfFormattedData_PerfDisk_PhysicalDisk \
             -ErrorAction SilentlyContinue | Where-Object Name -eq '_Total'; \
        ConvertTo-Json -Compress -InputObject ([ordered]@{ \
          Gpu = [math]::Min(100, ($g | Measure-Object UtilizationPercentage -Sum).Sum); \
          VramMb = [math]::Round((($m | Measure-Object DedicatedUsage -Maximum).Maximum)/1MB, 0); \
          Disco = [math]::Min(100, $d.PercentDiskTime) })";

    shell::powershell(script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default()
}

/// Memória total da placa de vídeo, lida do registro.
///
/// `Win32_VideoController.AdapterRAM` é um campo de 32 bits e estoura em placa
/// de 4 GB ou mais, devolvendo número errado. O valor real fica no registro,
/// gravado pelo driver, e não depende de idioma.
fn vram_total_mb() -> Option<f64> {
    let base = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";

    for indice in 0..8 {
        let caminho = format!("{}\\{:04}", base, indice);

        if let Ok(crate::modules::changelog::PreviousValue::Binary(bytes)) =
            super::registry::read("HKLM", &caminho, "HardwareInformation.qwMemorySize")
        {
            if bytes.len() >= 8 {
                let mut oito = [0u8; 8];
                oito.copy_from_slice(&bytes[..8]);
                let total = u64::from_le_bytes(oito);

                if total > 0 {
                    return Some(total as f64 / 1_048_576.0);
                }
            }
        }
    }

    None
}

// ------------------------------------------------------------- veredito

/// Decide qual recurso é o limite.
///
/// A ordem importa: memória esgotada explica tudo o mais que aparece junto, e
/// por isso vem antes. Um núcleo saturado só é diagnóstico quando os outros
/// estão sobrando — senão é só a máquina trabalhando.
pub fn decidir(
    cpu_total: f64,
    cpu_max_core: f64,
    gpu: f64,
    vram_usada: f64,
    vram_total: Option<f64>,
    ram_livre_gb: f64,
    disco: f64,
) -> Limite {
    // Sem carga nenhuma não existe gargalo. Julgar aqui seria inventar.
    if cpu_total < CARGA_MINIMA && gpu < CARGA_MINIMA && disco < CARGA_MINIMA {
        return Limite::SemCarga;
    }

    // Memória do sistema no fim: explica travada, disco alto e tudo o mais que
    // aparecer junto, então é o primeiro a ser considerado.
    if ram_livre_gb < 0.5 {
        return Limite::MemoriaRam;
    }

    // Memória de vídeo no fim: causa engasgo severo e não aparece em nenhum
    // outro indicador.
    if let Some(total) = vram_total {
        if total > 0.0 && vram_usada / total * 100.0 >= 95.0 {
            return Limite::MemoriaVideo;
        }
    }

    if disco >= SATURADO {
        return Limite::Disco;
    }

    if gpu >= SATURADO {
        return Limite::Gpu;
    }

    // Um núcleo no talo com o conjunto folgado: a assinatura do jogo preso em
    // uma linha de execução só.
    if cpu_max_core >= SATURADO && cpu_total < FOLGADO {
        return Limite::CpuUmNucleo;
    }

    if cpu_total >= SATURADO {
        return Limite::CpuTodos;
    }

    Limite::NaoIdentificado
}

/// Texto para a tela.
pub fn explicar(limite: Limite, cpu_total: f64, cpu_max_core: f64, gpu: f64) -> (String, String) {
    match limite {
        Limite::SemCarga => (
            "A máquina está praticamente parada.".to_string(),
            "Para descobrir o que limita o desempenho é preciso medir com o jogo aberto e \
             rodando de verdade. Com o PC ocioso não há limite nenhum a encontrar, e apontar \
             um seria chute."
                .to_string(),
        ),

        Limite::CpuUmNucleo => (
            format!(
                "O processador é o limite, e por um núcleo só: o mais carregado está em \
                 {:.0}% enquanto o conjunto fica em {:.0}%.",
                cpu_max_core, cpu_total
            ),
            "Esta é a situação mais comum em FiveM e GTA V, e a mais mal interpretada. O \
             Gerenciador de Tarefas mostra a média — algo como 25% — e dá a impressão de que \
             sobra máquina. Sobra, mas na parte errada: o jogo depende de um núcleo e esse \
             está no talo.\n\nO que muda isso é um processador com núcleo mais rápido, não um \
             com mais núcleos. Placa de vídeo melhor também não resolve, porque ela já está \
             esperando. É uma informação cara de descobrir errado."
                .to_string(),
        ),

        Limite::CpuTodos => (
            format!("O processador é o limite: todos os núcleos em {:.0}%.", cpu_total),
            "Diferente do caso de um núcleo só, aqui a máquina está inteira ocupada. Vale \
             conferir na aba Painel se algum programa em segundo plano está consumindo o que \
             deveria ir para o jogo, antes de pensar em trocar peça."
                .to_string(),
        ),

        Limite::Gpu => (
            format!("A placa de vídeo é o limite, trabalhando a {:.0}%.", gpu),
            "Em jogo, isso costuma ser boa notícia: significa que o resto da máquina está \
             entregando tudo que a placa consegue consumir, e nenhum ajuste de sistema vai \
             além disso. Para mais quadros, o caminho é baixar as configurações gráficas ou \
             a resolução — ou trocar a placa."
                .to_string(),
        ),

        Limite::MemoriaVideo => (
            "A memória da placa de vídeo acabou.".to_string(),
            "Quando ela enche, a placa passa a buscar dados na memória do sistema, que é bem \
             mais lenta, e o resultado são engasgos fortes e irregulares. Reduzir a qualidade \
             das texturas costuma resolver na hora, e é de graça."
                .to_string(),
        ),

        Limite::MemoriaRam => (
            "A memória do sistema acabou.".to_string(),
            "Com a memória no fim o Windows passa a usar o disco no lugar dela, e disco é \
             ordens de grandeza mais lento. É a causa mais comum de travadas de vários \
             segundos. Feche o que não estiver usando — a aba Painel mostra quem está \
             consumindo — e considere aumentar a memória, que costuma ser a peça mais barata \
             com maior efeito."
                .to_string(),
        ),

        Limite::Disco => (
            "O disco é o limite.".to_string(),
            "O disco estar ocupado durante o jogo aponta para carregamento constante ou para \
             algo trabalhando em segundo plano. Vale conferir a saúde do disco na aba \
             Diagnóstico: disco mecânico ou com desgaste avançado se comporta assim, e nesse \
             caso nenhum ajuste de software resolve."
                .to_string(),
        ),

        Limite::NaoIdentificado => (
            "Nenhum recurso chegou perto do limite na janela medida.".to_string(),
            "Isso significa que, no momento da medição, nada estava segurando a máquina. Se \
             você sente travadas, elas podem ser curtas demais para aparecer numa média — a \
             aba Resultado mede engasgos, que é a medida certa para esse caso. Preferimos \
             dizer que não encontramos a apontar um culpado provável."
                .to_string(),
        ),
    }
}

/// Mede por alguns segundos e diz o que está limitando.
///
/// Bloqueia. Deve ser chamada fora do runtime assíncrono.
pub fn analisar(segundos: u64) -> BottleneckReport {
    use sysinfo::System;

    let inicio = std::time::Instant::now();
    let mut sistema = System::new_all();

    let mut cpu_totais: Vec<f64> = Vec::new();
    let mut cpu_maximos: Vec<f64> = Vec::new();
    let mut gpus: Vec<f64> = Vec::new();
    let mut discos: Vec<f64> = Vec::new();
    let mut vram_max = 0.0f64;
    let mut ram_livre_min = f64::MAX;

    let vram_total = vram_total_mb();
    let ram_total_gb = sistema.total_memory() as f64 / 1_073_741_824.0;

    // Cada volta custa mais de um segundo por causa do WMI, então o número de
    // amostras acompanha o tempo pedido em vez de ser fixo.
    while inicio.elapsed().as_secs() < segundos {
        sistema.refresh_cpu_all();
        std::thread::sleep(std::time::Duration::from_millis(300));
        sistema.refresh_cpu_all();
        sistema.refresh_memory();

        let por_nucleo: Vec<f64> = sistema.cpus().iter().map(|c| c.cpu_usage() as f64).collect();

        if !por_nucleo.is_empty() {
            cpu_totais.push(por_nucleo.iter().sum::<f64>() / por_nucleo.len() as f64);
            cpu_maximos.push(por_nucleo.iter().cloned().fold(0.0, f64::max));
        }

        ram_livre_min = ram_livre_min.min(sistema.available_memory() as f64 / 1_073_741_824.0);

        let bruto = amostrar_wmi();
        gpus.push(bruto.gpu.unwrap_or(0.0));
        discos.push(bruto.disco.unwrap_or(0.0));
        vram_max = vram_max.max(bruto.vram_mb.unwrap_or(0.0));
    }

    let media = |v: &[f64]| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };

    // Para o núcleo mais carregado vale o pico, e não a média: o gargalo de um
    // núcleo só aparece em rajadas, e a média o esconderia.
    let cpu_max_core = cpu_maximos.iter().cloned().fold(0.0, f64::max);
    let cpu_total = media(&cpu_totais);
    let gpu = media(&gpus);
    let disco = media(&discos);
    let ram_livre = if ram_livre_min == f64::MAX { 0.0 } else { ram_livre_min };

    let limite = decidir(cpu_total, cpu_max_core, gpu, vram_max, vram_total, ram_livre, disco);
    let (summary, advice) = explicar(limite, cpu_total, cpu_max_core, gpu);

    BottleneckReport {
        limite,
        summary,
        advice,
        cpu_total,
        cpu_max_core,
        gpu_percent: gpu,
        vram_used_mb: vram_max,
        vram_total_mb: vram_total,
        ram_available_gb: ram_livre,
        ram_total_gb,
        disk_percent: disco,
        samples: cpu_totais.len(),
        seconds: inicio.elapsed().as_secs_f64(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maquina_parada_nao_recebe_diagnostico() {
        // A regra que impede o chute. Sem carga não existe gargalo, e apontar
        // um seria inventar.
        assert_eq!(
            decidir(3.0, 8.0, 1.0, 500.0, Some(4096.0), 6.0, 0.0),
            Limite::SemCarga
        );

        let (_, conselho) = explicar(Limite::SemCarga, 3.0, 8.0, 1.0);
        assert!(conselho.contains("seria chute"));
    }

    #[test]
    fn um_nucleo_no_talo_com_o_resto_sobrando() {
        // O caso do FiveM: um núcleo a 99%, média em 25%, placa em 40%.
        // O Gerenciador de Tarefas mostra 25% e o dono acha que sobra máquina.
        assert_eq!(
            decidir(25.0, 99.0, 40.0, 1500.0, Some(4096.0), 5.0, 10.0),
            Limite::CpuUmNucleo
        );

        let (resumo, conselho) = explicar(Limite::CpuUmNucleo, 25.0, 99.0, 40.0);
        assert!(resumo.contains("99%") && resumo.contains("25%"));

        // A frase que evita o cliente comprar a peça errada.
        assert!(conselho.contains("núcleo mais rápido, não um"));
        assert!(conselho.contains("Placa de vídeo melhor também não resolve"));
    }

    #[test]
    fn nucleo_alto_com_maquina_cheia_nao_e_gargalo_de_um_nucleo() {
        // Com tudo carregado, um núcleo a 99% é só a máquina trabalhando.
        assert_eq!(
            decidir(95.0, 99.0, 50.0, 1500.0, Some(4096.0), 5.0, 10.0),
            Limite::CpuTodos
        );
    }

    #[test]
    fn memoria_no_fim_vem_antes_de_tudo() {
        // Memória esgotada explica disco alto e travada; se ela for julgada
        // depois, o relatório apontaria o disco e mandaria trocar a peça errada.
        assert_eq!(
            decidir(95.0, 99.0, 99.0, 4000.0, Some(4096.0), 0.2, 99.0),
            Limite::MemoriaRam
        );
    }

    #[test]
    fn placa_no_limite_e_boa_noticia_em_jogo() {
        assert_eq!(
            decidir(40.0, 60.0, 98.0, 2000.0, Some(4096.0), 5.0, 10.0),
            Limite::Gpu
        );

        let (_, conselho) = explicar(Limite::Gpu, 40.0, 60.0, 98.0);
        assert!(conselho.contains("boa notícia"));
    }

    #[test]
    fn memoria_de_video_cheia_e_detectada() {
        assert_eq!(
            decidir(40.0, 60.0, 70.0, 3980.0, Some(4096.0), 5.0, 10.0),
            Limite::MemoriaVideo
        );

        // Sem saber o total da placa, não dá para dizer que ela encheu.
        assert_ne!(
            decidir(40.0, 60.0, 70.0, 3980.0, None, 5.0, 10.0),
            Limite::MemoriaVideo
        );
    }

    #[test]
    fn nada_saturado_admite_que_nao_encontrou() {
        assert_eq!(
            decidir(50.0, 70.0, 55.0, 1000.0, Some(4096.0), 5.0, 30.0),
            Limite::NaoIdentificado
        );

        let (_, conselho) = explicar(Limite::NaoIdentificado, 50.0, 70.0, 55.0);
        assert!(conselho.contains("Preferimos dizer que não encontramos"));
    }

    #[test]
    fn memoria_de_video_total_desta_maquina() {
        match vram_total_mb() {
            Some(mb) => {
                println!("memória de vídeo: {:.0} MB", mb);
                // Uma placa com menos de 128 MB ou mais de 128 GB seria erro
                // de leitura, não hardware.
                assert!(mb >= 128.0 && mb <= 131_072.0, "valor implausível: {}", mb);
            }
            None => println!("não foi possível ler o total da placa"),
        }
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analisar(4);

        println!("limite: {:?}", r.limite);
        println!("resumo: {}", r.summary);
        println!(
            "  cpu {:.0}% (pico de núcleo {:.0}%) | gpu {:.0}% | disco {:.0}%",
            r.cpu_total, r.cpu_max_core, r.gpu_percent, r.disk_percent
        );
        println!(
            "  vram {:.0} MB de {:?} | ram livre {:.1} de {:.1} GB | {} amostras em {:.1} s",
            r.vram_used_mb, r.vram_total_mb, r.ram_available_gb, r.ram_total_gb, r.samples, r.seconds
        );

        assert!(!r.summary.is_empty());
        assert!(r.samples > 0, "nenhuma amostra coletada");

        // O pico de um núcleo nunca pode ser menor que a média de todos.
        assert!(
            r.cpu_max_core >= r.cpu_total - 0.5,
            "pico {} menor que a média {}",
            r.cpu_max_core,
            r.cpu_total
        );

        // Percentuais dentro da faixa: contador estourado viraria diagnóstico
        // falso.
        for (nome, valor) in [
            ("cpu", r.cpu_total),
            ("núcleo", r.cpu_max_core),
            ("gpu", r.gpu_percent),
            ("disco", r.disk_percent),
        ] {
            assert!(
                (0.0..=100.0).contains(&valor),
                "{} fora da faixa: {}",
                nome,
                valor
            );
        }
    }
}
