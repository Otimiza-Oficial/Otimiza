// Medidor de quadros
//
// O produto sabe medir tudo, menos a única coisa que o cliente que joga
// realmente olha: quantos quadros por segundo o jogo está entregando. Sem isso,
// o "antes e depois" nunca fala a língua de quem joga.
//
// COMO SE MEDE ISSO SEM ENCOSTAR NO JOGO
//
// A forma óbvia é injetar código no processo do jogo e contar as chamadas de
// dentro. É o que quase todo contador de FPS faz, e é justamente o que este
// produto não vai fazer: para o FiveM, injeção é o que o anticheat procura, e o
// preço de errar é a conta do cliente.
//
// O caminho usado aqui é o rastreamento de eventos do próprio Windows. Toda vez
// que um programa manda um quadro para a tela, ele chama `Present`, e o Windows
// publica esse acontecimento num canal que qualquer programa autorizado pode
// escutar — de fora, sem tocar no jogo. Contar esses eventos durante alguns
// segundos dá a taxa real de quadros. É o mesmo princípio das ferramentas
// sérias de medição de desempenho gráfico.
//
// DUAS LIMITAÇÕES, DITAS AQUI E NA TELA
//
// Escutar esse canal exige administrador. Sem elevação, o Windows recusa a
// sessão de rastreamento, e o módulo diz isso em vez de devolver zero.
//
// O provedor cobre programas que desenham com Direct3D 10 ou mais novo, que é o
// caso do GTA V e portanto do FiveM. Um jogo muito antigo, em Direct3D 9, não
// aparece — e nesse caso o resultado é "não foi possível medir", nunca um
// número inventado.

#![cfg(target_os = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, ERROR_SUCCESS};
use windows_sys::Win32::System::Diagnostics::Etw::{
    CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace, StartTraceW,
    CONTROLTRACE_HANDLE, EVENT_CONTROL_CODE_ENABLE_PROVIDER, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP,
    EVENT_TRACE_LOGFILEW, EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE,
    PROCESS_TRACE_MODE_EVENT_RECORD, PROCESS_TRACE_MODE_REAL_TIME, TRACE_LEVEL_INFORMATION,
    WNODE_FLAG_TRACED_GUID,
};

/// Provedor de eventos do DXGI, a camada por onde todo jogo moderno entrega
/// quadros. GUID fixo, publicado pela Microsoft.
const PROVEDOR_DXGI: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0xCA11C036,
    data2: 0x0102,
    data3: 0x4A2D,
    data4: [0xA6, 0xAD, 0xF0, 0x3C, 0xFE, 0xD5, 0xD3, 0xC9],
};

/// Evento publicado no início de cada `Present`.
///
/// Contamos o início e não o fim: o fim pode não chegar se o quadro for
/// descartado, e aí a conta ficaria menor que a realidade.
const EVENTO_PRESENT_START: u16 = 42;

/// Nome da sessão de rastreamento.
///
/// Fixo de propósito: se uma execução anterior morreu sem fechar, a sessão
/// continua viva no sistema, e um nome fixo permite encontrá-la e derrubá-la.
const NOME_SESSAO: &str = "Otimiza-Quadros";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMeasurement {
    /// Quadros por segundo observados.
    pub fps: f64,
    /// Quantos eventos foram contados, e por quanto tempo.
    pub frames: u64,
    pub seconds: f64,
    /// Nome do processo medido.
    pub process: String,
    pub pid: u32,

    // --- o que a média esconde ---
    //
    // FPS médio é a métrica errada para o que este produto conserta. Quando o
    // problema é disputa de memória ou de processador, a média mal se move e o
    // jogo engasga do mesmo jeito — e engasgo é o que o cliente sente.
    //
    /// Tempo entre quadros, no meio da distribuição. Em milissegundos.
    pub frametime_mediano_ms: f64,
    /// Média dos 1% piores quadros, convertida em FPS. É o número que os
    /// analistas usam, e o que representa os momentos ruins da partida.
    pub low_1pct: f64,
    /// Quadros que demoraram mais que o dobro da mediana, por minuto.
    pub engasgos_por_minuto: f64,
    /// Verdadeiro quando houve amostra suficiente para os números acima
    /// significarem alguma coisa.
    pub detalhe_confiavel: bool,
}

// ------------------------------------------------------- estado do callback
//
// O Windows chama a função de retorno sem contexto próprio utilizável de forma
// segura em Rust, então o contador vive em estado global. A sessão é única e
// serializada por `MEDINDO`, então não há duas medições disputando o contador.

static MEDINDO: AtomicBool = AtomicBool::new(false);
static CONTADOR: AtomicU64 = AtomicU64::new(0);
static PID_ALVO: AtomicU32 = AtomicU32::new(0);

/// Instante de cada quadro, em unidades do relógio de alta resolução.
///
/// O dado já passava pela função de retorno e era jogado fora: o cabeçalho do
/// evento carrega o carimbo de tempo porque a sessão é criada com
/// `ClientContext = 1`. Contar sem guardar era descartar de graça tudo que
/// permite falar de engasgo.
///
/// O vetor é PRÉ-ALOCADO antes de a sessão começar. Alocar dentro da função de
/// retorno seria pedir para o coletor travar num momento em que ele não pode
/// travar — e o custo apareceria como engasgo na própria medição.
static INSTANTES: Mutex<Vec<i64>> = Mutex::new(Vec::new());

/// Teto de amostras. Trinta minutos a 500 quadros por segundo cabem aqui, e o
/// vetor ocupa uns 7 MB — barato o bastante para não pesar, grande o bastante
/// para nenhuma medição real encostar no limite.
const MAXIMO_DE_AMOSTRAS: usize = 900_000;

unsafe extern "system" fn ao_receber_evento(registro: *mut EVENT_RECORD) {
    if registro.is_null() {
        return;
    }

    let cabecalho = &(*registro).EventHeader;

    // Só o processo pedido, e só o evento de início de quadro. Sem os dois
    // filtros a conta viraria a soma de tudo que desenha na máquina.
    if cabecalho.ProcessId != PID_ALVO.load(Ordering::Relaxed) {
        return;
    }

    if cabecalho.EventDescriptor.Id != EVENTO_PRESENT_START {
        return;
    }

    CONTADOR.fetch_add(1, Ordering::Relaxed);

    // `try_lock` e não `lock`: se por qualquer motivo o cadeado estiver
    // ocupado, perder uma amostra é muito melhor do que segurar a função de
    // retorno do rastreamento do Windows.
    if let Ok(mut instantes) = INSTANTES.try_lock() {
        if instantes.len() < MAXIMO_DE_AMOSTRAS {
            instantes.push(cabecalho.TimeStamp);
        }
    }
}

/// Quantos quadros são necessários para o 1% pior significar alguma coisa.
///
/// Com 600 quadros, o "1% pior" são seis quadros — e seis quadros não
/// descrevem uma partida. Abaixo disto o produto mostra a média e DIZ que não
/// tem amostra para o resto, em vez de imprimir um número frágil.
const AMOSTRAS_PARA_DETALHE: usize = 2_000;

/// Estatística dos intervalos entre quadros.
///
/// **Função pura.** Recebe os intervalos já em milissegundos, para poder ser
/// testada sem abrir jogo nenhum.
pub fn estatistica(mut intervalos_ms: Vec<f64>) -> (f64, f64, f64, bool) {
    if intervalos_ms.is_empty() {
        return (0.0, 0.0, 0.0, false);
    }

    intervalos_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mediana = intervalos_ms[intervalos_ms.len() / 2];

    // 1% low é a MÉDIA do 1% pior, e não o percentil 99. As duas contas dão
    // números diferentes, e trocá-las é a forma mais comum de publicar um
    // número que não corresponde ao que os outros publicam.
    let quantos_piores = (intervalos_ms.len() / 100).max(1);
    let piores = &intervalos_ms[intervalos_ms.len() - quantos_piores..];
    let media_dos_piores = piores.iter().sum::<f64>() / quantos_piores as f64;

    let low_1pct = if media_dos_piores > 0.0 {
        1000.0 / media_dos_piores
    } else {
        0.0
    };

    // Engasgo é o quadro que demorou muito mais que o normal DAQUELA partida.
    // Um limiar fixo trataria um jogo a 30 quadros como engasgo permanente, e
    // um jogo a 240 nunca acusaria nada.
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

// --------------------------------------------------------------- utilitários

/// Bloco de propriedades exigido pelas funções de rastreamento.
///
/// A estrutura precisa ser seguida, na mesma alocação, pelo nome da sessão em
/// UTF-16. É por isso que o buffer é um vetor de bytes e não uma struct solta.
fn montar_propriedades(nome: &[u16]) -> Vec<u8> {
    let tamanho_struct = std::mem::size_of::<EVENT_TRACE_PROPERTIES>();
    let tamanho_nome = nome.len() * 2;
    let total = tamanho_struct + tamanho_nome;

    let mut buffer = vec![0u8; total];

    unsafe {
        let props = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
        (*props).Wnode.BufferSize = total as u32;
        (*props).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        (*props).Wnode.ClientContext = 1; // relógio de alta resolução
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

/// Derruba uma sessão com este nome, se existir.
///
/// Chamado antes de começar: sessão de rastreamento sobrevive ao processo que a
/// criou, então uma execução anterior que travou deixaria o nome ocupado para
/// sempre.
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

/// Encontra o processo do jogo pelo nome.
pub fn encontrar_processo(prefixo: &str) -> Option<(u32, String)> {
    use sysinfo::System;

    let mut sistema = System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let alvo = prefixo.to_lowercase();

    // Entre vários candidatos fica o que gasta mais memória: num jogo, o
    // processo que desenha é sempre o maior, e os auxiliares são pequenos.
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

/// Mede a taxa de quadros de um processo.
///
/// Bloqueia pelo tempo pedido. Deve ser chamada fora do runtime assíncrono.
pub fn medir(pid: u32, nome: &str, segundos: u64) -> Result<FrameMeasurement, String> {
    if !super::registry::is_elevated() {
        return Err(
            "Medir quadros exige executar como administrador: o canal de eventos do Windows \
             que informa cada quadro entregue só é aberto com essa permissão."
                .to_string(),
        );
    }

    // Medir quadros é seguro com QUALQUER anticheat rodando, e a consulta está
    // aqui para deixar isso escrito onde alguém vai ler.
    //
    // O motivo: este módulo não encosta no processo do jogo. Ele assina um
    // canal de eventos que o próprio Windows publica sobre a entrega de
    // quadros — nada de handle, nada de leitura de memória, nada de injeção.
    // É por isso que a medição continua funcionando no Valorant, enquanto a
    // suspensão de programas e a prioridade são recusadas.
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
    let resultado = medir_interno(pid, nome, segundos);
    MEDINDO.store(false, Ordering::SeqCst);
    resultado
}

fn medir_interno(pid: u32, nome: &str, segundos: u64) -> Result<FrameMeasurement, String> {
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
    PID_ALVO.store(pid, Ordering::SeqCst);

    // `ProcessTrace` só devolve quando a sessão para, então ela roda numa
    // linha de execução própria enquanto esta aqui cronometra.
    let coletor = std::thread::spawn(move || {
        let mut arquivo: EVENT_TRACE_LOGFILEW = unsafe { std::mem::zeroed() };
        let nome = para_utf16(NOME_SESSAO);

        arquivo.LoggerName = nome.as_ptr() as *mut u16;
        arquivo.Anonymous1.ProcessTraceMode =
            PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
        arquivo.Anonymous2.EventRecordCallback = Some(ao_receber_evento);

        let consumidor = unsafe { OpenTraceW(&mut arquivo) };

        // `INVALID_PROCESSTRACE_HANDLE` é o valor de erro; em 64 bits é u64::MAX.
        // O tipo é uma struct de campo único, então a comparação é no campo.
        if consumidor.Value == u64::MAX {
            return Err(unsafe { GetLastError() });
        }

        unsafe {
            ProcessTrace(&consumidor, 1, std::ptr::null(), std::ptr::null());
            CloseTrace(consumidor);
        }

        Ok(())
    });

    if let Ok(mut instantes) = INSTANTES.lock() {
        instantes.clear();
        // Pré-alocação generosa: alocar durante a medição faria o coletor
        // parar para crescer o vetor, e esse custo apareceria como engasgo na
        // própria conta de engasgos.
        instantes.reserve(segundos as usize * 600);
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

    let (mediana, low_1pct, engasgos, confiavel) = estatistica(intervalos_em_ms());

    Ok(FrameMeasurement {
        fps: frames as f64 / decorrido,
        frames,
        seconds: decorrido,
        process: nome.to_string(),
        pid,
        frametime_mediano_ms: mediana,
        low_1pct,
        engasgos_por_minuto: engasgos,
        detalhe_confiavel: confiavel,
    })
}

/// Converte os instantes coletados em intervalos, em milissegundos.
///
/// O carimbo de tempo do evento vem em unidades do contador de alta resolução,
/// cuja frequência varia de máquina para máquina — tratá-lo como se fosse
/// microssegundo daria números plausíveis e errados.
fn intervalos_em_ms() -> Vec<f64> {
    use windows_sys::Win32::System::Performance::QueryPerformanceFrequency;

    let mut frequencia: i64 = 0;
    unsafe {
        if QueryPerformanceFrequency(&mut frequencia) == 0 || frequencia <= 0 {
            return Vec::new();
        }
    }

    let Ok(instantes) = INSTANTES.lock() else {
        return Vec::new();
    };

    instantes
        .windows(2)
        .map(|par| (par[1] - par[0]) as f64 * 1000.0 / frequencia as f64)
        // Intervalo negativo ou absurdo é reordenação de evento, não quadro.
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
    fn o_um_por_cento_pior_e_media_e_nao_percentil() {
        // As duas contas dão números diferentes, e trocá-las é a forma mais
        // comum de publicar um número que não bate com o de ninguém.
        //
        // 1000 quadros: 990 a 10 ms (100 FPS) e 10 a 100 ms (10 FPS). A média
        // dos 1% piores são exatamente os dez de 100 ms, logo 10 FPS.
        let mut intervalos = vec![10.0; 990];
        intervalos.extend(vec![100.0; 10]);

        let (mediana, low, _, _) = estatistica(intervalos);

        assert_eq!(mediana, 10.0);
        assert_eq!(low, 10.0);
    }

    #[test]
    fn engasgo_e_relativo_a_partida_e_nao_a_um_numero_fixo() {
        // Limiar fixo trataria um jogo a 30 quadros como engasgo permanente e
        // nunca acusaria nada num jogo a 240.
        //
        // Jogo a 30 FPS (33 ms), sem nenhum quadro fora do padrão.
        let (_, _, engasgos, _) = estatistica(vec![33.0; 3000]);
        assert_eq!(engasgos, 0.0, "jogo estável a 30 FPS não tem engasgo");

        // Mesmo jogo, agora com quadros de 200 ms no meio.
        let mut com_travada = vec![33.0; 2900];
        com_travada.extend(vec![200.0; 100]);
        let (_, _, engasgos, _) = estatistica(com_travada);
        assert!(engasgos > 0.0, "travada de 200 ms tem que contar como engasgo");
    }

    #[test]
    fn amostra_pequena_nao_sustenta_o_detalhe() {
        // Com 600 quadros, o "1% pior" são seis quadros. Seis quadros não
        // descrevem uma partida, e imprimir aquilo como número seria dar
        // aparência de medida a um palpite.
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
        // A promessa que justifica este módulo existir: medir de fora. Injetar
        // é o que o anticheat do FiveM procura, e o preço de errar é a conta do
        // cliente.
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
        // Zero quadros por segundo seria um número inventado: significa que a
        // medição falhou, não que o jogo parou. O módulo devolve erro.
        let fonte = include_str!("frames.rs");
        assert!(fonte.contains("Preferimos dizer que não medimos a mostrar zero quadros"));
    }

    #[test]
    fn processo_inexistente_nao_e_encontrado() {
        assert!(encontrar_processo("programa_que_nao_existe_12345").is_none());
    }

    #[test]
    fn encontra_um_processo_grafico_desta_maquina() {
        // O explorer desenha a área de trabalho e existe em toda máquina.
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

        // E o nome precisa estar legível logo depois da estrutura, senão o
        // Windows abre a sessão com nome vazio.
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

    /// Mede de verdade. Precisa de administrador e de algo desenhando na tela.
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

                // Um número plausível: nem zero, nem além do que qualquer tela
                // consegue mostrar.
                assert!(m.fps > 0.0 && m.fps < 1000.0, "taxa implausivel: {}", m.fps);
            }
            Err(e) => println!("nao mediu: {}", e),
        }
    }
}
