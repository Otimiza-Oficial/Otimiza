// Caçador de DPC (docs/planos/PLANO-1.0-NOVA-ERA.md, 7.2): qual DRIVER segura o processador em DPC ou interrupção
// (ISR) a ponto de virar tranco, chiado de áudio ou atraso de entrada. É o que o LatencyMon faz, com código próprio.
//
// Como: uma sessão de rastreamento do KERNEL em tempo real (nunca arquivo .etl em disco), com os eventos de DPC, de
// interrupção e de carga de imagem. Cada evento de DPC/ISR do provedor PerfInfo traz `InitialTime` (quando a rotina
// começou) e `Routine` (o endereço dela); o carimbo do próprio evento é o FIM. Duração = fim − início, no relógio da
// sessão (QPC, pedido com `ClientContext = 1`). O endereço vira driver pelos eventos de imagem (base + tamanho), que o
// kernel reenvia para tudo que já está carregado quando a sessão começa (o "rundown").
//
// Honestidade: tudo aqui é MEDIDO nesta captura, com a duração e a resolução do relógio junto. Evento abaixo da
// resolução é contado como tal, não como zero. Endereço sem módulo vira "sem módulo", nunca um driver chutado. Uma
// ISR que interrompe uma DPC entra também no tempo da DPC (o kernel mede início e fim da rotina, não o tempo líquido):
// o LatencyMon tem o mesmo limite, e por isso o veredito olha picos por driver, não a soma das categorias.
//
// Memória limitada: agrega no ato (por rotina: contagem, soma, máximo, faixas e um histograma de 161 baldes), não
// guarda evento. Só os picos acima de 500 µs ficam com o carimbo, até um teto, para a causa da travada cruzar depois.
//
// A parte pura (`Agregador`, `analisar`, leitura dos bytes) não toca na máquina e é testada com eventos sintéticos;
// a captura (`capturar`) fica em `maquina`, só no Windows, exige administrador e para a sessão em qualquer saída
// (guarda RAII), porque uma sessão de kernel esquecida fica viva até alguém pará-la ou o PC reiniciar.

// Nenhum comando liga isto ainda (os comandos ficam para quem divide `src/commands/`): sem o `allow`, a captura e a
// análise inteiras acusariam "nunca usado". Tirar quando o comando existir.
#![allow(dead_code, unused_imports)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Duração da captura quando ninguém pede outra. 30 s pega pico raro (Wi-Fi varrendo redes, a cada ~60 s em alguns
/// drivers, pode escapar: está dito na tela, não escondido).
pub const SEGUNDOS_PADRAO: u64 = 30;
/// Teto: a sessão de kernel custa processador enquanto existe; mais de um minuto vira outra ferramenta.
pub const SEGUNDOS_MAXIMO: u64 = 60;
pub const SEGUNDOS_MINIMO: u64 = 5;

/// As faixas do LatencyMon e do que se sente: 500 µs já atrasa buffer de áudio pequeno; 1 ms é o ponto em que uma
/// rotina de driver passa a ser problema em qualquer guia sério; 2 ms come metade de um quadro a 240 Hz.
pub const LIMIAR_ATENCAO_NS: u64 = 500_000;
pub const LIMIAR_PICO_NS: u64 = 1_000_000;
pub const LIMIAR_FORTE_NS: u64 = 2_000_000;

/// Acima disso o par início/fim não é de uma rotina só: evento cruzado ou carimbo de outro relógio. Descartado e
/// contado, nunca somado como "DPC de 3 segundos".
const DURACAO_IMPOSSIVEL_NS: u64 = 1_000_000_000;

/// Rotinas distintas numa máquina comum ficam na casa das centenas. Passou disso, o resto vai num balde só (a conta
/// fecha, a memória não cresce).
const MAXIMO_DE_ROTINAS: usize = 4096;
const MAXIMO_DE_IMAGENS: usize = 8192;
/// Picos com carimbo, para a causa da travada. Mais que isso numa captura de 60 s já é diagnóstico fechado.
pub const MAXIMO_DE_PICOS: usize = 2000;

/// Endereços do kernel em x64 ficam na metade de cima. DLL de processo comum (o rundown manda todas) não interessa.
const INICIO_DO_KERNEL: u64 = 0xFFFF_8000_0000_0000;

// Histograma geométrico: balde 0 é < 1 µs; depois 8 baldes por oitava (~9% de largura) de 1 µs até ~1 s. O p99 sai
// como o TETO do balde ("até X µs"), nunca mais preciso do que a medida.
const BALDES_POR_OITAVA: f64 = 8.0;
const BALDES: usize = 161;

fn balde(ns: u64) -> usize {
    if ns < 1000 {
        return 0;
    }
    let i = 1 + (BALDES_POR_OITAVA * (ns as f64 / 1000.0).log2()).floor() as usize;
    i.min(BALDES - 1)
}

fn teto_do_balde_ns(i: usize) -> f64 {
    if i == 0 {
        1000.0
    } else {
        1000.0 * 2f64.powf(i as f64 / BALDES_POR_OITAVA)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub enum TipoDeRotina {
    /// DPC comum, de temporizador ou "threaded".
    Dpc,
    /// Rotina de interrupção (ISR), inclusive MSI.
    Isr,
}

#[derive(Debug, Clone)]
struct Estatistica {
    contagem: u64,
    total_ns: u64,
    maximo_ns: u64,
    acima_atencao: u64,
    acima_pico: u64,
    acima_forte: u64,
    abaixo_da_resolucao: u64,
    histograma: Box<[u32; BALDES]>,
}

impl Estatistica {
    fn nova() -> Self {
        Estatistica {
            contagem: 0,
            total_ns: 0,
            maximo_ns: 0,
            acima_atencao: 0,
            acima_pico: 0,
            acima_forte: 0,
            abaixo_da_resolucao: 0,
            histograma: Box::new([0; BALDES]),
        }
    }

    fn somar(&mut self, ns: u64, abaixo_da_resolucao: bool) {
        self.contagem += 1;
        self.total_ns = self.total_ns.saturating_add(ns);
        self.maximo_ns = self.maximo_ns.max(ns);
        self.acima_atencao += (ns > LIMIAR_ATENCAO_NS) as u64;
        self.acima_pico += (ns > LIMIAR_PICO_NS) as u64;
        self.acima_forte += (ns > LIMIAR_FORTE_NS) as u64;
        self.abaixo_da_resolucao += abaixo_da_resolucao as u64;
        let h = &mut self.histograma[balde(ns)];
        *h = h.saturating_add(1);
    }

    fn juntar(&mut self, outra: &Estatistica) {
        self.contagem += outra.contagem;
        self.total_ns = self.total_ns.saturating_add(outra.total_ns);
        self.maximo_ns = self.maximo_ns.max(outra.maximo_ns);
        self.acima_atencao += outra.acima_atencao;
        self.acima_pico += outra.acima_pico;
        self.acima_forte += outra.acima_forte;
        self.abaixo_da_resolucao += outra.abaixo_da_resolucao;
        for (a, b) in self.histograma.iter_mut().zip(outra.histograma.iter()) {
            *a = a.saturating_add(*b);
        }
    }

    /// Teto do balde onde cai o evento de posto ⌈0,99·n⌉, limitado ao máximo medido (o teto do balde pode passar dele).
    fn p99_ns(&self) -> Option<f64> {
        if self.contagem == 0 {
            return None;
        }
        let posto = ((self.contagem as f64) * 0.99).ceil().max(1.0) as u64;
        let mut acumulado = 0u64;
        for (i, &c) in self.histograma.iter().enumerate() {
            acumulado += c as u64;
            if acumulado >= posto {
                return Some(teto_do_balde_ns(i).min(self.maximo_ns as f64));
            }
        }
        Some(self.maximo_ns as f64)
    }
}

/// Um módulo do kernel visto na captura: base, tamanho e o nome do arquivo (só o nome, minúsculo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imagem {
    pub base: u64,
    pub tamanho: u64,
    pub arquivo: String,
}

#[derive(Debug, Clone, Copy)]
struct PicoCru {
    inicio: i64,
    fim: i64,
    rotina: u64,
    tipo: TipoDeRotina,
    ns: u64,
}

/// Acumula a captura no ato. Recebe os carimbos no relógio da sessão e a frequência dele.
#[derive(Debug)]
pub struct Agregador {
    frequencia: i64,
    por_rotina: HashMap<(u64, TipoDeRotina), Estatistica>,
    /// Rotinas além do teto: a conta continua, sem nome.
    excedente: HashMap<TipoDeRotina, Estatistica>,
    imagens: Vec<Imagem>,
    picos: Vec<PicoCru>,
    picos_alem_do_teto: u64,
    eventos: u64,
    descartados: u64,
}

impl Agregador {
    pub fn novo(frequencia: i64) -> Self {
        Agregador {
            frequencia,
            por_rotina: HashMap::with_capacity(512),
            excedente: HashMap::new(),
            imagens: Vec::with_capacity(512),
            picos: Vec::with_capacity(256),
            picos_alem_do_teto: 0,
            eventos: 0,
            descartados: 0,
        }
    }

    /// Uma execução de rotina. `inicio` e `fim` no relógio da sessão. Fim antes do início ou duração impossível é
    /// descartado e contado.
    pub fn registrar(&mut self, tipo: TipoDeRotina, rotina: u64, inicio: i64, fim: i64) {
        self.eventos += 1;
        if self.frequencia <= 0 || fim < inicio {
            self.descartados += 1;
            return;
        }
        let ticks = (fim - inicio) as u128;
        let ns = (ticks * 1_000_000_000 / self.frequencia as u128).min(u64::MAX as u128) as u64;
        if ns > DURACAO_IMPOSSIVEL_NS {
            self.descartados += 1;
            return;
        }
        let abaixo = ticks == 0;

        let chave = (rotina, tipo);
        if let Some(e) = self.por_rotina.get_mut(&chave) {
            e.somar(ns, abaixo);
        } else if self.por_rotina.len() < MAXIMO_DE_ROTINAS {
            let mut e = Estatistica::nova();
            e.somar(ns, abaixo);
            self.por_rotina.insert(chave, e);
        } else {
            self.excedente
                .entry(tipo)
                .or_insert_with(Estatistica::nova)
                .somar(ns, abaixo);
        }

        if ns > LIMIAR_ATENCAO_NS {
            if self.picos.len() < MAXIMO_DE_PICOS {
                self.picos.push(PicoCru {
                    inicio,
                    fim,
                    rotina,
                    tipo,
                    ns,
                });
            } else {
                self.picos_alem_do_teto += 1;
            }
        }
    }

    /// Imagem carregada (ou já carregada, no rundown). Só as do kernel interessam.
    pub fn registrar_imagem(&mut self, base: u64, tamanho: u64, caminho: &str) {
        if base < INICIO_DO_KERNEL || tamanho == 0 || self.imagens.len() >= MAXIMO_DE_IMAGENS {
            return;
        }
        let arquivo = nome_do_arquivo(caminho);
        if arquivo.is_empty() || self.imagens.iter().any(|i| i.base == base) {
            return;
        }
        self.imagens.push(Imagem {
            base,
            tamanho,
            arquivo,
        });
    }

    pub fn eventos(&self) -> u64 {
        self.eventos
    }

    pub fn imagens(&self) -> usize {
        self.imagens.len()
    }
}

/// `\SystemRoot\System32\drivers\NVLDDMKM.SYS` → `nvlddmkm.sys`.
pub fn nome_do_arquivo(caminho: &str) -> String {
    caminho
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or("")
        .trim_matches(char::from(0))
        .trim()
        .to_lowercase()
}

// ---------------------------------------------------------------------------------------------------------------
// Leitura dos bytes dos eventos (MOF do kernel). Pura, para ser testada com bytes montados à mão.
// ---------------------------------------------------------------------------------------------------------------

/// Tipos de evento (opcode) da classe PerfInfo que interessam. 66 ThreadedDPC, 68 DPC, 69 TimerDPC; 67 ISR, 50 ISR-MSI.
pub fn tipo_do_opcode_perfinfo(opcode: u8) -> Option<TipoDeRotina> {
    match opcode {
        66 | 68 | 69 => Some(TipoDeRotina::Dpc),
        50 | 67 => Some(TipoDeRotina::Isr),
        _ => None,
    }
}

/// Opcodes da classe Image que trazem módulo carregado: 10 Load, 3 DCStart (rundown do que já estava carregado).
pub fn opcode_de_imagem_carregada(opcode: u8) -> bool {
    matches!(opcode, 3 | 10)
}

fn ler_u32(b: &[u8], em: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(em..em + 4)?.try_into().ok()?))
}

fn ler_u64(b: &[u8], em: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(em..em + 8)?.try_into().ok()?))
}

fn ler_ponteiro(b: &[u8], em: usize, de_64: bool) -> Option<u64> {
    if de_64 {
        ler_u64(b, em)
    } else {
        ler_u32(b, em).map(u64::from)
    }
}

/// `DPC`/`ISR`: `InitialTime` (WmiTime, 8 bytes) e `Routine` (ponteiro). O resto da ISR (retorno, vetor) não importa.
pub fn ler_dpc_ou_isr(dados: &[u8], de_64: bool) -> Option<(i64, u64)> {
    let inicio = ler_u64(dados, 0)? as i64;
    let rotina = ler_ponteiro(dados, 8, de_64)?;
    Some((inicio, rotina))
}

/// `Image_Load` versão 2 e 3 (Vista em diante, iguais em tamanho; a 3 só divide o `Reserved0`): ImageBase, ImageSize
/// (ponteiros), ProcessId, ImageCheckSum, TimeDateStamp, Reserved0 (4 bytes cada), DefaultBase (ponteiro), Reserved1–4
/// (4 bytes cada) e o caminho em UTF-16 terminado em zero. Versão mais velha: `None` (sem chute de deslocamento).
pub fn ler_imagem(dados: &[u8], de_64: bool, versao: u8) -> Option<(u64, u64, String)> {
    if versao < 2 {
        return None;
    }
    let p = if de_64 { 8 } else { 4 };
    let base = ler_ponteiro(dados, 0, de_64)?;
    let tamanho = ler_ponteiro(dados, p, de_64)?;
    let nome_em = 2 * p + 16 + p + 16;
    let cru = dados.get(nome_em..)?;
    let unidades: Vec<u16> = cru
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    let caminho = String::from_utf16(&unidades).ok()?;
    (!caminho.is_empty()).then_some((base, tamanho, caminho))
}

// ---------------------------------------------------------------------------------------------------------------
// Quem é o driver e o que fazer. Lista curta e explicada: arquivo fora dela vira "outro driver", com o nome dele.
// ---------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "CategoriaDoDriver"))]
pub enum Categoria {
    Video,
    Rede,
    Energia,
    Disco,
    Usb,
    Audio,
    /// Framework de drivers da Microsoft: hospeda a rotina de outro driver, que a captura não nomeia.
    FrameworkWdf,
    /// O próprio Windows (relógio, agendador).
    Windows,
    Outro,
    SemModulo,
}

pub fn classificar(arquivo: &str) -> (Categoria, String) {
    let a = arquivo.to_lowercase();
    let comeca = |prefixos: &[&str]| prefixos.iter().any(|p| a.starts_with(p));
    let (c, nome): (Categoria, &str) = match a.as_str() {
        "nvlddmkm.sys" => (Categoria::Video, "Driver de vídeo NVIDIA"),
        "dxgkrnl.sys" | "dxgmms2.sys" | "dxgmms1.sys" => {
            (Categoria::Video, "Núcleo gráfico do Windows (DirectX), movido pelo driver de vídeo")
        }
        "amdkmdag.sys" | "atikmdag.sys" | "atikmpag.sys" | "amdkmpfd.sys" => {
            (Categoria::Video, "Driver de vídeo AMD")
        }
        "ndis.sys" => (Categoria::Rede, "Pilha de rede do Windows (NDIS), movida pelo driver da placa"),
        "tcpip.sys" | "netio.sys" | "afd.sys" => (Categoria::Rede, "Pilha TCP/IP do Windows"),
        "nwifi.sys" | "wdiwifi.sys" | "vwifibus.sys" => (Categoria::Rede, "Wi-Fi do Windows"),
        "acpi.sys" => (Categoria::Energia, "ACPI (energia e firmware da placa-mãe)"),
        "intelppm.sys" | "amdppm.sys" | "processr.sys" => {
            (Categoria::Energia, "Gerência de energia do processador")
        }
        "storport.sys" | "stornvme.sys" | "storahci.sys" | "disk.sys" | "classpnp.sys" | "ataport.sys"
        | "pciide.sys" | "secnvme.sys" => (Categoria::Disco, "Controlador de disco"),
        "usbxhci.sys" | "ucx01000.sys" | "usbport.sys" | "usbhub3.sys" | "usbhub.sys" | "usbehci.sys"
        | "hidusb.sys" | "hidclass.sys" => (Categoria::Usb, "Controlador USB"),
        "hdaudbus.sys" | "portcls.sys" | "hdaudio.sys" | "usbaudio.sys" | "usbaudio2.sys" | "nvhda64v.sys"
        | "atihdwt6.sys" | "rtkvhd64.sys" | "intcaudiobus.sys" => (Categoria::Audio, "Áudio"),
        "wdf01000.sys" | "wdf01000" => (Categoria::FrameworkWdf, "Framework de drivers do Windows (WDF)"),
        "ntoskrnl.exe" | "ntkrnlmp.exe" | "hal.dll" => (Categoria::Windows, "O próprio Windows (kernel)"),
        _ if comeca(&["igdkmd", "igdkmdn", "igdkmdnd"]) => (Categoria::Video, "Driver de vídeo Intel"),
        // Placas de rede e Wi-Fi mais comuns: Realtek (rt*/rtwlan*), Intel (e1*, netwtw*, netwaw*), Killer, Qualcomm
        // Atheros (athw*, qcwlan*), Broadcom (bcmwl*), MediaTek (mtkwl*).
        _ if comeca(&[
            "rt640", "rt68", "rtux", "rtwlan", "rtkfilter", "e1d", "e1r", "e1i", "e2f", "netwtw", "netwaw",
            "netwbw", "netadaptercx", "killer", "athw", "qcwlan", "bcmwl", "mtkwl", "ixgbe",
        ]) =>
        {
            (Categoria::Rede, "Placa de rede ou Wi-Fi")
        }
        _ if comeca(&["iastor", "nvme", "amdsata", "amdraid", "rcraid"]) => {
            (Categoria::Disco, "Controlador de disco")
        }
        _ if comeca(&["rtkvhd", "intcaud", "intcdaud", "nahimic", "sswrapper"]) => (Categoria::Audio, "Áudio"),
        "" => (Categoria::SemModulo, "Endereço fora dos módulos conhecidos"),
        _ => (Categoria::Outro, ""),
    };
    let nome = if nome.is_empty() {
        format!("Driver {}", arquivo)
    } else {
        nome.to_string()
    };
    (c, nome)
}

/// A ação honesta por categoria. Só aparece para driver com pico: não se manda ninguém mexer em driver saudável.
pub fn acao(c: Categoria) -> &'static str {
    match c {
        Categoria::Video => {
            "Atualize ou reinstale o driver de vídeo limpo (desinstalar o atual e instalar o mais novo do site do \
             fabricante)."
        }
        Categoria::Rede => {
            "Driver da placa de rede ou Wi-Fi: atualize pelo site do fabricante da placa-mãe ou da placa. Teste \
             jogando por cabo: se o pico some, é o Wi-Fi."
        }
        Categoria::Energia => {
            "ACPI é a parte de energia da placa-mãe: atualize a BIOS e o driver do chipset. Recursos de economia de \
             energia da BIOS podem estar envolvidos."
        }
        Categoria::Disco => {
            "Controlador de disco: atualize o driver do chipset ou do armazenamento (NVMe/SATA) e confira a saúde \
             do disco."
        }
        Categoria::Usb => {
            "Algum dispositivo USB. Desconecte um por vez (fone, controle, hub, webcam) e capture de novo: o que \
             fizer o pico sumir é o culpado."
        }
        Categoria::Audio => {
            "Driver de áudio: atualize pelo site do fabricante da placa-mãe ou do fone. Desligar saídas de áudio que \
             você não usa (HDMI do monitor, por exemplo) também ajuda a achar."
        }
        Categoria::FrameworkWdf => {
            "É um driver construído com o framework da Microsoft, e a captura não diz qual. Desconecte dispositivos \
             um por vez e capture de novo para achar."
        }
        Categoria::Windows => {
            "É o próprio Windows (relógio e agendador). Não há driver de terceiro para trocar aqui."
        }
        Categoria::Outro => {
            "Atualize o driver deste dispositivo pelo site do fabricante, ou desative o dispositivo se não usa."
        }
        Categoria::SemModulo => {
            "A rotina não caiu em nenhum módulo carregado, e preferimos não chutar o driver."
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Relatório.
// ---------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "RelogioDoDpc"))]
pub enum Relogio {
    /// Contador de alta resolução: o que pedimos.
    Qpc,
    /// Hora do sistema: resolução ruim, o relatório avisa.
    Sistema,
    Ciclos,
    Desconhecido,
}

impl Relogio {
    /// `ReservedFlags` do cabeçalho da sessão.
    pub fn do_cabecalho(v: u32) -> Self {
        match v {
            1 => Relogio::Qpc,
            2 => Relogio::Sistema,
            3 => Relogio::Ciclos,
            _ => Relogio::Desconhecido,
        }
    }
}

/// Como o endereço virou driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "AtribuicaoDoDpc"))]
pub enum Atribuicao {
    /// Caiu dentro de base + tamanho de um módulo visto na captura.
    Exata,
    /// Sem o evento de imagem: o módulo de base mais próxima abaixo, pela lista de drivers do Windows.
    Aproximada,
    SemModulo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "NivelDoDpc"))]
pub enum Nivel {
    Saudavel,
    /// Passou de 500 µs, ou um único evento isolado acima de 1 ms.
    Atencao,
    /// Dois ou mais eventos acima de 1 ms, ou qualquer um acima de 2 ms.
    Pico,
}

fn nivel(e: &Estatistica) -> Nivel {
    if e.acima_forte >= 1 || e.acima_pico >= 2 {
        Nivel::Pico
    } else if e.acima_pico == 1 || e.acima_atencao >= 1 {
        Nivel::Atencao
    } else {
        Nivel::Saudavel
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct DriverMedido {
    /// Nome do arquivo (`nvlddmkm.sys`), ou vazio quando a rotina não caiu em módulo nenhum.
    pub arquivo: String,
    pub nome_amigavel: String,
    pub categoria: Categoria,
    pub atribuicao: Atribuicao,
    pub contagem: u32,
    pub contagem_dpc: u32,
    pub contagem_isr: u32,
    pub tempo_total_ms: f64,
    /// Fração do tempo de UM núcleo (DPC cai quase toda no núcleo 0): o que se compara com o LatencyMon.
    pub pct_de_um_nucleo: f64,
    pub maximo_us: f64,
    /// Teto do balde do histograma onde está o p99 (±9%): "até X µs".
    pub p99_ate_us: f64,
    pub acima_500us: u32,
    pub acima_1ms: u32,
    pub acima_2ms: u32,
    /// Execuções curtas demais para o relógio (0 tique): contadas, não zeradas.
    pub abaixo_da_resolucao: u32,
    pub nivel: Nivel,
    /// Só com `Atencao` ou `Pico`.
    pub acao: Option<String>,
}

/// Um pico com carimbo, no relógio da sessão (QPC): a causa da travada cruza com os quadros pelo mesmo relógio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct PicoDeDriver {
    pub inicio_qpc: i64,
    pub fim_qpc: i64,
    pub arquivo: String,
    pub tipo: TipoDeRotina,
    pub duracao_us: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "VereditoDoDpc"))]
#[serde(tag = "tipo")]
pub enum Veredito {
    /// Nenhum evento de DPC/ISR chegou: NÃO é "PC saudável", é "não medimos".
    SemEventos { texto: String },
    /// Resultado bom e válido.
    NenhumPico { texto: String, observacao: Option<String> },
    DriversComPico { culpados: Vec<String>, texto: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct RelatorioDpc {
    /// Sempre "MEDIDO": nada aqui é estimado.
    pub origem: String,
    pub duracao_s: f64,
    pub nucleos: u32,
    pub relogio: Relogio,
    pub resolucao_ns: f64,
    pub eventos: u32,
    /// Fim antes do início ou duração acima de 1 s: fora da conta.
    pub descartados: u32,
    /// O que o Windows diz que perdeu (buffer cheio). Com perda, a contagem é piso, e o relatório diz.
    pub eventos_perdidos: u32,
    pub buffers_perdidos: u32,
    pub modulos_vistos: u32,
    pub sessao: String,
    pub pct_total_de_um_nucleo: f64,
    pub pct_total_do_processador: f64,
    /// Ordenados do pior para o melhor.
    pub drivers: Vec<DriverMedido>,
    pub picos: Vec<PicoDeDriver>,
    /// Picos acima de 500 µs que passaram do teto da lista (contados nos drivers, sem carimbo).
    pub picos_sem_carimbo: u32,
    pub veredito: Veredito,
    pub avisos: Vec<String>,
}

/// O que a captura sabe da sessão, além dos eventos.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextoDaCaptura {
    pub duracao_s: f64,
    pub nucleos: u32,
    pub relogio: Relogio,
    pub eventos_perdidos: u32,
    pub buffers_perdidos: u32,
    pub sessao: String,
}

/// Rotina → módulo. `imagens` e `aproximadas` ordenadas por base.
pub fn atribuir(rotina: u64, imagens: &[Imagem], aproximadas: &[(u64, String)]) -> (String, Atribuicao) {
    let i = imagens.partition_point(|m| m.base <= rotina);
    if i > 0 {
        let m = &imagens[i - 1];
        if rotina < m.base.saturating_add(m.tamanho) {
            return (m.arquivo.clone(), Atribuicao::Exata);
        }
    }
    if rotina >= INICIO_DO_KERNEL {
        let j = aproximadas.partition_point(|(b, _)| *b <= rotina);
        if j > 0 {
            return (aproximadas[j - 1].1.clone(), Atribuicao::Aproximada);
        }
    }
    (String::new(), Atribuicao::SemModulo)
}

fn arred(v: f64, casas: i32) -> f64 {
    let m = 10f64.powi(casas);
    (v * m).round() / m
}

fn sat32(v: u64) -> u32 {
    v.min(u32::MAX as u64) as u32
}

/// **Pura.** Junta as rotinas por driver, aplica as faixas e escreve o veredito.
pub fn analisar(ag: &Agregador, ctx: &ContextoDaCaptura, aproximadas: &[(u64, String)]) -> RelatorioDpc {
    let mut imagens = ag.imagens.clone();
    imagens.sort_by_key(|m| m.base);
    let mut aprox = aproximadas.to_vec();
    aprox.sort_by_key(|(b, _)| *b);

    struct Junta {
        atribuicao: Atribuicao,
        geral: Estatistica,
        dpc: u64,
        isr: u64,
    }
    let mut por_driver: HashMap<String, Junta> = HashMap::new();
    let mut nome_da_rotina: HashMap<u64, String> = HashMap::new();

    let mut somar = |arquivo: String, atrib: Atribuicao, tipo: TipoDeRotina, e: &Estatistica| {
        let j = por_driver.entry(arquivo).or_insert_with(|| Junta {
            atribuicao: atrib,
            geral: Estatistica::nova(),
            dpc: 0,
            isr: 0,
        });
        // Uma rotina exata vale mais que uma aproximada para dizer como o driver foi nomeado.
        if atrib == Atribuicao::Exata {
            j.atribuicao = Atribuicao::Exata;
        }
        j.geral.juntar(e);
        match tipo {
            TipoDeRotina::Dpc => j.dpc += e.contagem,
            TipoDeRotina::Isr => j.isr += e.contagem,
        }
    };

    for (&(rotina, tipo), e) in &ag.por_rotina {
        let (arquivo, atrib) = atribuir(rotina, &imagens, &aprox);
        nome_da_rotina.insert(rotina, arquivo.clone());
        somar(arquivo, atrib, tipo, e);
    }
    for (&tipo, e) in &ag.excedente {
        somar(String::new(), Atribuicao::SemModulo, tipo, e);
    }

    let duracao_ms = (ctx.duracao_s * 1000.0).max(f64::MIN_POSITIVE);
    let nucleos = ctx.nucleos.max(1);

    let mut drivers: Vec<DriverMedido> = por_driver
        .into_iter()
        .map(|(arquivo, j)| {
            let (categoria, nome_amigavel) = classificar(&arquivo);
            let total_ms = j.geral.total_ns as f64 / 1e6;
            let n = nivel(&j.geral);
            DriverMedido {
                arquivo,
                nome_amigavel,
                categoria,
                atribuicao: j.atribuicao,
                contagem: sat32(j.geral.contagem),
                contagem_dpc: sat32(j.dpc),
                contagem_isr: sat32(j.isr),
                tempo_total_ms: arred(total_ms, 3),
                pct_de_um_nucleo: arred(total_ms / duracao_ms * 100.0, 3),
                maximo_us: arred(j.geral.maximo_ns as f64 / 1000.0, 1),
                p99_ate_us: arred(j.geral.p99_ns().unwrap_or(0.0) / 1000.0, 1),
                acima_500us: sat32(j.geral.acima_atencao),
                acima_1ms: sat32(j.geral.acima_pico),
                acima_2ms: sat32(j.geral.acima_forte),
                abaixo_da_resolucao: sat32(j.geral.abaixo_da_resolucao),
                nivel: n,
                acao: (n != Nivel::Saudavel).then(|| acao(categoria).to_string()),
            }
        })
        .collect();

    drivers.sort_by(|a, b| {
        b.nivel
            .cmp(&a.nivel)
            .then(b.maximo_us.total_cmp(&a.maximo_us))
            .then(b.tempo_total_ms.total_cmp(&a.tempo_total_ms))
    });

    let total_ms: f64 = drivers.iter().map(|d| d.tempo_total_ms).sum();

    let mut picos: Vec<PicoDeDriver> = ag
        .picos
        .iter()
        .map(|p| PicoDeDriver {
            inicio_qpc: p.inicio,
            fim_qpc: p.fim,
            arquivo: nome_da_rotina
                .get(&p.rotina)
                .cloned()
                .unwrap_or_else(|| atribuir(p.rotina, &imagens, &aprox).0),
            tipo: p.tipo,
            duracao_us: arred(p.ns as f64 / 1000.0, 1),
        })
        .collect();
    picos.sort_by_key(|p| p.fim_qpc);

    let frequencia = ag.frequencia.max(1) as f64;
    let resolucao_ns = 1e9 / frequencia;

    let mut avisos = Vec::new();
    if ctx.relogio != Relogio::Qpc {
        avisos.push(format!(
            "A sessão não usou o contador de alta resolução ({:?}): durações curtas ficam imprecisas.",
            ctx.relogio
        ));
    }
    if ctx.eventos_perdidos > 0 || ctx.buffers_perdidos > 0 {
        avisos.push(format!(
            "O Windows perdeu {} eventos ({} buffers) durante a captura: as contagens são um piso.",
            ctx.eventos_perdidos, ctx.buffers_perdidos
        ));
    }
    if ag.imagens.is_empty() && !ag.por_rotina.is_empty() {
        avisos.push(
            "Nenhum evento de módulo chegou: os nomes vieram da lista de drivers do Windows (aproximados)."
                .to_string(),
        );
    }
    if ag.descartados > 0 {
        avisos.push(format!(
            "{} eventos com carimbo inconsistente ficaram fora da conta.",
            ag.descartados
        ));
    }

    let veredito = veredito(ag.eventos, &drivers, ctx.duracao_s, resolucao_ns);

    RelatorioDpc {
        origem: "MEDIDO".to_string(),
        duracao_s: arred(ctx.duracao_s, 2),
        nucleos,
        relogio: ctx.relogio,
        resolucao_ns: arred(resolucao_ns, 1),
        eventos: sat32(ag.eventos),
        descartados: sat32(ag.descartados),
        eventos_perdidos: ctx.eventos_perdidos,
        buffers_perdidos: ctx.buffers_perdidos,
        modulos_vistos: sat32(ag.imagens.len() as u64),
        sessao: ctx.sessao.clone(),
        pct_total_de_um_nucleo: arred(total_ms / duracao_ms * 100.0, 3),
        pct_total_do_processador: arred(total_ms / duracao_ms * 100.0 / nucleos as f64, 3),
        drivers,
        picos,
        picos_sem_carimbo: sat32(ag.picos_alem_do_teto),
        veredito,
        avisos,
    }
}

fn rotulo(d: &DriverMedido) -> String {
    if d.arquivo.is_empty() {
        d.nome_amigavel.clone()
    } else {
        format!("{} ({})", d.nome_amigavel, d.arquivo)
    }
}

fn veredito(eventos: u64, drivers: &[DriverMedido], duracao_s: f64, resolucao_ns: f64) -> Veredito {
    if eventos == 0 {
        return Veredito::SemEventos {
            texto: "Nenhum evento de DPC ou interrupção chegou na captura. Isso não quer dizer que o PC está \
                    saudável: quer dizer que não conseguimos medir."
                .to_string(),
        };
    }

    let culpados: Vec<&DriverMedido> = drivers.iter().filter(|d| d.nivel == Nivel::Pico).collect();
    if culpados.is_empty() {
        let atencao: Vec<String> = drivers
            .iter()
            .filter(|d| d.nivel == Nivel::Atencao)
            .map(|d| format!("{}: máximo de {:.0} µs", rotulo(d), d.maximo_us))
            .collect();
        return Veredito::NenhumPico {
            texto: format!(
                "Nenhum driver causando pico em {:.0} s medidos: nenhuma rotina passou de 1 ms de forma repetida. \
                 Relógio com resolução de {:.0} ns.",
                duracao_s, resolucao_ns
            ),
            observacao: (!atencao.is_empty()).then(|| {
                format!(
                    "Isolado, sem repetir: {}. Vale capturar de novo durante o jogo se o tranco continuar.",
                    atencao.join("; ")
                )
            }),
        };
    }

    let pior = culpados[0];
    let texto = format!(
        "{} segurou o processador por até {:.2} ms ({} vezes acima de 1 ms em {:.0} s). {}",
        rotulo(pior),
        pior.maximo_us / 1000.0,
        pior.acima_1ms,
        duracao_s,
        acao(pior.categoria)
    );
    Veredito::DriversComPico {
        culpados: culpados.iter().map(|d| rotulo(d)).collect(),
        texto,
    }
}

/// Os picos dentro de uma janela do relógio QPC: o que a causa da travada pergunta ("teve driver segurando o
/// processador quando o quadro atrasou?"). Inclui pico que só encosta na janela.
pub fn picos_na_janela(rel: &RelatorioDpc, inicio_qpc: i64, fim_qpc: i64) -> Vec<&PicoDeDriver> {
    rel.picos
        .iter()
        .filter(|p| p.fim_qpc >= inicio_qpc && p.inicio_qpc <= fim_qpc)
        .collect()
}

/// O resultado de pedir uma captura. Cada recusa tem nome: a tela nunca mostra "nenhum pico" sobre algo que não mediu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[serde(tag = "estado")]
pub enum ResultadoDaCaptura {
    Medido { relatorio: RelatorioDpc },
    PrecisaDeAdministrador { texto: String },
    /// Outra ferramenta (LatencyMon, WPR, xperf) está com a sessão de kernel, ou o Windows já tem o máximo de
    /// sessões de sistema abertas.
    SessaoOcupada { texto: String },
    JaEmAndamento { texto: String },
    NaoSuportado { texto: String },
    Falhou { etapa: String, codigo: u32, texto: String },
}

/// Como interpretar o código de `StartTrace`. Pura: a regra é testada sem a máquina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InicioDaSessao {
    Comecou,
    /// Sessão com o NOSSO nome sobrou de uma execução que caiu: pode ser parada e recomeçada.
    SobraNossa,
    Ocupada,
    PrecisaDeAdministrador,
    /// O modo "logger de sistema" não existe aqui (Windows 7): tentar o NT Kernel Logger.
    TentarKernelLogger,
    Falhou(u32),
}

pub const ERRO_SUCESSO: u32 = 0;
pub const ERRO_ACESSO_NEGADO: u32 = 5;
pub const ERRO_PARAMETRO_INVALIDO: u32 = 87;
pub const ERRO_NAO_SUPORTADO: u32 = 50;
pub const ERRO_JA_EXISTE: u32 = 183;
pub const ERRO_SEM_RECURSOS_DO_SISTEMA: u32 = 1450;

pub fn interpretar_inicio(codigo: u32, logger_de_sistema: bool) -> InicioDaSessao {
    match codigo {
        ERRO_SUCESSO => InicioDaSessao::Comecou,
        ERRO_ACESSO_NEGADO => InicioDaSessao::PrecisaDeAdministrador,
        // Com o nosso nome, é sobra nossa; o NT Kernel Logger tem nome fixo e pode ser de outro programa: nunca o
        // paramos.
        ERRO_JA_EXISTE if logger_de_sistema => InicioDaSessao::SobraNossa,
        ERRO_JA_EXISTE => InicioDaSessao::Ocupada,
        // São no máximo oito sessões de sistema por vez.
        ERRO_SEM_RECURSOS_DO_SISTEMA => InicioDaSessao::Ocupada,
        ERRO_PARAMETRO_INVALIDO | ERRO_NAO_SUPORTADO if logger_de_sistema => {
            InicioDaSessao::TentarKernelLogger
        }
        outro => InicioDaSessao::Falhou(outro),
    }
}

pub fn texto_precisa_de_administrador() -> String {
    "O Caçador de DPC precisa do Otimiza aberto como administrador: o Windows só abre o rastreamento do kernel com \
     essa permissão. Nada foi medido."
        .to_string()
}

pub fn texto_sessao_ocupada(kernel_logger: bool) -> String {
    if kernel_logger {
        "Outra ferramenta está usando o rastreamento do kernel agora (LatencyMon, WPR ou xperf, por exemplo). Feche \
         e tente de novo; se nada estiver aberto, reiniciar o PC libera."
            .to_string()
    } else {
        "O Windows já tem o máximo de sessões de rastreamento do sistema abertas (LatencyMon, WPR, Process Monitor). \
         Feche-as e tente de novo."
            .to_string()
    }
}

// ---------------------------------------------------------------------------------------------------------------
// A captura de verdade. Só no Windows; exige administrador.
// ---------------------------------------------------------------------------------------------------------------

#[cfg(windows)]
pub use maquina::{cancelar, capturar};

#[cfg(windows)]
mod maquina {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use windows_sys::Win32::System::Diagnostics::Etw::{
        CloseTrace, ControlTraceW, OpenTraceW, ProcessTrace, StartTraceW, CONTROLTRACE_HANDLE,
        EVENT_HEADER_FLAG_32_BIT_HEADER, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_FLAG_DPC,
        EVENT_TRACE_FLAG_IMAGE_LOAD, EVENT_TRACE_FLAG_INTERRUPT, EVENT_TRACE_LOGFILEW, EVENT_TRACE_PROPERTIES,
        EVENT_TRACE_REAL_TIME_MODE, EVENT_TRACE_SYSTEM_LOGGER_MODE, ImageLoadGuid, PerfInfoGuid,
        PROCESS_TRACE_MODE_EVENT_RECORD, PROCESS_TRACE_MODE_RAW_TIMESTAMP, PROCESS_TRACE_MODE_REAL_TIME,
        SystemTraceControlGuid, WNODE_FLAG_TRACED_GUID,
    };

    /// Nome fixo: uma sessão nossa que sobrou de uma execução que caiu é reconhecida e parada.
    const NOME_SESSAO: &str = "Otimiza-DPC";
    const NOME_KERNEL_LOGGER: &str = "NT Kernel Logger";
    /// GUID próprio da sessão de sistema (o do NT Kernel Logger é fixo e só dele).
    const GUID_DA_SESSAO: windows_sys::core::GUID =
        windows_sys::core::GUID::from_u128(0x4f1c2a7e_3b6d_4e8a_9c51_0d7e2b6a9f13);

    static CAPTURANDO: AtomicBool = AtomicBool::new(false);
    static CANCELAR: AtomicBool = AtomicBool::new(false);

    /// Pede para a captura em andamento terminar já, com o que mediu até agora.
    pub fn cancelar() {
        CANCELAR.store(true, Ordering::SeqCst);
    }

    fn para_utf16(texto: &str) -> Vec<u16> {
        texto.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// O nome vai na mesma alocação, logo depois da estrutura: por isso um vetor de bytes. Folga para o Windows
    /// escrever o nome de volta no STOP/QUERY.
    fn montar_propriedades(nome: &[u16], kernel_logger: bool) -> Vec<u8> {
        let tamanho_struct = std::mem::size_of::<EVENT_TRACE_PROPERTIES>();
        let total = tamanho_struct + 1024 * 2;
        let mut buffer = vec![0u8; total];
        let nucleos = num_cpus::get() as u32;
        unsafe {
            let p = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
            (*p).Wnode.BufferSize = total as u32;
            (*p).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
            (*p).Wnode.ClientContext = 1; /* QPC: a resolução que a DPC pede */
            (*p).Wnode.Guid = if kernel_logger {
                SystemTraceControlGuid
            } else {
                GUID_DA_SESSAO
            };
            (*p).LogFileMode = EVENT_TRACE_REAL_TIME_MODE
                | if kernel_logger {
                    0
                } else {
                    EVENT_TRACE_SYSTEM_LOGGER_MODE
                };
            (*p).EnableFlags = EVENT_TRACE_FLAG_DPC | EVENT_TRACE_FLAG_INTERRUPT | EVENT_TRACE_FLAG_IMAGE_LOAD;
            // DPC e interrupção chegam a dezenas de milhares por segundo. 128 KB por buffer, no máximo
            // ~(2·núcleos + 64) buffers: poucos MB de memória do kernel, e o relatório diz se ainda assim perdeu.
            (*p).BufferSize = 128;
            (*p).MinimumBuffers = (2 * nucleos).max(16);
            (*p).MaximumBuffers = (2 * nucleos).max(16) + 64;
            (*p).FlushTimer = 1;
            (*p).LoggerNameOffset = tamanho_struct as u32;
            if !kernel_logger {
                std::ptr::copy_nonoverlapping(
                    nome.as_ptr(),
                    buffer.as_mut_ptr().add(tamanho_struct) as *mut u16,
                    nome.len(),
                );
            }
        }
        buffer
    }

    fn parar_por_nome(nome: &str, kernel_logger: bool) -> (u32, u32, u32) {
        let n = para_utf16(nome);
        let mut props = montar_propriedades(&n, kernel_logger);
        let codigo = unsafe {
            ControlTraceW(
                CONTROLTRACE_HANDLE { Value: 0 },
                n.as_ptr(),
                props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
                EVENT_TRACE_CONTROL_STOP,
            )
        };
        let p = props.as_ptr() as *const EVENT_TRACE_PROPERTIES;
        unsafe { (codigo, (*p).EventsLost, (*p).RealTimeBuffersLost) }
    }

    /// Guarda RAII: sessão de kernel esquecida fica ligada até o PC reiniciar. Para no `Drop` (erro, retorno cedo,
    /// pânico), e `parar` devolve as perdas que o Windows contou.
    struct Sessao {
        nome: &'static str,
        kernel_logger: bool,
        ativa: bool,
    }

    impl Sessao {
        fn parar(&mut self) -> (u32, u32) {
            if !self.ativa {
                return (0, 0);
            }
            self.ativa = false;
            let (_, perdidos, buffers) = parar_por_nome(self.nome, self.kernel_logger);
            (perdidos, buffers)
        }
    }

    impl Drop for Sessao {
        fn drop(&mut self) {
            self.parar();
        }
    }

    fn iniciar(nome: &'static str, kernel_logger: bool) -> u32 {
        let n = para_utf16(nome);
        let mut props = montar_propriedades(&n, kernel_logger);
        let mut h = CONTROLTRACE_HANDLE { Value: 0 };
        unsafe { StartTraceW(&mut h, n.as_ptr(), props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES) }
    }

    fn abrir_sessao() -> Result<Sessao, ResultadoDaCaptura> {
        // Sobra de uma execução nossa que caiu: tem o nosso nome, e é nossa para parar.
        parar_por_nome(NOME_SESSAO, false);

        let mut codigo = iniciar(NOME_SESSAO, false);
        if interpretar_inicio(codigo, true) == InicioDaSessao::SobraNossa {
            parar_por_nome(NOME_SESSAO, false);
            codigo = iniciar(NOME_SESSAO, false);
        }
        let (nome, kernel_logger, decisao) = match interpretar_inicio(codigo, true) {
            InicioDaSessao::TentarKernelLogger => {
                let c = iniciar(NOME_KERNEL_LOGGER, true);
                (NOME_KERNEL_LOGGER, true, interpretar_inicio(c, false))
            }
            outra => (NOME_SESSAO, false, outra),
        };
        match decisao {
            InicioDaSessao::Comecou => Ok(Sessao {
                nome,
                kernel_logger,
                ativa: true,
            }),
            InicioDaSessao::PrecisaDeAdministrador => Err(ResultadoDaCaptura::PrecisaDeAdministrador {
                texto: texto_precisa_de_administrador(),
            }),
            InicioDaSessao::Ocupada | InicioDaSessao::SobraNossa => Err(ResultadoDaCaptura::SessaoOcupada {
                texto: texto_sessao_ocupada(kernel_logger),
            }),
            InicioDaSessao::TentarKernelLogger => Err(ResultadoDaCaptura::NaoSuportado {
                texto: "Este Windows não abre a sessão de rastreamento do kernel que a captura usa.".to_string(),
            }),
            InicioDaSessao::Falhou(c) => Err(ResultadoDaCaptura::Falhou {
                etapa: "iniciar a sessão".to_string(),
                codigo: c,
                texto: format!("O Windows recusou abrir a sessão de rastreamento (código {}).", c),
            }),
        }
    }

    /// Roda na thread do `ProcessTrace`: o `UserContext` aponta para o `Agregador` dessa mesma thread, então não há
    /// trava nem disputa. `catch_unwind`: pânico atravessando a fronteira com o Windows derrubaria o processo e
    /// deixaria a sessão viva.
    unsafe extern "system" fn ao_receber_evento(registro: *mut EVENT_RECORD) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe { tratar(registro) }));
    }

    unsafe fn tratar(registro: *mut EVENT_RECORD) {
        let Some(r) = registro.as_ref() else {
            return;
        };
        let ag = r.UserContext as *mut Agregador;
        let Some(ag) = ag.as_mut() else {
            return;
        };
        if r.UserData.is_null() || r.UserDataLength == 0 {
            return;
        }
        let dados = std::slice::from_raw_parts(r.UserData as *const u8, r.UserDataLength as usize);
        let cab = &r.EventHeader;
        let de_64 = (cab.Flags as u32 & EVENT_HEADER_FLAG_32_BIT_HEADER) == 0;
        let opcode = cab.EventDescriptor.Opcode;

        if guid_igual(&cab.ProviderId, &PerfInfoGuid) {
            if let Some(tipo) = tipo_do_opcode_perfinfo(opcode) {
                if let Some((inicio, rotina)) = ler_dpc_ou_isr(dados, de_64) {
                    ag.registrar(tipo, rotina, inicio, cab.TimeStamp);
                }
            }
        } else if guid_igual(&cab.ProviderId, &ImageLoadGuid) && opcode_de_imagem_carregada(opcode) {
            if let Some((base, tamanho, caminho)) = ler_imagem(dados, de_64, cab.EventDescriptor.Version) {
                ag.registrar_imagem(base, tamanho, &caminho);
            }
        }
    }

    fn guid_igual(a: &windows_sys::core::GUID, b: &windows_sys::core::GUID) -> bool {
        a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
    }

    /// Os drivers que o Windows lista, com a base. Sem tamanho: só serve de nome aproximado quando o evento de
    /// imagem não chegou.
    fn drivers_do_windows() -> Vec<(u64, String)> {
        use windows_sys::Win32::System::ProcessStatus::{EnumDeviceDrivers, GetDeviceDriverBaseNameW};
        let mut bases: Vec<*mut core::ffi::c_void> = vec![std::ptr::null_mut(); 1024];
        let mut preciso = 0u32;
        let ok = unsafe {
            EnumDeviceDrivers(
                bases.as_mut_ptr(),
                (bases.len() * std::mem::size_of::<usize>()) as u32,
                &mut preciso,
            )
        };
        if ok == 0 {
            return Vec::new();
        }
        let n = (preciso as usize / std::mem::size_of::<usize>()).min(bases.len());
        bases[..n]
            .iter()
            .filter(|b| !b.is_null())
            .filter_map(|&b| {
                let mut nome = [0u16; 260];
                let len = unsafe { GetDeviceDriverBaseNameW(b, nome.as_mut_ptr(), nome.len() as u32) };
                (len > 0).then(|| {
                    (
                        b as u64,
                        String::from_utf16_lossy(&nome[..len as usize]).to_lowercase(),
                    )
                })
            })
            .collect()
    }

    /// Captura por `segundos` (entre 5 e 60) e devolve o relatório. **Bloqueia**: fora do runtime assíncrono.
    pub fn capturar(segundos: u64) -> ResultadoDaCaptura {
        if !crate::modules::windows::registry::is_elevated() {
            return ResultadoDaCaptura::PrecisaDeAdministrador {
                texto: texto_precisa_de_administrador(),
            };
        }
        if CAPTURANDO.swap(true, Ordering::SeqCst) {
            return ResultadoDaCaptura::JaEmAndamento {
                texto: "Já há uma captura de DPC em andamento.".to_string(),
            };
        }
        CANCELAR.store(false, Ordering::SeqCst);
        let r = capturar_interno(segundos.clamp(SEGUNDOS_MINIMO, SEGUNDOS_MAXIMO));
        CAPTURANDO.store(false, Ordering::SeqCst);
        r
    }

    struct Coleta {
        agregador: Box<Agregador>,
        relogio: Relogio,
    }

    fn capturar_interno(segundos: u64) -> ResultadoDaCaptura {
        let mut sessao = match abrir_sessao() {
            Ok(s) => s,
            Err(r) => return r,
        };
        let nome = sessao.nome;

        // A thread avisa se abriu o consumidor; sem isso uma falha de abertura esperaria a captura inteira.
        let (aviso, abriu) = std::sync::mpsc::channel::<Result<(), u32>>();
        let coletor = std::thread::spawn(move || -> Option<Coleta> {
            let n = para_utf16(nome);
            let mut arquivo: EVENT_TRACE_LOGFILEW = unsafe { std::mem::zeroed() };
            arquivo.LoggerName = n.as_ptr() as *mut u16;
            // Carimbo CRU: sem isto o Windows converte o carimbo do evento para hora do sistema, e ele deixa de
            // estar no mesmo relógio do `InitialTime`, que vem cru dentro do evento.
            arquivo.Anonymous1.ProcessTraceMode =
                PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD | PROCESS_TRACE_MODE_RAW_TIMESTAMP;
            arquivo.Anonymous2.EventRecordCallback = Some(ao_receber_evento);
            // Frequência ainda desconhecida: o cabeçalho só chega no `OpenTrace`. Corrigida logo abaixo, antes de
            // qualquer evento (os eventos só vêm dentro do `ProcessTrace`).
            // Ponteiro cru dono do agregador até o `ProcessTrace` voltar: o Windows o devolve em cada evento.
            let agregador = Box::into_raw(Box::new(Agregador::novo(0)));
            arquivo.Context = agregador as *mut core::ffi::c_void;

            let consumidor = unsafe { OpenTraceW(&mut arquivo) };
            if consumidor.Value == u64::MAX {
                let codigo = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                drop(unsafe { Box::from_raw(agregador) });
                let _ = aviso.send(Err(codigo));
                return None;
            }
            let relogio = Relogio::do_cabecalho(arquivo.LogfileHeader.ReservedFlags);
            let mut frequencia = arquivo.LogfileHeader.PerfFreq;
            if frequencia <= 0 {
                frequencia = crate::modules::windows::frames::frequencia_qpc().unwrap_or(0);
            }
            unsafe { (*agregador).frequencia = frequencia };
            let _ = aviso.send(Ok(()));

            // Depois do `CloseTrace` nenhum evento chega mais: o agregador volta a ter dono.
            let agregador = unsafe {
                ProcessTrace(&consumidor, 1, std::ptr::null(), std::ptr::null());
                CloseTrace(consumidor);
                Box::from_raw(agregador)
            };
            Some(Coleta { agregador, relogio })
        });

        match abriu.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {}
            Ok(Err(codigo)) => {
                sessao.parar();
                let _ = coletor.join();
                return ResultadoDaCaptura::Falhou {
                    etapa: "ler a sessão".to_string(),
                    codigo,
                    texto: format!("A sessão abriu, mas não deu para ler os eventos (código {}).", codigo),
                };
            }
            Err(_) => {
                sessao.parar();
                return ResultadoDaCaptura::Falhou {
                    etapa: "ler a sessão".to_string(),
                    codigo: 0,
                    texto: "O leitor de eventos não respondeu em 5 segundos.".to_string(),
                };
            }
        }

        let cronometro = Instant::now();
        let prazo = Duration::from_secs(segundos);
        while cronometro.elapsed() < prazo && !CANCELAR.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(100));
        }
        let duracao_s = cronometro.elapsed().as_secs_f64();

        let (perdidos, buffers) = sessao.parar();
        let Ok(Some(coleta)) = coletor.join() else {
            return ResultadoDaCaptura::Falhou {
                etapa: "ler a sessão".to_string(),
                codigo: 0,
                texto: "O leitor de eventos terminou sem resultado.".to_string(),
            };
        };

        let ctx = ContextoDaCaptura {
            duracao_s,
            nucleos: num_cpus::get() as u32,
            relogio: coleta.relogio,
            eventos_perdidos: perdidos,
            buffers_perdidos: buffers,
            sessao: if sessao.kernel_logger {
                "NT Kernel Logger".to_string()
            } else {
                "logger de sistema próprio".to_string()
            },
        };
        let aproximadas = drivers_do_windows();
        ResultadoDaCaptura::Medido {
            relatorio: analisar(&coleta.agregador, &ctx, &aproximadas),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FREQ: i64 = 10_000_000; // 100 ns por tique, o QPC do Windows 10/11
    const NVIDIA: u64 = 0xFFFF_F800_1000_0000;
    const USB: u64 = 0xFFFF_F800_2000_0000;
    const REDE: u64 = 0xFFFF_F800_3000_0000;

    fn ctx(s: f64) -> ContextoDaCaptura {
        ContextoDaCaptura {
            duracao_s: s,
            nucleos: 8,
            relogio: Relogio::Qpc,
            eventos_perdidos: 0,
            buffers_perdidos: 0,
            sessao: "teste".to_string(),
        }
    }

    fn com_imagens() -> Agregador {
        let mut a = Agregador::novo(FREQ);
        a.registrar_imagem(NVIDIA, 0x100_0000, r"\SystemRoot\System32\DriverStore\FileRepository\nv\nvlddmkm.sys");
        a.registrar_imagem(USB, 0x10_0000, r"\SystemRoot\System32\drivers\USBXHCI.SYS");
        a.registrar_imagem(REDE, 0x10_0000, r"\SystemRoot\System32\drivers\ndis.sys");
        a
    }

    /// Duração em µs → par início/fim em tiques.
    fn evento(a: &mut Agregador, tipo: TipoDeRotina, rotina: u64, em: i64, us: i64) {
        a.registrar(tipo, rotina, em, em + us * 10);
    }

    #[test]
    fn nenhum_pico_e_um_resultado_bom_e_valido() {
        let mut a = com_imagens();
        for i in 0..10_000 {
            evento(&mut a, TipoDeRotina::Dpc, NVIDIA + 0x40, i * 1000, 20);
            evento(&mut a, TipoDeRotina::Isr, USB + 0x10, i * 1000 + 500, 5);
        }
        let r = analisar(&a, &ctx(30.0), &[]);
        assert!(matches!(r.veredito, Veredito::NenhumPico { observacao: None, .. }));
        assert_eq!(r.eventos, 20_000);
        assert_eq!(r.origem, "MEDIDO");
        assert!(r.drivers.iter().all(|d| d.nivel == Nivel::Saudavel && d.acao.is_none()));
        let nv = r.drivers.iter().find(|d| d.arquivo == "nvlddmkm.sys").unwrap();
        assert_eq!(nv.contagem, 10_000);
        assert_eq!(nv.contagem_dpc, 10_000);
        assert_eq!(nv.atribuicao, Atribuicao::Exata);
        // 10 000 × 20 µs = 200 ms em 30 s = 0,667% de um núcleo.
        assert!((nv.tempo_total_ms - 200.0).abs() < 1e-6);
        assert!((nv.pct_de_um_nucleo - 0.667).abs() < 0.001);
        assert_eq!(nv.maximo_us, 20.0);
        assert!((r.resolucao_ns - 100.0).abs() < 1e-9);
    }

    #[test]
    fn driver_de_video_com_picos_repetidos_vira_culpado_com_acao_de_video() {
        let mut a = com_imagens();
        for i in 0..1000 {
            evento(&mut a, TipoDeRotina::Dpc, NVIDIA + 0x40, i * 10_000, 30);
        }
        evento(&mut a, TipoDeRotina::Dpc, NVIDIA + 0x80, 50_000_000, 1500);
        evento(&mut a, TipoDeRotina::Dpc, NVIDIA + 0x80, 60_000_000, 1200);
        let r = analisar(&a, &ctx(30.0), &[]);
        let Veredito::DriversComPico { culpados, texto } = &r.veredito else {
            panic!("esperava culpado: {:?}", r.veredito)
        };
        assert_eq!(culpados.len(), 1);
        assert!(culpados[0].contains("nvlddmkm.sys"));
        assert!(texto.contains("driver de vídeo limpo"));
        let nv = &r.drivers[0];
        assert_eq!(nv.nivel, Nivel::Pico);
        assert_eq!(nv.acima_1ms, 2);
        assert_eq!(nv.acima_2ms, 0);
        assert_eq!(nv.acima_500us, 2);
        assert_eq!(nv.maximo_us, 1500.0);
        // As duas rotinas do mesmo driver somam.
        assert_eq!(nv.contagem, 1002);
        assert_eq!(r.picos.len(), 2);
        assert!(r.picos.iter().all(|p| p.arquivo == "nvlddmkm.sys"));
    }

    #[test]
    fn um_unico_evento_acima_de_1ms_e_atencao_nao_culpa() {
        let mut a = com_imagens();
        evento(&mut a, TipoDeRotina::Dpc, REDE + 0x20, 0, 1100);
        evento(&mut a, TipoDeRotina::Dpc, REDE + 0x20, 100_000, 10);
        let r = analisar(&a, &ctx(30.0), &[]);
        let Veredito::NenhumPico { observacao: Some(obs), .. } = &r.veredito else {
            panic!("esperava sem pico com observação: {:?}", r.veredito)
        };
        assert!(obs.contains("ndis.sys"));
        assert_eq!(r.drivers[0].nivel, Nivel::Atencao);
        assert!(r.drivers[0].acao.as_deref().unwrap().contains("cabo"));
    }

    #[test]
    fn qualquer_evento_acima_de_2ms_e_pico() {
        let mut a = com_imagens();
        evento(&mut a, TipoDeRotina::Isr, USB + 0x10, 0, 2500);
        let r = analisar(&a, &ctx(30.0), &[]);
        assert!(matches!(r.veredito, Veredito::DriversComPico { .. }));
        assert_eq!(r.drivers[0].categoria, Categoria::Usb);
        assert_eq!(r.drivers[0].contagem_isr, 1);
        assert!(r.drivers[0].acao.as_deref().unwrap().contains("Desconecte"));
    }

    #[test]
    fn sem_eventos_nao_e_saudavel() {
        let a = com_imagens();
        let r = analisar(&a, &ctx(30.0), &[]);
        assert!(matches!(r.veredito, Veredito::SemEventos { .. }));
    }

    #[test]
    fn rotina_fora_de_modulo_nao_vira_driver_chutado() {
        let mut a = com_imagens();
        evento(&mut a, TipoDeRotina::Dpc, 0xFFFF_F800_9000_0000, 0, 3000);
        let r = analisar(&a, &ctx(30.0), &[]);
        assert_eq!(r.drivers[0].arquivo, "");
        assert_eq!(r.drivers[0].atribuicao, Atribuicao::SemModulo);
        assert_eq!(r.drivers[0].categoria, Categoria::SemModulo);
    }

    #[test]
    fn sem_evento_de_imagem_usa_a_lista_do_windows_como_aproximada() {
        let mut a = Agregador::novo(FREQ);
        evento(&mut a, TipoDeRotina::Dpc, NVIDIA + 0x1234, 0, 10);
        let lista = vec![(USB, "usbxhci.sys".to_string()), (NVIDIA, "nvlddmkm.sys".to_string())];
        let r = analisar(&a, &ctx(10.0), &lista);
        assert_eq!(r.drivers[0].arquivo, "nvlddmkm.sys");
        assert_eq!(r.drivers[0].atribuicao, Atribuicao::Aproximada);
        assert!(r.avisos.iter().any(|x| x.contains("aproximados")));
    }

    #[test]
    fn carimbo_invertido_ou_absurdo_e_descartado_e_contado() {
        let mut a = com_imagens();
        a.registrar(TipoDeRotina::Dpc, NVIDIA, 1000, 500);
        a.registrar(TipoDeRotina::Dpc, NVIDIA, 0, FREQ * 5); // 5 s
        evento(&mut a, TipoDeRotina::Dpc, NVIDIA, 0, 10);
        let r = analisar(&a, &ctx(30.0), &[]);
        assert_eq!(r.eventos, 3);
        assert_eq!(r.descartados, 2);
        assert_eq!(r.drivers[0].contagem, 1);
        assert!(r.avisos.iter().any(|x| x.contains("inconsistente")));
    }

    #[test]
    fn duracao_de_zero_tique_e_abaixo_da_resolucao_nao_zero_escondido() {
        let mut a = com_imagens();
        a.registrar(TipoDeRotina::Isr, USB, 100, 100);
        let r = analisar(&a, &ctx(30.0), &[]);
        assert_eq!(r.drivers[0].abaixo_da_resolucao, 1);
        assert_eq!(r.drivers[0].contagem, 1);
    }

    #[test]
    fn p99_e_o_teto_do_balde_e_nunca_passa_do_maximo() {
        let mut a = com_imagens();
        for i in 0..990 {
            evento(&mut a, TipoDeRotina::Dpc, NVIDIA, i * 1000, 10);
        }
        for i in 0..10 {
            evento(&mut a, TipoDeRotina::Dpc, NVIDIA, 10_000_000 + i * 1000, 400);
        }
        let r = analisar(&a, &ctx(30.0), &[]);
        let d = &r.drivers[0];
        // Posto 990 de 1000: ainda está nos de 10 µs; o teto do balde fica até ~9% acima.
        assert!(d.p99_ate_us >= 10.0 && d.p99_ate_us <= 11.0, "{}", d.p99_ate_us);

        let mut b = com_imagens();
        evento(&mut b, TipoDeRotina::Dpc, NVIDIA, 0, 400);
        let r = analisar(&b, &ctx(30.0), &[]);
        assert_eq!(r.drivers[0].p99_ate_us, 400.0);
    }

    #[test]
    fn baldes_crescem_e_cobrem_ate_um_segundo() {
        assert_eq!(balde(0), 0);
        assert_eq!(balde(999), 0);
        assert_eq!(balde(1000), 1);
        assert!(balde(1_000_000) > balde(500_000));
        assert_eq!(balde(u64::MAX), BALDES - 1);
        for ns in [1500u64, 20_000, 700_000, 999_000_000] {
            let i = balde(ns);
            assert!(teto_do_balde_ns(i) >= ns as f64, "{ns} cabe no balde {i}");
        }
    }

    #[test]
    fn memoria_limitada_rotinas_alem_do_teto_vao_para_um_balde_so() {
        let mut a = Agregador::novo(FREQ);
        for i in 0..(MAXIMO_DE_ROTINAS as u64 + 100) {
            a.registrar(TipoDeRotina::Dpc, INICIO_DO_KERNEL + i * 16, 0, 10);
        }
        assert_eq!(a.por_rotina.len(), MAXIMO_DE_ROTINAS);
        let r = analisar(&a, &ctx(30.0), &[]);
        let total: u32 = r.drivers.iter().map(|d| d.contagem).sum();
        assert_eq!(total, MAXIMO_DE_ROTINAS as u32 + 100);
    }

    #[test]
    fn picos_tem_teto_e_os_alem_dele_sao_contados() {
        let mut a = com_imagens();
        for i in 0..(MAXIMO_DE_PICOS as i64 + 5) {
            evento(&mut a, TipoDeRotina::Dpc, NVIDIA, i * 100_000, 600);
        }
        let r = analisar(&a, &ctx(30.0), &[]);
        assert_eq!(r.picos.len(), MAXIMO_DE_PICOS);
        assert_eq!(r.picos_sem_carimbo, 5);
        assert_eq!(r.drivers[0].acima_500us as usize, MAXIMO_DE_PICOS + 5);
    }

    #[test]
    fn picos_na_janela_cruza_pelo_relogio_qpc() {
        let mut a = com_imagens();
        evento(&mut a, TipoDeRotina::Dpc, NVIDIA, 1_000_000, 1500); // fim 1_015_000
        evento(&mut a, TipoDeRotina::Dpc, USB, 5_000_000, 1500);
        let r = analisar(&a, &ctx(30.0), &[]);
        let na = picos_na_janela(&r, 1_010_000, 1_100_000);
        assert_eq!(na.len(), 1);
        assert_eq!(na[0].arquivo, "nvlddmkm.sys");
        assert!(picos_na_janela(&r, 2_000_000, 3_000_000).is_empty());
    }

    #[test]
    fn imagens_de_usuario_sao_ignoradas() {
        let mut a = Agregador::novo(FREQ);
        a.registrar_imagem(0x7FF8_0000_0000, 0x1000, r"C:\Windows\System32\ntdll.dll");
        a.registrar_imagem(NVIDIA, 0, "nvlddmkm.sys");
        assert_eq!(a.imagens(), 0);
    }

    #[test]
    fn le_dpc_de_64_e_de_32_bits() {
        let mut b = Vec::new();
        b.extend_from_slice(&123_456i64.to_le_bytes());
        b.extend_from_slice(&NVIDIA.to_le_bytes());
        assert_eq!(ler_dpc_ou_isr(&b, true), Some((123_456, NVIDIA)));
        let mut c = Vec::new();
        c.extend_from_slice(&7i64.to_le_bytes());
        c.extend_from_slice(&0x8000_1000u32.to_le_bytes());
        assert_eq!(ler_dpc_ou_isr(&c, false), Some((7, 0x8000_1000)));
        assert_eq!(ler_dpc_ou_isr(&b[..10], true), None);
    }

    fn imagem_64(base: u64, tamanho: u64, caminho: &str) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&base.to_le_bytes());
        b.extend_from_slice(&tamanho.to_le_bytes());
        b.extend_from_slice(&[0u8; 16]); // ProcessId, CheckSum, TimeDateStamp, Reserved0
        b.extend_from_slice(&[0u8; 8]); // DefaultBase
        b.extend_from_slice(&[0u8; 16]); // Reserved1–4
        for u in caminho.encode_utf16().chain(std::iter::once(0)) {
            b.extend_from_slice(&u.to_le_bytes());
        }
        b
    }

    #[test]
    fn le_imagem_versao_3_de_64_bits() {
        let b = imagem_64(NVIDIA, 0x2000, r"\SystemRoot\System32\drivers\ACPI.sys");
        let (base, tam, caminho) = ler_imagem(&b, true, 3).unwrap();
        assert_eq!((base, tam), (NVIDIA, 0x2000));
        assert_eq!(nome_do_arquivo(&caminho), "acpi.sys");
        assert_eq!(ler_imagem(&b, true, 1), None);
        assert_eq!(ler_imagem(&b[..40], true, 3), None);
    }

    #[test]
    fn opcodes_do_kernel() {
        assert_eq!(tipo_do_opcode_perfinfo(68), Some(TipoDeRotina::Dpc));
        assert_eq!(tipo_do_opcode_perfinfo(66), Some(TipoDeRotina::Dpc));
        assert_eq!(tipo_do_opcode_perfinfo(69), Some(TipoDeRotina::Dpc));
        assert_eq!(tipo_do_opcode_perfinfo(67), Some(TipoDeRotina::Isr));
        assert_eq!(tipo_do_opcode_perfinfo(50), Some(TipoDeRotina::Isr));
        assert_eq!(tipo_do_opcode_perfinfo(46), None); // SampledProfile
        assert!(opcode_de_imagem_carregada(3) && opcode_de_imagem_carregada(10));
        assert!(!opcode_de_imagem_carregada(2) && !opcode_de_imagem_carregada(4));
    }

    #[test]
    fn classifica_os_drivers_do_laboratorio_e_os_comuns() {
        let casos = [
            ("nvlddmkm.sys", Categoria::Video),
            ("dxgkrnl.sys", Categoria::Video),
            ("igdkmdn64.sys", Categoria::Video),
            ("USBXHCI.SYS", Categoria::Usb),
            ("ndis.sys", Categoria::Rede),
            ("tcpip.sys", Categoria::Rede),
            ("Netwtw10.sys", Categoria::Rede),
            ("rt640x64.sys", Categoria::Rede),
            ("ACPI.sys", Categoria::Energia),
            ("storport.sys", Categoria::Disco),
            ("stornvme.sys", Categoria::Disco),
            ("HDAudBus.sys", Categoria::Audio),
            ("Wdf01000.sys", Categoria::FrameworkWdf),
            ("ntoskrnl.exe", Categoria::Windows),
            ("algumacoisa.sys", Categoria::Outro),
            ("", Categoria::SemModulo),
        ];
        for (arquivo, esperada) in casos {
            assert_eq!(classificar(arquivo).0, esperada, "{arquivo}");
        }
        assert!(classificar("algumacoisa.sys").1.contains("algumacoisa.sys"));
        assert!(acao(Categoria::Energia).contains("BIOS"));
        assert!(acao(Categoria::Disco).contains("disco"));
    }

    #[test]
    fn interpreta_o_inicio_da_sessao_com_nome() {
        assert_eq!(interpretar_inicio(0, true), InicioDaSessao::Comecou);
        assert_eq!(interpretar_inicio(5, true), InicioDaSessao::PrecisaDeAdministrador);
        assert_eq!(interpretar_inicio(5, false), InicioDaSessao::PrecisaDeAdministrador);
        assert_eq!(interpretar_inicio(183, true), InicioDaSessao::SobraNossa);
        // O NT Kernel Logger com dono: nunca paramos o dos outros.
        assert_eq!(interpretar_inicio(183, false), InicioDaSessao::Ocupada);
        assert_eq!(interpretar_inicio(1450, true), InicioDaSessao::Ocupada);
        assert_eq!(interpretar_inicio(87, true), InicioDaSessao::TentarKernelLogger);
        assert_eq!(interpretar_inicio(87, false), InicioDaSessao::Falhou(87));
        assert_eq!(interpretar_inicio(1234, true), InicioDaSessao::Falhou(1234));
    }

    #[test]
    fn avisa_perda_de_eventos_e_relogio_ruim() {
        let mut a = com_imagens();
        evento(&mut a, TipoDeRotina::Dpc, NVIDIA, 0, 10);
        let mut c = ctx(30.0);
        c.eventos_perdidos = 12;
        c.relogio = Relogio::Sistema;
        let r = analisar(&a, &c, &[]);
        assert!(r.avisos.iter().any(|x| x.contains("piso")));
        assert!(r.avisos.iter().any(|x| x.contains("alta resolução")));
    }

    #[test]
    fn resultado_serializa_com_o_estado_nomeado() {
        let r = ResultadoDaCaptura::PrecisaDeAdministrador {
            texto: texto_precisa_de_administrador(),
        };
        let j = serde_json::to_value(&r).unwrap();
        assert_eq!(j["estado"], "PrecisaDeAdministrador");
        assert!(j["texto"].as_str().unwrap().contains("administrador"));
    }

    /// Captura de verdade nesta máquina. Exige administrador; sem ele, confere o resultado nomeado.
    #[cfg(windows)]
    #[test]
    #[ignore = "lê esta máquina: abre uma sessão de kernel por 10 s (precisa de administrador)"]
    fn captura_real_nesta_maquina() {
        let r = capturar(10);
        println!("{}", serde_json::to_string_pretty(&r).unwrap());
        if !crate::modules::windows::registry::is_elevated() {
            assert!(matches!(r, ResultadoDaCaptura::PrecisaDeAdministrador { .. }));
            return;
        }
        let ResultadoDaCaptura::Medido { relatorio } = r else {
            panic!("não mediu: {r:?}")
        };
        assert!(relatorio.eventos > 0, "nenhum evento");
        assert!(relatorio.modulos_vistos > 0, "nenhum módulo do rundown");
        assert_eq!(relatorio.relogio, Relogio::Qpc);
    }
}
