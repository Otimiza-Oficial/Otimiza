// Medidor de quadros pelo rastreamento de eventos do Windows (`Present` do DXGI), de fora, sem tocar no jogo:
// injeção é o que o anticheat do FiveM procura. Exige administrador; cobre Direct3D 10 em diante. Sem elevação ou
// em D3D9, "não foi possível medir", nunca zero.

#![cfg(target_os = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, ERROR_SUCCESS};
use windows_sys::Win32::System::Diagnostics::Etw::{
    CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace, StartTraceW,
    CONTROLTRACE_HANDLE, EVENT_CONTROL_CODE_ENABLE_PROVIDER, EVENT_RECORD,
    EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_LOGFILEW, EVENT_TRACE_PROPERTIES,
    EVENT_TRACE_REAL_TIME_MODE, PROCESS_TRACE_MODE_EVENT_RECORD, PROCESS_TRACE_MODE_REAL_TIME,
    TRACE_LEVEL_INFORMATION, WNODE_FLAG_TRACED_GUID,
};

/// GUID fixo, publicado pela Microsoft.
const PROVEDOR_DXGI: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0xCA11C036,
    data2: 0x0102,
    data3: 0x4A2D,
    data4: [0xA6, 0xAD, 0xF0, 0x3C, 0xFE, 0xD5, 0xD3, 0xC9],
};

/// O início, não o fim: o fim não chega se o quadro for descartado.
const EVENTO_PRESENT_START: u16 = 42;

/// Fixo: uma sessão de execução anterior que morreu continua viva, e o nome permite derrubá-la.
const NOME_SESSAO: &str = "Otimiza-Quadros";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMeasurement {
    pub fps: f64,
    pub frames: u64,
    pub seconds: f64,
    pub process: String,
    pub pid: u32,

    // A média mal se move com disputa de memória ou processador; o engasgo é o que o cliente sente.
    pub frametime_mediano_ms: f64,
    /// A média do 1% pior, em FPS: o que os analistas usam.
    pub low_1pct: f64,
    pub engasgos_por_minuto: f64,
    pub detalhe_confiavel: bool,
    /// Calculada por `core::fluidez`. `None` em medição antiga ou sem quadro.
    #[serde(default)]
    pub saude: Option<crate::core::fluidez::SaudeDosQuadros>,
}

// A função de retorno do Windows não tem contexto utilizável com segurança em Rust: estado global, serializado
// por `MEDINDO`.

static MEDINDO: AtomicBool = AtomicBool::new(false);
static CONTADOR: AtomicU64 = AtomicU64::new(0);
static PID_ALVO: AtomicU32 = AtomicU32::new(0);

/// Geração de quadros externa (Lossless Scaling desenha na janela DELE): os dois processos na MESMA sessão, senão
/// a variação do jogo viraria "efeito da geração". Zero é sem segundo processo.
static PID_SECUNDARIO: AtomicU32 = AtomicU32::new(0);
static CONTADOR_SECUNDARIO: AtomicU64 = AtomicU64::new(0);
static INSTANTES_SECUNDARIOS: Mutex<Vec<i64>> = Mutex::new(Vec::new());

/// O carimbo vem porque a sessão é criada com `ClientContext = 1`. PRÉ-ALOCADO: alocar na função de retorno
/// apareceria como engasgo na própria medição.
static INSTANTES: Mutex<Vec<i64>> = Mutex::new(Vec::new());

/// Trinta minutos a 500 FPS, ~7 MB.
const MAXIMO_DE_AMOSTRAS: usize = 900_000;

unsafe extern "system" fn ao_receber_evento(registro: *mut EVENT_RECORD) {
    if registro.is_null() {
        return;
    }

    let cabecalho = &(*registro).EventHeader;

    // Sem os dois filtros, a conta somaria tudo que desenha na máquina.
    if cabecalho.EventDescriptor.Id != EVENTO_PRESENT_START {
        return;
    }

    let pid = cabecalho.ProcessId;
    let (contador, lista) = if pid == PID_ALVO.load(Ordering::Relaxed) {
        (&CONTADOR, &INSTANTES)
    } else {
        let segundo = PID_SECUNDARIO.load(Ordering::Relaxed);
        if segundo == 0 || pid != segundo {
            return;
        }
        (&CONTADOR_SECUNDARIO, &INSTANTES_SECUNDARIOS)
    };

    contador.fetch_add(1, Ordering::Relaxed);

    // `try_lock`: perder uma amostra é melhor que segurar o retorno do rastreamento do Windows.
    if let Ok(mut instantes) = lista.try_lock() {
        if instantes.len() < MAXIMO_DE_AMOSTRAS {
            instantes.push(cabecalho.TimeStamp);
        }
    }
}

/// Com 600 quadros o "1% pior" são seis: abaixo disto mostra a média e diz que falta amostra.
const AMOSTRAS_PARA_DETALHE: usize = 2_000;

/// **Pura**: testável sem abrir jogo.
pub fn estatistica(mut intervalos_ms: Vec<f64>) -> (f64, f64, f64, bool) {
    if intervalos_ms.is_empty() {
        return (0.0, 0.0, 0.0, false);
    }

    intervalos_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mediana = intervalos_ms[intervalos_ms.len() / 2];

    // 1% low é a MÉDIA do 1% pior, não o P99: trocá-los publica um número que não bate com o de ninguém.
    let quantos_piores = (intervalos_ms.len() / 100).max(1);
    let piores = &intervalos_ms[intervalos_ms.len() - quantos_piores..];
    let media_dos_piores = piores.iter().sum::<f64>() / quantos_piores as f64;

    let low_1pct = if media_dos_piores > 0.0 {
        1000.0 / media_dos_piores
    } else {
        0.0
    };

    // Relativo à partida: um limiar fixo acusaria sempre a 30 FPS e nunca a 240.
    let limiar = (mediana * 2.0).max(50.0);
    let engasgos = intervalos_ms.iter().filter(|ms| **ms > limiar).count();
    let duracao_minutos = intervalos_ms.iter().sum::<f64>() / 60_000.0;

    let por_minuto = if duracao_minutos > 0.0 {
        engasgos as f64 / duracao_minutos
    } else {
        0.0
    };

    (
        (mediana * 100.0).round() / 100.0,
        (low_1pct * 10.0).round() / 10.0,
        (por_minuto * 10.0).round() / 10.0,
        intervalos_ms.len() >= AMOSTRAS_PARA_DETALHE,
    )
}

/// **Pura.** A mediana ignora os engasgos; a média é puxada por eles; o afastamento é o sintoma. P95 e P99 são
/// percentis, não médias de cauda. `None` com amostra curta: cauda de poucas dezenas de quadros é ruído.
pub fn percentis(intervalos_ms: &[f64]) -> Option<(f64, f64, f64)> {
    if intervalos_ms.len() < AMOSTRAS_PARA_DETALHE {
        return None;
    }

    // A mesma conta de `core::estatistica::percentil`, para o Mapa e a medição automática não darem P99 diferentes.
    use crate::core::estatistica::{media, percentil};
    let arred = |v: f64| (v * 100.0).round() / 100.0;

    Some((
        arred(media(intervalos_ms)?),
        arred(percentil(intervalos_ms, 95.0)?),
        arred(percentil(intervalos_ms, 99.0)?),
    ))
}

/// O nome da sessão em UTF-16 vem na mesma alocação, logo depois: por isso um vetor de bytes.
fn montar_propriedades(nome: &[u16]) -> Vec<u8> {
    let tamanho_struct = std::mem::size_of::<EVENT_TRACE_PROPERTIES>();
    let tamanho_nome = nome.len() * 2;
    let total = tamanho_struct + tamanho_nome;

    let mut buffer = vec![0u8; total];

    unsafe {
        let props = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
        (*props).Wnode.BufferSize = total as u32;
        (*props).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        (*props).Wnode.ClientContext = 1; /* relógio de alta resolução */
        (*props).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
        (*props).LoggerNameOffset = tamanho_struct as u32;

        std::ptr::copy_nonoverlapping(
            nome.as_ptr(),
            buffer.as_mut_ptr().add(tamanho_struct) as *mut u16,
            nome.len(),
        );
    }

    buffer
}

fn para_utf16(texto: &str) -> Vec<u16> {
    texto.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Sessão de rastreamento sobrevive ao processo que a criou.
fn derrubar_sessao_antiga() {
    let nome = para_utf16(NOME_SESSAO);
    let mut props = montar_propriedades(&nome);

    unsafe {
        ControlTraceW(
            CONTROLTRACE_HANDLE { Value: 0 },
            nome.as_ptr(),
            props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
            EVENT_TRACE_CONTROL_STOP,
        );
    }
}

pub fn encontrar_processo(prefixo: &str) -> Option<(u32, String)> {
    use sysinfo::System;

    let mut sistema = System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let alvo = prefixo.to_lowercase();

    // O processo que desenha é sempre o maior.
    sistema
        .processes()
        .iter()
        .filter_map(|(pid, processo)| {
            let nome = processo.name().to_string_lossy().to_lowercase();
            nome.contains(&alvo)
                .then(|| (pid.as_u32(), nome, processo.memory()))
        })
        .max_by_key(|(_, _, memoria)| *memoria)
        .map(|(pid, nome, _)| (pid, nome))
}

/// Bloqueia pelo tempo pedido: fora do runtime assíncrono.
pub fn medir(pid: u32, nome: &str, segundos: u64) -> Result<FrameMeasurement, String> {
    medir_par(pid, nome, None, segundos).map(|(principal, _)| principal.resumo)
}

#[derive(Debug, Clone)]
pub struct MedicaoCrua {
    pub resumo: FrameMeasurement,
    pub intervalos_ms: Vec<f64>,
    /// O QUANDO de cada tranco, para cruzar com o disco (shader e asset dão o mesmo buraco). Em unidades do contador,
    /// o mesmo relógio de qualquer amostrador: uma origem própria desalinharia as séries.
    pub trancos_qpc: Vec<i64>,
}

/// O segundo volta `None` sem quadro: gerador parado não vira "zero quadros exibidos".
pub fn medir_par(
    pid: u32,
    nome: &str,
    segundo: Option<(u32, String)>,
    segundos: u64,
) -> Result<(MedicaoCrua, Option<MedicaoCrua>), String> {
    if !super::registry::is_elevated() {
        return Err(
            "Medir quadros exige executar como administrador: o canal de eventos do Windows \
             que informa cada quadro entregue só é aberto com essa permissão."
                .to_string(),
        );
    }

    // Seguro com QUALQUER anticheat: não abre handle, não lê memória, não injeta.
    if let Some(recusa) = super::anticheat::permite(
        super::anticheat::Acao::MedirQuadros,
        &super::anticheat::detectar_agora(),
    )
    .motivo()
    {
        return Err(recusa.to_string());
    }

    if MEDINDO.swap(true, Ordering::SeqCst) {
        return Err("Já existe uma medição de quadros em andamento.".to_string());
    }

    // A partir daqui todo caminho de saída precisa liberar a trava.
    let resultado = medir_interno(pid, nome, segundo, segundos);
    PID_SECUNDARIO.store(0, Ordering::SeqCst);
    MEDINDO.store(false, Ordering::SeqCst);
    resultado
}

fn medir_interno(
    pid: u32,
    nome: &str,
    segundo: Option<(u32, String)>,
    segundos: u64,
) -> Result<(MedicaoCrua, Option<MedicaoCrua>), String> {
    derrubar_sessao_antiga();

    let nome_sessao = para_utf16(NOME_SESSAO);
    let mut props = montar_propriedades(&nome_sessao);
    let mut sessao = CONTROLTRACE_HANDLE { Value: 0 };

    let inicio = unsafe {
        StartTraceW(
            &mut sessao,
            nome_sessao.as_ptr(),
            props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
        )
    };

    if inicio != ERROR_SUCCESS {
        return Err(if inicio == ERROR_ALREADY_EXISTS {
            "Já há uma sessão de medição presa no sistema. Reinicie o PC e tente de novo."
                .to_string()
        } else {
            format!("Não foi possível iniciar a medição (código {}).", inicio)
        });
    }

    let habilitou = unsafe {
        EnableTraceEx2(
            sessao,
            &PROVEDOR_DXGI,
            EVENT_CONTROL_CODE_ENABLE_PROVIDER,
            TRACE_LEVEL_INFORMATION as u8,
            0,
            0,
            0,
            std::ptr::null(),
        )
    };

    if habilitou != ERROR_SUCCESS {
        parar_sessao();
        return Err(format!(
            "Não foi possível escutar os eventos de quadro (código {}).",
            habilitou
        ));
    }

    CONTADOR.store(0, Ordering::SeqCst);
    CONTADOR_SECUNDARIO.store(0, Ordering::SeqCst);
    PID_ALVO.store(pid, Ordering::SeqCst);
    PID_SECUNDARIO.store(segundo.as_ref().map_or(0, |(p, _)| *p), Ordering::SeqCst);

    // `ProcessTrace` só volta quando a sessão para: roda numa thread própria.
    let coletor = std::thread::spawn(move || {
        let mut arquivo: EVENT_TRACE_LOGFILEW = unsafe { std::mem::zeroed() };
        let nome = para_utf16(NOME_SESSAO);

        arquivo.LoggerName = nome.as_ptr() as *mut u16;
        arquivo.Anonymous1.ProcessTraceMode =
            PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
        arquivo.Anonymous2.EventRecordCallback = Some(ao_receber_evento);

        let consumidor = unsafe { OpenTraceW(&mut arquivo) };

        // `INVALID_PROCESSTRACE_HANDLE` é u64::MAX em 64 bits, no campo único da struct.
        if consumidor.Value == u64::MAX {
            return Err(unsafe { GetLastError() });
        }

        unsafe {
            ProcessTrace(&consumidor, 1, std::ptr::null(), std::ptr::null());
            CloseTrace(consumidor);
        }

        Ok(())
    });

    for lista in [&INSTANTES, &INSTANTES_SECUNDARIOS] {
        if let Ok(mut instantes) = lista.lock() {
            instantes.clear();
            instantes.reserve(segundos as usize * 600);
        }
    }

    let cronometro = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_secs(segundos));
    let decorrido = cronometro.elapsed().as_secs_f64();

    parar_sessao();

    let saida_coletor = coletor.join();
    let frames = CONTADOR.load(Ordering::SeqCst);

    if let Ok(Err(codigo)) = saida_coletor {
        return Err(format!(
            "Não foi possível ler os eventos de quadro (código {}).",
            codigo
        ));
    }

    if frames == 0 {
        return Err(format!(
            "Nenhum quadro foi contado em {} segundos. O `{}` pode estar minimizado, parado \
             numa tela de carregamento, ou desenhando por um caminho antigo que este canal \
             não cobre. Preferimos dizer que não medimos a mostrar zero quadros.",
            segundos, nome
        ));
    }

    let principal = montar(
        nome,
        pid,
        frames,
        decorrido,
        intervalos_em_ms(&INSTANTES),
        trancos_qpc(&INSTANTES),
    );

    let secundaria = segundo.and_then(|(pid2, nome2)| {
        let quadros = CONTADOR_SECUNDARIO.load(Ordering::SeqCst);
        (quadros > 0).then(|| {
            montar(
                &nome2,
                pid2,
                quadros,
                decorrido,
                intervalos_em_ms(&INSTANTES_SECUNDARIOS),
                trancos_qpc(&INSTANTES_SECUNDARIOS),
            )
        })
    });

    Ok((principal, secundaria))
}

fn montar(
    nome: &str,
    pid: u32,
    frames: u64,
    decorrido: f64,
    intervalos_ms: Vec<f64>,
    trancos_qpc: Vec<i64>,
) -> MedicaoCrua {
    let (mediana, low_1pct, engasgos, confiavel) = estatistica(intervalos_ms.clone());

    MedicaoCrua {
        resumo: FrameMeasurement {
            fps: frames as f64 / decorrido,
            frames,
            seconds: decorrido,
            process: nome.to_string(),
            pid,
            frametime_mediano_ms: mediana,
            low_1pct,
            engasgos_por_minuto: engasgos,
            detalhe_confiavel: confiavel,
            saude: crate::core::fluidez::analisar(&intervalos_ms),
        },
        intervalos_ms,
        trancos_qpc,
    }
}

/// O MESMO limiar de `estatistica`. O carimbo é o do FIM do quadro demorado, quando a pessoa sentiu.
fn trancos_qpc(lista: &Mutex<Vec<i64>>) -> Vec<i64> {
    let Some(frequencia) = frequencia_qpc() else {
        return Vec::new();
    };
    let Ok(instantes) = lista.lock() else {
        return Vec::new();
    };

    let para_ms = |ticks: i64| ticks as f64 * 1000.0 / frequencia as f64;

    let intervalos: Vec<f64> = instantes
        .windows(2)
        .map(|par| para_ms(par[1] - par[0]))
        .collect();

    let limiar = limiar_de_engasgo(&intervalos);

    instantes
        .windows(2)
        .filter(|par| {
            let ms = para_ms(par[1] - par[0]);
            ms > limiar && ms < 10_000.0
        })
        .map(|par| par[1])
        .collect()
}

/// O dobro da mediana, com piso de 50 ms para jogo muito rápido não acusar oscilação normal.
fn limiar_de_engasgo(intervalos_ms: &[f64]) -> f64 {
    if intervalos_ms.is_empty() {
        return f64::INFINITY;
    }

    let mut ordenados: Vec<f64> = intervalos_ms.to_vec();
    ordenados.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    (ordenados[ordenados.len() / 2] * 2.0).max(50.0)
}

pub fn frequencia_qpc() -> Option<i64> {
    use windows_sys::Win32::System::Performance::QueryPerformanceFrequency;

    let mut frequencia: i64 = 0;
    let ok = unsafe { QueryPerformanceFrequency(&mut frequencia) != 0 };

    (ok && frequencia > 0).then_some(frequencia)
}

/// Público para outro amostrador carimbar no mesmo relógio.
pub fn agora_qpc() -> Option<i64> {
    use windows_sys::Win32::System::Performance::QueryPerformanceCounter;

    let mut agora: i64 = 0;
    let ok = unsafe { QueryPerformanceCounter(&mut agora) != 0 };

    ok.then_some(agora)
}

/// **Pura.** Devolve `(com o disco ocupado, total)`. Não conclui: coincidência não é causa, e disco sempre ocupado
/// faria tudo parecer streaming. `None` sem tranco ou sem amostragem cobrindo a janela.
pub fn trancos_com_disco(
    trancos_qpc: &[i64],
    disco: &[(i64, f64)],
    frequencia: i64,
    janela_ms: f64,
    ocupado_pct: f64,
) -> Option<(usize, usize)> {
    if trancos_qpc.is_empty() || disco.is_empty() || frequencia <= 0 {
        return None;
    }

    let janela_ticks = (janela_ms / 1000.0 * frequencia as f64) as i64;

    let com_disco = trancos_qpc
        .iter()
        .filter(|tranco| {
            // Só dentro da janela: leitura de dois segundos depois não descreve o instante do tranco.
            disco
                .iter()
                .filter(|(quando, _)| (*quando - **tranco).abs() <= janela_ticks)
                .min_by_key(|(quando, _)| (*quando - **tranco).abs())
                .is_some_and(|(_, pct)| *pct >= ocupado_pct)
        })
        .count();

    Some((com_disco, trancos_qpc.len()))
}

/// A frequência do contador varia por máquina: tratá-lo como microssegundo daria números plausíveis e errados.
fn intervalos_em_ms(lista: &Mutex<Vec<i64>>) -> Vec<f64> {
    use windows_sys::Win32::System::Performance::QueryPerformanceFrequency;

    let mut frequencia: i64 = 0;
    unsafe {
        if QueryPerformanceFrequency(&mut frequencia) == 0 || frequencia <= 0 {
            return Vec::new();
        }
    }

    let Ok(instantes) = lista.lock() else {
        return Vec::new();
    };

    instantes
        .windows(2)
        .map(|par| (par[1] - par[0]) as f64 * 1000.0 / frequencia as f64)
        // Negativo ou absurdo é reordenação de evento.
        .filter(|ms| *ms > 0.0 && *ms < 10_000.0)
        .collect()
}

fn parar_sessao() {
    let nome = para_utf16(NOME_SESSAO);
    let mut props = montar_propriedades(&nome);

    unsafe {
        ControlTraceW(
            CONTROLTRACE_HANDLE { Value: 0 },
            nome.as_ptr(),
            props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
            EVENT_TRACE_CONTROL_STOP,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_1pct_do_nucleo_e_o_mesmo_daqui() {
        // Duas contas divergentes dariam dois "1% low" na tela.
        let mut intervalos: Vec<f64> = (0..5_000).map(|i| 8.0 + (i % 7) as f64 * 0.9).collect();
        for i in (0..5_000).step_by(97) {
            intervalos[i] = 31.0;
        }
        let (_, low_aqui, _, _) = estatistica(intervalos.clone());
        let saude = crate::core::fluidez::analisar(&intervalos).unwrap();
        let low_nucleo = (saude.low_1pct_fps.unwrap() * 10.0).round() / 10.0;
        assert_eq!(low_aqui, low_nucleo);
    }

    #[test]
    fn o_um_por_cento_pior_e_media_e_nao_percentil() {
        // 990 quadros a 10 ms e 10 a 100 ms: o 1% pior são os dez de 100 ms, 10 FPS.
        let mut intervalos = vec![10.0; 990];
        intervalos.extend(vec![100.0; 10]);

        let (mediana, low, _, _) = estatistica(intervalos);

        assert_eq!(mediana, 10.0);
        assert_eq!(low, 10.0);
    }

    #[test]
    fn percentil_nao_e_media_de_cauda() {
        // Os mesmos quadros: o P99 ainda é 10 ms.
        let mut intervalos = vec![10.0; 990];
        intervalos.extend(vec![100.0; 10]);

        let longo: Vec<f64> = intervalos.iter().cycle().take(3000).copied().collect();
        let (media, p95, p99) = percentis(&longo).expect("amostra suficiente");

        assert_eq!(p95, 10.0, "95% dos quadros estão em 10 ms");
        assert_eq!(p99, 10.0, "e o P99 também — a cauda é 1% exato");
        assert!(
            media > 10.0 && media < 12.0,
            "a média é puxada pelos trancos: {media}"
        );

        // Cauda de exatamente 5%: o percentil marca a fronteira, não a entrada dela.
        let mut na_fronteira = vec![10.0; 2850];
        na_fronteira.extend(vec![100.0; 150]);
        let (_, p95, _) = percentis(&na_fronteira).expect("amostra suficiente");
        assert_eq!(p95, 10.0);

        let mut com_cauda = vec![10.0; 2820];
        com_cauda.extend(vec![100.0; 180]);
        let (_, p95, p99) = percentis(&com_cauda).expect("amostra suficiente");
        assert_eq!(p95, 100.0);
        assert_eq!(p99, 100.0);
    }

    const HZ: i64 = 1_000_000;

    fn em_ms(ms: f64) -> i64 {
        (ms / 1000.0 * HZ as f64) as i64
    }

    #[test]
    fn tranco_colado_no_disco_ocupado_conta() {
        let trancos = vec![em_ms(1000.0), em_ms(2000.0), em_ms(3000.0)];
        let disco = vec![
            (em_ms(950.0), 90.0),
            (em_ms(1950.0), 85.0),
            (em_ms(2980.0), 95.0),
        ];

        let (com, total) =
            trancos_com_disco(&trancos, &disco, HZ, 300.0, 40.0).expect("há trancos");
        assert_eq!((com, total), (3, 3));
    }

    #[test]
    fn disco_quieto_no_instante_do_tranco_nao_conta() {
        let trancos = vec![em_ms(1000.0), em_ms(2000.0)];
        let disco = vec![
            (em_ms(980.0), 2.0),
            (em_ms(1990.0), 1.0),
            (em_ms(5000.0), 99.0),
        ];

        let (com, total) =
            trancos_com_disco(&trancos, &disco, HZ, 300.0, 40.0).expect("há trancos");
        assert_eq!((com, total), (0, 2));
    }

    #[test]
    fn amostra_fora_da_janela_nao_descreve_o_tranco() {
        // Sem limite de distância, o "mais próximo" contaria.
        let trancos = vec![em_ms(1000.0)];
        let disco = vec![(em_ms(2000.0), 99.0)];

        let (com, _) = trancos_com_disco(&trancos, &disco, HZ, 300.0, 40.0).expect("há trancos");
        assert_eq!(com, 0);
    }

    #[test]
    fn sem_tranco_ou_sem_disco_nao_ha_proporcao() {
        assert_eq!(trancos_com_disco(&[], &[(0, 90.0)], HZ, 300.0, 40.0), None);
        assert_eq!(trancos_com_disco(&[em_ms(1.0)], &[], HZ, 300.0, 40.0), None);
        assert_eq!(
            trancos_com_disco(&[em_ms(1.0)], &[(0, 90.0)], 0, 300.0, 40.0),
            None
        );
    }

    #[test]
    fn limiar_de_engasgo_acompanha_a_partida() {
        assert_eq!(limiar_de_engasgo(&[16.6; 100]), 50.0);
        assert!((limiar_de_engasgo(&[33.0; 100]) - 66.0).abs() < 0.01);
        assert_eq!(limiar_de_engasgo(&[]), f64::INFINITY);
    }

    #[test]
    fn percentil_de_amostra_curta_nao_existe() {
        assert_eq!(percentis(&[16.0; 300]), None);
        assert_eq!(percentis(&[]), None);
    }

    #[test]
    fn media_e_mediana_se_afastam_quando_o_jogo_engasga() {
        let liso = vec![16.0; 3000];
        let (media, _, _) = percentis(&liso).expect("amostra");
        let (mediana, _, _, _) = estatistica(liso);
        assert_eq!(media, mediana, "jogo liso: média e mediana iguais");

        let mut engasgado = vec![16.0; 2700];
        engasgado.extend(vec![300.0; 300]);
        let (media, _, _) = percentis(&engasgado).expect("amostra");
        let (mediana, _, _, _) = estatistica(engasgado);
        assert!(
            media > mediana * 2.0,
            "média {media} contra mediana {mediana}"
        );
    }

    #[test]
    fn engasgo_e_relativo_a_partida_e_nao_a_um_numero_fixo() {
        // Jogo a 30 FPS (33 ms), sem quadro fora do padrão.
        let (_, _, engasgos, _) = estatistica(vec![33.0; 3000]);
        assert_eq!(engasgos, 0.0, "jogo estável a 30 FPS não tem engasgo");

        let mut com_travada = vec![33.0; 2900];
        com_travada.extend(vec![200.0; 100]);
        let (_, _, engasgos, _) = estatistica(com_travada);
        assert!(
            engasgos > 0.0,
            "travada de 200 ms tem que contar como engasgo"
        );
    }

    #[test]
    fn amostra_pequena_nao_sustenta_o_detalhe() {
        let (_, _, _, confiavel) = estatistica(vec![16.7; 600]);
        assert!(!confiavel);

        let (_, _, _, confiavel) = estatistica(vec![16.7; 5000]);
        assert!(confiavel);
    }

    #[test]
    fn sem_amostra_nenhuma_devolve_zero_e_diz_que_nao_confia() {
        let (mediana, low, engasgos, confiavel) = estatistica(Vec::new());

        assert_eq!((mediana, low, engasgos), (0.0, 0.0, 0.0));
        assert!(!confiavel);
    }

    #[test]
    fn nao_ha_injecao_no_processo_do_jogo() {
        let fonte = include_str!("frames.rs");
        let producao = fonte.split("#[cfg(test)]").next().unwrap();

        for proibido in [
            "CreateRemoteThread",
            "WriteProcessMemory",
            "SetWindowsHookEx",
            "VirtualAllocEx",
        ] {
            assert!(
                !producao.contains(proibido),
                "`{}` apareceu no medidor de quadros",
                proibido
            );
        }
    }

    #[test]
    fn sem_quadro_contado_nao_devolve_zero() {
        // Zero FPS significa medição falha, não jogo parado.
        let fonte = include_str!("frames.rs");
        assert!(fonte.contains("Preferimos dizer que não medimos a mostrar zero quadros"));
    }

    #[test]
    fn processo_inexistente_nao_e_encontrado() {
        assert!(encontrar_processo("programa_que_nao_existe_12345").is_none());
    }

    #[test]
    fn encontra_um_processo_grafico_desta_maquina() {
        // O explorer desenha a área de trabalho em toda máquina.
        let achado = encontrar_processo("explorer");
        println!("explorer: {:?}", achado);
        assert!(achado.is_some(), "explorer.exe deveria estar rodando");
    }

    #[test]
    fn propriedades_carregam_o_nome_da_sessao() {
        let nome = para_utf16(NOME_SESSAO);
        let props = montar_propriedades(&nome);

        let tamanho_struct = std::mem::size_of::<EVENT_TRACE_PROPERTIES>();
        assert_eq!(props.len(), tamanho_struct + nome.len() * 2);

        unsafe {
            let p = props.as_ptr() as *const EVENT_TRACE_PROPERTIES;
            assert_eq!((*p).Wnode.BufferSize as usize, props.len());
            assert_eq!((*p).LoggerNameOffset as usize, tamanho_struct);
            assert_eq!((*p).LogFileMode, EVENT_TRACE_REAL_TIME_MODE);
        }

        // Sem o nome logo depois da estrutura, a sessão abre com nome vazio.
        let lido: Vec<u16> = unsafe {
            std::slice::from_raw_parts(
                props.as_ptr().add(tamanho_struct) as *const u16,
                nome.len() - 1,
            )
        }
        .to_vec();

        assert_eq!(String::from_utf16_lossy(&lido), NOME_SESSAO);
    }

    #[test]
    fn sem_elevacao_o_modulo_explica_em_vez_de_devolver_numero() {
        if super::super::registry::is_elevated() {
            println!("rodando elevado; caso nao exercitado");
            return;
        }

        let erro = medir(std::process::id(), "teste", 1).unwrap_err();
        assert!(erro.contains("administrador"), "erro inesperado: {}", erro);
    }

    /// Precisa de administrador e de algo desenhando na tela.
    #[test]
    #[ignore]
    fn mede_quadros_de_verdade() {
        let alvo = std::env::var("OTIMIZA_ALVO").unwrap_or_else(|_| "msedge".to_string());

        let (pid, nome) = encontrar_processo(&alvo).expect("processo alvo precisa estar aberto");
        println!("medindo {} (pid {})", nome, pid);

        match medir(pid, &nome, 5) {
            Ok(m) => {
                println!(
                    "{:.1} quadros por segundo ({} quadros em {:.1} s)",
                    m.fps, m.frames, m.seconds
                );

                assert!(m.fps > 0.0 && m.fps < 1000.0, "taxa implausivel: {}", m.fps);
            }
            Err(e) => println!("nao mediu: {}", e),
        }
    }
}
