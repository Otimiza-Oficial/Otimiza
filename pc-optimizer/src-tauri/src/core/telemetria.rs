// Telemetria: UMA coleta barata (PDH em processo, sem PowerShell) que todos os módulos usam. Tudo contador do
// Windows: CPU total e o núcleo mais ocupado (CPU a 40% com um núcleo a 100% é jogo preso num núcleo), clock
// efetivo derivado, GPU somada por motor como no Gerenciador de Tarefas, VRAM, RAM, commit, paginação e disco.
// Temperatura e potência do processador exigem driver de kernel, que o Otimiza não instala: ficam UNKNOWN,
// nunca zero.

use serde::{Deserialize, Serialize};

/// `None` = o contador não existe ou não respondeu nesta máquina, e a tela diz isso.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Amostra {
    pub instante_ms: u64,
    pub cpu_total_pct: Option<f64>,
    pub cpu_nucleo_max_pct: Option<f64>,
    pub nucleos: Option<u32>,
    pub clock_efetivo_mhz: Option<f64>,
    pub clock_nominal_mhz: Option<f64>,
    pub gpu_pct: Option<f64>,
    pub gpu_3d_pct: Option<f64>,
    pub vram_usada_mb: Option<f64>,
    pub vram_compartilhada_mb: Option<f64>,
    pub ram_disponivel_mb: Option<f64>,
    pub commit_pct: Option<f64>,
    pub paginas_lidas_s: Option<f64>,
    pub disco_latencia_ms: Option<f64>,
    pub disco_fila: Option<f64>,
    pub disco_ocupado_pct: Option<f64>,
    /// Os 3 programas que mais usavam processador (fora o jogo e o Otimiza): é o que o detetive de travadas usa.
    #[serde(default)]
    pub processos: Vec<ProcessoNaAmostra>,
    #[serde(default, skip_serializing)]
    pub nucleos_pct: Vec<f64>,
    /// Bit 0 térmico, bit 1 energia.
    #[serde(default)]
    pub limite_flags: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProcessoNaAmostra {
    pub nome: String,
    pub cpu: f64,
}

/// Ex.: `pid_1234_luid_0x00000000_0x0000D1B5_phys_0_eng_3_engtype_3D`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotorDeGpu {
    pub pid: Option<u32>,
    pub luid: String,
    pub tipo: String,
}

pub fn ler_instancia_de_motor(nome: &str) -> Option<MotorDeGpu> {
    let minusculo = nome.to_ascii_lowercase();
    let luid_ini = minusculo.find("luid_")?;
    let resto = &minusculo[luid_ini + 5..];
    let partes: Vec<&str> = resto.splitn(3, '_').collect();
    if partes.len() < 2 || !partes[0].starts_with("0x") || !partes[1].starts_with("0x") {
        return None;
    }
    let luid = format!("{}_{}", partes[0], partes[1]);
    let tipo = minusculo.rsplit_once("engtype_")?.1.to_string();
    let pid = minusculo
        .strip_prefix("pid_")
        .and_then(|r| r.split('_').next())
        .and_then(|n| n.parse().ok());
    Some(MotorDeGpu { pid, luid, tipo })
}

/// Soma cada tipo de motor entre processos e fica com o mais ocupado.
pub fn uso_por_placa(instancias: &[(String, f64)]) -> Vec<(String, f64, f64)> {
    use std::collections::BTreeMap;
    let mut por_placa: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    for (nome, v) in instancias {
        if let Some(m) = ler_instancia_de_motor(nome) {
            *por_placa.entry(m.luid).or_default().entry(m.tipo).or_default() += v.max(0.0);
        }
    }
    por_placa
        .into_iter()
        .map(|(luid, tipos)| {
            let maior = tipos.values().copied().fold(0.0_f64, f64::max).min(100.0);
            let tres_d = tipos.get("3d").copied().unwrap_or(0.0).min(100.0);
            (luid, maior, tres_d)
        })
        .collect()
}

pub fn memoria_por_placa(instancias: &[(String, f64)]) -> Vec<(String, f64)> {
    instancias
        .iter()
        .filter_map(|(nome, bytes)| {
            let minusculo = nome.to_ascii_lowercase();
            let ini = minusculo.find("luid_")?;
            let partes: Vec<&str> = minusculo[ini + 5..].splitn(3, '_').collect();
            (partes.len() >= 2).then(|| (format!("{}_{}", partes[0], partes[1]), bytes / 1_048_576.0))
        })
        .collect()
}

pub fn luid_como_no_pdh(alto: i32, baixo: u32) -> String {
    format!("0x{:08x}_0x{:08x}", alto as u32, baixo)
}

pub fn so_nucleos(instancias: &[(String, f64)]) -> Vec<f64> {
    instancias
        .iter()
        .filter(|(n, _)| !n.contains("_Total"))
        .map(|(_, v)| v.clamp(0.0, 100.0))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placa {
    pub nome: String,
    pub luid: String,
    pub vram_total_mb: f64,
}

/// A de mais memória dedicada primeiro (em notebook, a dedicada). Adaptador de software fica de fora.
#[cfg(windows)]
pub fn placas() -> Vec<Placa> {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};

    let Ok(fabrica) = (unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }) else {
        return Vec::new();
    };
    let mut v = Vec::new();
    let mut i = 0;
    while let Ok(adaptador) = unsafe { fabrica.EnumAdapters1(i) } {
        i += 1;
        let Ok(d) = (unsafe { adaptador.GetDesc1() }) else { continue };
        if d.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        let fim = d.Description.iter().position(|c| *c == 0).unwrap_or(d.Description.len());
        v.push(Placa {
            nome: String::from_utf16_lossy(&d.Description[..fim]).trim().to_string(),
            luid: luid_como_no_pdh(d.AdapterLuid.HighPart, d.AdapterLuid.LowPart),
            vram_total_mb: d.DedicatedVideoMemory as f64 / 1_048_576.0,
        });
    }
    v.sort_by(|a, b| b.vram_total_mb.total_cmp(&a.vram_total_mb));
    v
}

/// Pelo DXGI (`CheckInterfaceSupport`), sem PowerShell.
#[cfg(windows)]
pub fn versao_do_driver() -> Option<String> {
    use windows::core::Interface;
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIDevice, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};
    let fabrica: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.ok()?;
    let mut melhor: Option<(u64, String)> = None;
    let mut i = 0;
    while let Ok(a) = unsafe { fabrica.EnumAdapters1(i) } {
        i += 1;
        let Ok(d) = (unsafe { a.GetDesc1() }) else { continue };
        if d.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        let Ok(v) = (unsafe { a.CheckInterfaceSupport(&IDXGIDevice::IID) }) else { continue };
        let v = v as u64;
        let texto = format!("{}.{}.{}.{}", v >> 48, (v >> 32) & 0xFFFF, (v >> 16) & 0xFFFF, v & 0xFFFF);
        if melhor.as_ref().is_none_or(|(mem, _)| d.DedicatedVideoMemory as u64 > *mem) {
            melhor = Some((d.DedicatedVideoMemory as u64, texto));
        }
    }
    melhor.map(|(_, t)| t)
}

#[cfg(windows)]
pub fn build_do_windows() -> Option<String> {
    use crate::modules::changelog::PreviousValue;
    use crate::modules::windows::registry;
    const CHAVE: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let build = registry::read_text("HKLM", CHAVE, "CurrentBuildNumber").ok().flatten()?;
    Some(match registry::read("HKLM", CHAVE, "UBR") {
        Ok(PreviousValue::Dword(ubr)) => format!("{}.{}", build, ubr),
        _ => build,
    })
}

#[cfg(windows)]
pub struct Coletor {
    q: super::pdh::Consulta,
    inicio: std::time::Instant,
    placa: Option<Placa>,
    cpu_total: super::pdh::Contador,
    cpu_nucleos: super::pdh::Contador,
    desempenho: super::pdh::Contador,
    frequencia: super::pdh::Contador,
    gpu_motores: super::pdh::Contador,
    vram_dedicada: super::pdh::Contador,
    vram_compartilhada: super::pdh::Contador,
    ram_disponivel: super::pdh::Contador,
    commit: super::pdh::Contador,
    paginas: super::pdh::Contador,
    disco_latencia: super::pdh::Contador,
    disco_fila: super::pdh::Contador,
    disco_ocioso: super::pdh::Contador,
    limite_flags: super::pdh::Contador,
    processos: Option<(sysinfo::System, Option<u32>)>,
}

#[cfg(windows)]
impl Coletor {
    /// Contadores de taxa só valem a partir da segunda coleta. `None` quando o PDH não abre.
    pub fn novo() -> Option<Self> {
        let q = super::pdh::Consulta::nova()?;
        let c = |p: &str| q.adicionar(p);
        let coletor = Coletor {
            cpu_total: c(r"\Processor Information(_Total)\% Processor Utility"),
            cpu_nucleos: c(r"\Processor Information(*)\% Processor Utility"),
            desempenho: c(r"\Processor Information(_Total)\% Processor Performance"),
            frequencia: c(r"\Processor Information(_Total)\Processor Frequency"),
            gpu_motores: c(r"\GPU Engine(*)\Utilization Percentage"),
            vram_dedicada: c(r"\GPU Adapter Memory(*)\Dedicated Usage"),
            vram_compartilhada: c(r"\GPU Adapter Memory(*)\Shared Usage"),
            ram_disponivel: c(r"\Memory\Available MBytes"),
            commit: c(r"\Memory\% Committed Bytes In Use"),
            paginas: c(r"\Memory\Pages Input/sec"),
            disco_latencia: c(r"\PhysicalDisk(_Total)\Avg. Disk sec/Transfer"),
            disco_fila: c(r"\PhysicalDisk(_Total)\Current Disk Queue Length"),
            disco_ocioso: c(r"\PhysicalDisk(_Total)\% Idle Time"),
            limite_flags: c(r"\Processor Information(_Total)\Performance Limit Flags"),
            placa: placas().into_iter().next(),
            processos: None,
            inicio: std::time::Instant::now(),
            q,
        };
        coletor.q.coletar();
        Some(coletor)
    }

    pub fn decorrido_ms(&self) -> u64 {
        self.inicio.elapsed().as_millis() as u64
    }

    pub fn placa(&self) -> Option<&Placa> {
        self.placa.as_ref()
    }

    pub fn acompanhar_processos(&mut self, fora: Option<u32>) {
        let mut s = sysinfo::System::new();
        s.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, sysinfo::ProcessRefreshKind::nothing().with_cpu());
        self.processos = Some((s, fora));
    }

    fn top_processos(&mut self) -> Vec<ProcessoNaAmostra> {
        let Some((s, fora)) = self.processos.as_mut() else { return Vec::new() };
        s.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, sysinfo::ProcessRefreshKind::nothing().with_cpu());
        let nucleos = num_cpus::get().max(1) as f64;
        let eu = std::process::id();
        let mut v: Vec<ProcessoNaAmostra> = s
            .processes()
            .iter()
            .filter(|(pid, _)| Some(pid.as_u32()) != *fora && pid.as_u32() != eu && pid.as_u32() > 4)
            .map(|(_, p)| ProcessoNaAmostra { nome: p.name().to_string_lossy().to_string(), cpu: p.cpu_usage() as f64 / 100.0 / nucleos })
            .filter(|p| p.cpu > 0.0)
            .collect();
        v.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
        v.truncate(3);
        v
    }

    /// Intervalo de pelo menos ~250 ms entre leituras.
    pub fn amostra(&mut self) -> Amostra {
        let processos = self.top_processos();
        let q = &self.q;
        q.coletar();

        let nucleos = so_nucleos(&q.lista(self.cpu_nucleos));
        let desempenho = q.valor(self.desempenho);
        let frequencia = q.valor(self.frequencia).filter(|f| *f > 0.0);

        let gpus = uso_por_placa(&q.lista(self.gpu_motores));
        let gpu = match &self.placa {
            Some(p) => gpus.iter().find(|(l, _, _)| *l == p.luid).cloned(),
            None => gpus.iter().cloned().max_by(|a, b| a.1.total_cmp(&b.1)),
        };
        let luid_da_gpu = gpu.as_ref().map(|g| g.0.clone()).or_else(|| self.placa.as_ref().map(|p| p.luid.clone()));
        let da_placa = |lista: Vec<(String, f64)>| -> Option<f64> {
            let luid = luid_da_gpu.as_ref()?;
            memoria_por_placa(&lista).into_iter().filter(|(l, _)| l == luid).map(|(_, mb)| mb).reduce(f64::max)
        };

        Amostra {
            instante_ms: self.inicio.elapsed().as_millis() as u64,
            cpu_total_pct: q.valor(self.cpu_total).map(|v| v.clamp(0.0, 100.0)),
            cpu_nucleo_max_pct: nucleos.iter().copied().reduce(f64::max),
            nucleos: (!nucleos.is_empty()).then_some(nucleos.len() as u32),
            clock_efetivo_mhz: match (frequencia, desempenho) {
                (Some(f), Some(d)) => Some(f * d / 100.0),
                _ => None,
            },
            clock_nominal_mhz: frequencia,
            gpu_pct: gpu.as_ref().map(|g| g.1),
            gpu_3d_pct: gpu.as_ref().map(|g| g.2),
            vram_usada_mb: da_placa(q.lista(self.vram_dedicada)),
            vram_compartilhada_mb: da_placa(q.lista(self.vram_compartilhada)),
            ram_disponivel_mb: q.valor(self.ram_disponivel),
            commit_pct: q.valor(self.commit),
            paginas_lidas_s: q.valor(self.paginas),
            disco_latencia_ms: q.valor(self.disco_latencia).map(|s| s * 1000.0),
            disco_fila: q.valor(self.disco_fila),
            disco_ocupado_pct: q.valor(self.disco_ocioso).map(|o| (100.0 - o).clamp(0.0, 100.0)),
            processos,
            limite_flags: q.valor(self.limite_flags).map(|f| f as u64),
            nucleos_pct: nucleos,
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_a_instancia_do_motor() {
        let m = ler_instancia_de_motor("pid_1234_luid_0x00000000_0x0000D1B5_phys_0_eng_3_engtype_3D").unwrap();
        assert_eq!(m.pid, Some(1234));
        assert_eq!(m.luid, "0x00000000_0x0000d1b5");
        assert_eq!(m.tipo, "3d");
        assert!(ler_instancia_de_motor("_Total").is_none());
    }

    #[test]
    fn uso_da_gpu_e_o_motor_mais_ocupado_somado_entre_processos() {
        let placa = "luid_0x00000000_0x0000D1B5_phys_0";
        let outra = "luid_0x00000000_0x00001111_phys_0";
        let inst = vec![
            (format!("pid_10_{}_eng_0_engtype_3D", placa), 60.0),
            (format!("pid_20_{}_eng_0_engtype_3D", placa), 25.0),
            (format!("pid_10_{}_eng_5_engtype_VideoDecode", placa), 30.0),
            (format!("pid_10_{}_eng_0_engtype_3D", outra), 5.0),
        ];
        let v = uso_por_placa(&inst);
        let d1b5 = v.iter().find(|(l, _, _)| l.ends_with("d1b5")).unwrap();
        assert_eq!(d1b5.1, 85.0);
        assert_eq!(d1b5.2, 85.0);
        let outra = v.iter().find(|(l, _, _)| l.ends_with("1111")).unwrap();
        assert_eq!(outra.1, 5.0);
    }

    #[test]
    fn soma_acima_de_cem_nao_passa_de_cem() {
        let inst = vec![
            ("pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D".to_string(), 70.0),
            ("pid_2_luid_0x0_0x1_phys_0_eng_0_engtype_3D".to_string(), 70.0),
        ];
        assert_eq!(uso_por_placa(&inst)[0].1, 100.0);
    }

    #[test]
    fn luid_do_dxgi_bate_com_o_do_pdh() {
        let l = luid_como_no_pdh(0, 0xD1B5);
        let m = ler_instancia_de_motor("pid_1_luid_0x00000000_0x0000D1B5_phys_0_eng_0_engtype_3D").unwrap();
        assert_eq!(l, m.luid);
        let mem = memoria_por_placa(&[("luid_0x00000000_0x0000D1B5_phys_0".to_string(), 2_097_152.0)]);
        assert_eq!(mem, vec![(l, 2.0)]);
    }

    #[test]
    fn nucleos_sem_os_totais() {
        let inst = vec![
            ("0,0".to_string(), 30.0),
            ("0,1".to_string(), 100.0),
            ("0,_Total".to_string(), 65.0),
            ("_Total".to_string(), 65.0),
        ];
        assert_eq!(so_nucleos(&inst), vec![30.0, 100.0]);
    }

    #[test]
    #[ignore = "lê os contadores desta máquina"]
    fn amostra_desta_maquina() {
        let mut c = Coletor::novo().expect("PDH");
        std::thread::sleep(std::time::Duration::from_millis(600));
        let a = c.amostra();
        println!("{:?}\n{:#?}", c.placa(), a);
        println!("driver {:?} · windows {:?}", versao_do_driver(), build_do_windows());
        assert!(a.cpu_total_pct.is_some());
    }
}
