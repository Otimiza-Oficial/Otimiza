// Placa e disco pelos contadores do Windows (PDH), sem abrir processo: pode rodar ao lado da medição de quadros
// sem pesar nela, o que o WMI por PowerShell (mais de um segundo) não pode. Contador que não responde chega como
// `None`, nunca 0. Nomes em inglês (`PdhAddEnglishCounterW`): os caminhos são traduzidos no Windows em português.

#![cfg(target_os = "windows")]

use serde::{Deserialize, Serialize};

const PDH_OK: u32 = 0;

fn largo(texto: &str) -> Vec<u16> {
    texto.encode_utf16().chain(std::iter::once(0)).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Amostra {
    /// Só os motores 3D: somar cópia, vídeo e codec passaria de 100 e não representaria o que o jogo pede.
    pub gpu_pct: Option<f64>,
    /// O MAIOR entre os adaptadores: somar integrada e dedicada daria um total que nenhuma tem.
    pub vram_mb: Option<f64>,
    /// Do MESMO adaptador de `vram_mb`: juntas respondem "a dedicada acabou e começou a derramar para a RAM?".
    pub vram_compartilhada_mb: Option<f64>,
    pub disco_ocupado_pct: Option<f64>,
    /// A LATÊNCIA: disco a 100% com 0,2 ms dá conta; a 40% com 30 ms trava o jogo.
    pub disco_latencia_ms: Option<f64>,
}

pub struct Contadores {
    consulta: isize,
    gpu: isize,
    vram: isize,
    vram_compartilhada: isize,
    disco_tempo: isize,
    disco_latencia: isize,
}

// Alças do Windows, não ponteiros; uma thread por vez.
unsafe impl Send for Contadores {}

impl Contadores {
    /// Sem os contadores de GPU (Windows antes do 10 1709, ou driver que não publica) a leitura sai ausente.
    pub fn novo() -> Option<Self> {
        use windows_sys::Win32::System::Performance::{
            PdhAddEnglishCounterW, PdhCollectQueryData, PdhOpenQueryW,
        };

        let mut consulta: isize = 0;
        if unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut consulta) } != PDH_OK {
            return None;
        }

        let adicionar = |caminho: &str| -> isize {
            let mut h: isize = 0;
            let c = largo(caminho);
            if unsafe { PdhAddEnglishCounterW(consulta, c.as_ptr(), 0, &mut h) } == PDH_OK {
                h
            } else {
                0
            }
        };

        let c = Contadores {
            consulta,
            gpu: adicionar(r"\GPU Engine(*)\Utilization Percentage"),
            vram: adicionar(r"\GPU Adapter Memory(*)\Dedicated Usage"),
            vram_compartilhada: adicionar(r"\GPU Adapter Memory(*)\Shared Usage"),
            disco_tempo: adicionar(r"\PhysicalDisk(_Total)\% Disk Time"),
            disco_latencia: adicionar(r"\PhysicalDisk(_Total)\Avg. Disk sec/Transfer"),
        };

        // Contador de taxa é a diferença entre duas coletas: esta é a base.
        unsafe { PdhCollectQueryData(consulta) };

        Some(c)
    }

    fn valor(&self, contador: isize) -> Option<f64> {
        use windows_sys::Win32::System::Performance::{
            PdhGetFormattedCounterValue, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE,
        };

        if contador == 0 {
            return None;
        }

        let mut v: PDH_FMT_COUNTERVALUE = unsafe { std::mem::zeroed() };
        let r = unsafe {
            PdhGetFormattedCounterValue(contador, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut v)
        };

        if r == PDH_OK {
            Some(unsafe { v.Anonymous.doubleValue })
        } else {
            None
        }
    }

    fn lista(&self, contador: isize) -> Vec<(String, f64)> {
        use windows_sys::Win32::System::Performance::{
            PdhGetFormattedCounterArrayW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
            PDH_MORE_DATA,
        };

        if contador == 0 {
            return Vec::new();
        }

        let mut tamanho: u32 = 0;
        let mut itens: u32 = 0;

        // A primeira chamada DEVE falhar com `MORE_DATA`; qualquer outra resposta é desistir.
        let r = unsafe {
            PdhGetFormattedCounterArrayW(
                contador,
                PDH_FMT_DOUBLE,
                &mut tamanho,
                &mut itens,
                std::ptr::null_mut(),
            )
        };
        if r != PDH_MORE_DATA || tamanho == 0 {
            return Vec::new();
        }

        let item = std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let mut buffer = vec![0u8; tamanho as usize + item];
        let r = unsafe {
            PdhGetFormattedCounterArrayW(
                contador,
                PDH_FMT_DOUBLE,
                &mut tamanho,
                &mut itens,
                buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W,
            )
        };
        if r != PDH_OK {
            return Vec::new();
        }

        let base = buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W;

        (0..itens as usize)
            .filter_map(|i| unsafe {
                let it = &*base.add(i);
                if it.szName.is_null() {
                    return None;
                }
                let mut n = 0;
                while *it.szName.add(n) != 0 {
                    n += 1;
                }
                let nome = String::from_utf16_lossy(std::slice::from_raw_parts(it.szName, n));
                Some((nome, it.FmtValue.Anonymous.doubleValue))
            })
            .collect()
    }

    /// Chamar duas vezes coladas devolve números sem sentido.
    pub fn coletar(&self) -> Amostra {
        use windows_sys::Win32::System::Performance::PdhCollectQueryData;
        unsafe { PdhCollectQueryData(self.consulta) };

        let motores: Vec<f64> = self
            .lista(self.gpu)
            .into_iter()
            .filter(|(nome, _)| nome.ends_with("engtype_3D"))
            .map(|(_, v)| v)
            .collect();

        // Lista vazia é "não se sabe", não placa parada.
        let gpu_pct = (!motores.is_empty()).then(|| motores.iter().sum::<f64>().min(100.0));

        let dedicada = self
            .lista(self.vram)
            .into_iter()
            .map(|(nome, bytes)| (nome, bytes / 1_048_576.0))
            .reduce(|a, b| if b.1 > a.1 { b } else { a });

        // Instância ausente é `None`, e não zero, que diria "não está derramando nada".
        let vram_compartilhada_mb = dedicada.as_ref().and_then(|(nome, _)| {
            self.lista(self.vram_compartilhada)
                .into_iter()
                .find(|(n, _)| n == nome)
                .map(|(_, bytes)| bytes / 1_048_576.0)
        });

        Amostra {
            gpu_pct,
            vram_mb: dedicada.map(|(_, mb)| mb),
            vram_compartilhada_mb,
            disco_ocupado_pct: self.valor(self.disco_tempo).map(|v| v.min(100.0)),
            // Vem em SEGUNDOS por transferência: sem converter, erraria por mil.
            disco_latencia_ms: self.valor(self.disco_latencia).map(|s| s * 1000.0),
        }
    }
}

impl Drop for Contadores {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Performance::PdhCloseQuery;
        unsafe { PdhCloseQuery(self.consulta) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Afirma o CONTRATO: o que vier está na faixa possível, e o que não vier vem ausente.
    #[test]
    fn os_contadores_respondem_dentro_do_possivel() {
        let Some(c) = Contadores::novo() else {
            return;
        };

        std::thread::sleep(std::time::Duration::from_millis(300));
        let a = c.coletar();

        println!("{a:?}");

        if let Some(gpu) = a.gpu_pct {
            assert!((0.0..=100.0).contains(&gpu), "uso da placa em {gpu}%");
        }
        if let Some(vram) = a.vram_mb {
            assert!(vram >= 0.0, "memória de vídeo em {vram} MB");
            assert!(
                vram < 1_048_576.0,
                "1 TB de VRAM é leitura errada, não placa boa"
            );
        }
        if let Some(mb) = a.vram_compartilhada_mb {
            assert!(mb >= 0.0, "memória compartilhada em {mb} MB");
            assert!(mb < 1_048_576.0, "1 TB compartilhado é leitura errada");
            assert!(
                a.vram_mb.is_some(),
                "compartilhada sem dedicada é instância de adaptador trocada"
            );
        }
        if let Some(disco) = a.disco_ocupado_pct {
            assert!((0.0..=100.0).contains(&disco));
        }
        if let Some(ms) = a.disco_latencia_ms {
            assert!(ms >= 0.0);
            assert!(ms < 10_000.0, "latência de {ms} ms");
        }
    }

    #[test]
    fn duas_consultas_convivem() {
        // Painel e medição de quadros abrem consultas ao mesmo tempo: o PDH precisa aguentar.
        let (Some(a), Some(b)) = (Contadores::novo(), Contadores::novo()) else {
            return;
        };

        std::thread::sleep(std::time::Duration::from_millis(300));
        let (x, y) = (a.coletar(), b.coletar());

        assert_eq!(x.gpu_pct.is_some(), y.gpu_pct.is_some());
        assert_eq!(x.vram_mb.is_some(), y.vram_mb.is_some());
        assert_eq!(
            x.vram_compartilhada_mb.is_some(),
            y.vram_compartilhada_mb.is_some()
        );
    }
}
