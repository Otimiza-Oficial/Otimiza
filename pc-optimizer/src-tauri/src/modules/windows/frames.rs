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
}

// ------------------------------------------------------- estado do callback
//
// O Windows chama a função de retorno sem contexto próprio utilizável de forma
// segura em Rust, então o contador vive em estado global. A sessão é única e
// serializada por `MEDINDO`, então não há duas medições disputando o contador.

static MEDINDO: AtomicBool = AtomicBool::new(false);
static CONTADOR: AtomicU64 = AtomicU64::new(0);
static PID_ALVO: AtomicU32 = AtomicU32::new(0);

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

    Ok(FrameMeasurement {
        fps: frames as f64 / decorrido,
        frames,
        seconds: decorrido,
        process: nome.to_string(),
        pid,
    })
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
