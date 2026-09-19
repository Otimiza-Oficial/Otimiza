// Contadores da placa de vídeo e do disco, pelo caminho barato
//
// POR QUE ISTO EXISTE
//
// A mesma informação já era lida em `bottleneck.rs`, por WMI através do
// PowerShell. O comentário lá diz o custo com todas as letras: cada chamada
// passa de um segundo. Isso obrigou o painel a ler a placa de vídeo só a cada
// dez segundos, e a mostrar o número com a idade escrita porque ele não era
// de agora.
//
// Pior: dentro da janela em que o Otimiza mede os quadros de um jogo, não cabe
// nenhuma consulta dessas. Abrir um PowerShell no meio de uma medição de
// desempenho é virar a carga que se está medindo. Sem o uso da placa naquela
// janela, "o jogo está limitado pelo motor e não pelo hardware" ficou sem como
// ser respondido.
//
// Os mesmos números existem como contadores de desempenho do Windows, que é o
// que este módulo lê. Não abre processo nenhum, custa microssegundos, e pode
// rodar ao lado da medição de quadros sem pesar nela.
//
// O QUE MUDA E O QUE NÃO MUDA
//
// Muda a origem e o custo. NÃO muda a regra: contador que não responde chega
// como `None`. O `unwrap_or(0.0)` que existia no laço do `bottleneck.rs`
// transformava consulta falha em "placa a 0%", e uma placa a 95% com uma
// leitura perdida no meio saía como 63% — o suficiente para o veredito deixar
// de dizer GPU.
//
// SOBRE OS NOMES EM INGLÊS
//
// `PdhAddEnglishCounterW`, e não `PdhAddCounterW`: os caminhos de contador são
// TRADUZIDOS no Windows em português, e a versão em inglês é a única que
// funciona em qualquer idioma. É a mesma escolha do amostrador do motor de
// energia.

#![cfg(target_os = "windows")]

use serde::{Deserialize, Serialize};

const PDH_OK: u32 = 0;

fn largo(texto: &str) -> Vec<u16> {
    texto.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Uma leitura dos contadores, com o que não respondeu ausente.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Amostra {
    /// Uso somado dos motores 3D, 0-100%.
    ///
    /// Só os motores 3D: uma placa expõe vários — 3D, cópia, vídeo, codec — e
    /// somar todos daria número acima de 100 que não representa o que um jogo
    /// pede.
    pub gpu_pct: Option<f64>,
    /// Memória dedicada em uso, em MB. O MAIOR entre os adaptadores.
    ///
    /// Máximo e não soma: numa máquina com placa integrada e dedicada, somar
    /// as duas produziria um total que nenhuma das duas tem.
    pub vram_mb: Option<f64>,
    /// `% Disk Time` do total dos discos físicos.
    pub disco_ocupado_pct: Option<f64>,
    /// `Avg. Disk sec/Transfer`, convertido para milissegundos.
    ///
    /// É a LATÊNCIA, que é outra pergunta que a ocupação: um disco 100%
    /// ocupado com latência de 0,2 ms está dando conta, e um disco a 40% com
    /// latência de 30 ms é o que trava o jogo.
    pub disco_latencia_ms: Option<f64>,
}

pub struct Contadores {
    consulta: isize,
    gpu: isize,
    vram: isize,
    disco_tempo: isize,
    disco_latencia: isize,
}

// Os campos são alças do Windows, não ponteiros para memória deste processo.
// A consulta é usada por uma thread de cada vez.
unsafe impl Send for Contadores {}

impl Contadores {
    /// Abre a consulta. `None` quando o Windows recusa.
    ///
    /// Uma máquina sem os contadores de GPU — Windows mais antigo que o
    /// 10 1709, ou driver que não os publica — devolve alça zero para eles, e
    /// a leitura correspondente sai como ausente em vez de zero.
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
            disco_tempo: adicionar(r"\PhysicalDisk(_Total)\% Disk Time"),
            disco_latencia: adicionar(r"\PhysicalDisk(_Total)\Avg. Disk sec/Transfer"),
        };

        // A primeira coleta é a linha de base: contador de taxa é a diferença
        // entre duas coletas, e sem esta a leitura seguinte não teria com o que
        // comparar.
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

    /// Todas as instâncias de um contador curinga, com o nome de cada uma.
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

        // A primeira chamada só descobre o tamanho do buffer; ela DEVE falhar
        // com `MORE_DATA`. Qualquer outra resposta é motivo para desistir.
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

    /// Lê os contadores agora.
    ///
    /// A janela medida é o tempo desde a coleta anterior desta mesma consulta.
    /// Chamar duas vezes coladas devolve números sem sentido — o mesmo cuidado
    /// que o amostrador do motor de energia exige.
    pub fn coletar(&self) -> Amostra {
        use windows_sys::Win32::System::Performance::PdhCollectQueryData;
        unsafe { PdhCollectQueryData(self.consulta) };

        // Instância de motor 3D tem o nome terminando em `engtype_3D`. As
        // outras — cópia, vídeo, codec — descrevem trabalho que não é o que um
        // jogo pede, e entrariam somando acima de 100.
        let motores: Vec<f64> = self
            .lista(self.gpu)
            .into_iter()
            .filter(|(nome, _)| nome.ends_with("engtype_3D"))
            .map(|(_, v)| v)
            .collect();

        // Lista vazia é lista vazia, não placa parada: quando o contador não
        // existe nesta máquina, a resposta é que não se sabe.
        let gpu_pct = (!motores.is_empty()).then(|| motores.iter().sum::<f64>().min(100.0));

        let vram_mb = self
            .lista(self.vram)
            .into_iter()
            .map(|(_, bytes)| bytes / 1_048_576.0)
            .reduce(f64::max);

        Amostra {
            gpu_pct,
            vram_mb,
            disco_ocupado_pct: self.valor(self.disco_tempo).map(|v| v.min(100.0)),
            // O contador vem em SEGUNDOS por transferência. Publicar isso como
            // milissegundo sem converter erraria por mil, que é exatamente o
            // tipo de engano que o contrato de telemetria passou a barrar com
            // unidade tipada.
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

    /// A consulta abre nesta máquina e devolve números possíveis.
    ///
    /// Não afirma que a placa está a tal por cento: afirma o CONTRATO — o que
    /// vier tem de estar na faixa possível, e o que não vier tem de vir
    /// ausente em vez de zero.
    #[test]
    fn os_contadores_respondem_dentro_do_possivel() {
        let Some(c) = Contadores::novo() else {
            // Máquina sem PDH. O teste não inventa aprovação nem reprovação.
            return;
        };

        // Uma janela mínima: contador de taxa precisa de duas coletas
        // separadas no tempo.
        std::thread::sleep(std::time::Duration::from_millis(300));
        let a = c.coletar();

        // Impresso com `--nocapture` para quem for depurar numa máquina que
        // responde diferente desta.
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
        if let Some(disco) = a.disco_ocupado_pct {
            assert!((0.0..=100.0).contains(&disco));
        }
        if let Some(ms) = a.disco_latencia_ms {
            assert!(ms >= 0.0);
            // Latência de mais de dez segundos por transferência não é disco
            // lento, é leitura estragada.
            assert!(ms < 10_000.0, "latência de {ms} ms");
        }
    }

    #[test]
    fn duas_consultas_convivem() {
        // O painel e a medição de quadros abrem consultas separadas ao mesmo
        // tempo. Se o PDH não aguentasse isso, medir a placa durante a partida
        // seria impossível — que é justamente o caso de uso deste módulo.
        let (Some(a), Some(b)) = (Contadores::novo(), Contadores::novo()) else {
            return;
        };

        std::thread::sleep(std::time::Duration::from_millis(300));
        let (x, y) = (a.coletar(), b.coletar());

        // Não se exige o MESMO número: são janelas ligeiramente diferentes.
        // Exige-se que as duas tenham respondido às mesmas perguntas.
        assert_eq!(x.gpu_pct.is_some(), y.gpu_pct.is_some());
        assert_eq!(x.vram_mb.is_some(), y.vram_mb.is_some());
    }
}
