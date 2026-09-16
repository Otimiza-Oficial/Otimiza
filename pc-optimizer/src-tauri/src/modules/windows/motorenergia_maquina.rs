// ---------------------------------------------------------------------------
// ADAPTIVE POWER ENGINE — a parte que conversa com o Windows
//
// As regras moram em `motorenergia.rs`, com testes. Aqui ficam só as leituras
// e escritas: CPUID, topologia, `powercfg /qh`, contadores de desempenho pelo
// PDH, a rajada de carga, o backup exato do plano anterior e as duas voltas
// ("plano anterior" e "padrão do Windows").
//
// TRÊS REGRAS DESTE ARQUIVO:
//
// 1. O plano do cliente nunca é escrito. O motor trabalha dentro do plano
//    OTIMIZA, copiado do Equilibrado. Voltar é reativar o plano anterior, que
//    ficou intacto — e o backup existe para PROVAR isso, relendo cada valor.
// 2. Toda escrita é relida do Windows. Código de saída não é prova.
// 3. Nenhum núcleo é desligado, nenhuma afinidade é fixada, nenhum processo
//    vai para tempo real. A rajada usa uma thread comum do próprio Otimiza.
// ---------------------------------------------------------------------------

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::motorenergia::{self as motor, AmostraCpu, Candidato, Cpuid, Enumeracao, Impressao, Parametros, Topologia};
use super::planoenergia::{self as plano, EQUILIBRADO_GUID, NOME_DO_PLANO};
use super::{power, registry, shell};

// ================================================================ CPUID

#[cfg(target_arch = "x86_64")]
#[allow(unused_unsafe)]
pub fn ler_cpuid() -> Cpuid {
    use std::arch::x86_64::{__cpuid, __cpuid_count};

    unsafe {
        let zero = __cpuid(0);
        let mut bytes = Vec::with_capacity(12);
        for registro in [zero.ebx, zero.edx, zero.ecx] {
            bytes.extend_from_slice(&registro.to_le_bytes());
        }
        let fabricante_bruto = String::from_utf8_lossy(&bytes).to_string();
        let maximo = zero.eax;

        let um = __cpuid(1);
        let (familia, modelo) = motor::familia_e_modelo(um.eax);

        let (hwp, hwp_epp, preferidos) = if maximo >= 6 {
            let seis = __cpuid(6);
            (seis.eax & (1 << 7) != 0, seis.eax & (1 << 10) != 0, seis.eax & (1 << 14) != 0)
        } else {
            (false, false, false)
        };

        let hibrido = maximo >= 7 && __cpuid_count(7, 0).edx & (1 << 15) != 0;

        let maximo_estendido = __cpuid(0x8000_0000).eax;
        let cppc = maximo_estendido >= 0x8000_0008 && __cpuid(0x8000_0008).ebx & (1 << 27) != 0;

        Cpuid { fabricante_bruto, familia, modelo, hibrido, hwp, hwp_epp, nucleos_preferidos_intel: preferidos, cppc }
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn ler_cpuid() -> Cpuid {
    Cpuid::default()
}

// ============================================================ topologia

/// Núcleos físicos, processadores lógicos e classes de eficiência, lidos do
/// Windows. A estrutura é percorrida pelos deslocamentos documentados em
/// `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX` / `PROCESSOR_RELATIONSHIP`.
pub fn ler_topologia() -> Topologia {
    use windows_sys::Win32::System::SystemInformation::{GetLogicalProcessorInformationEx, RelationProcessorCore};

    let mut tamanho: u32 = 0;
    unsafe {
        GetLogicalProcessorInformationEx(RelationProcessorCore, std::ptr::null_mut(), &mut tamanho);
    }
    if tamanho == 0 {
        return Topologia::default();
    }

    let mut buffer = vec![0u8; tamanho as usize];
    let ok = unsafe {
        GetLogicalProcessorInformationEx(RelationProcessorCore, buffer.as_mut_ptr() as *mut _, &mut tamanho)
    };
    if ok == 0 {
        return Topologia::default();
    }

    let mut t = Topologia::default();
    let mut classes: std::collections::BTreeMap<u8, u32> = Default::default();
    let mut pos = 0usize;

    while pos + 32 <= tamanho as usize {
        let tamanho_item = u32::from_le_bytes(buffer[pos + 4..pos + 8].try_into().unwrap()) as usize;
        if tamanho_item == 0 {
            break;
        }
        let classe = buffer[pos + 9];
        let grupos = u16::from_le_bytes(buffer[pos + 30..pos + 32].try_into().unwrap()) as usize;
        t.nucleos_fisicos += 1;
        *classes.entry(classe).or_default() += 1;
        for g in 0..grupos {
            let inicio = pos + 32 + g * 16;
            if inicio + 8 <= buffer.len() {
                let mascara = u64::from_le_bytes(buffer[inicio..inicio + 8].try_into().unwrap());
                t.processadores_logicos += mascara.count_ones();
            }
        }
        pos += tamanho_item;
    }

    t.classes = classes.into_iter().collect();
    t
}

// ============================================================ impressão

static IMPRESSAO: OnceLock<Impressao> = OnceLock::new();

fn nome_do_plano(guid: &str) -> Option<String> {
    plano::listar_planos()
        .ok()?
        .into_iter()
        .find(|(g, _)| g.eq_ignore_ascii_case(guid))
        .map(|(_, n)| n)
}

/// HARDWARE FINGERPRINT. A parte cara (PowerShell do chassi) é lida uma vez
/// por sessão; alimentação e plano ativo são relidos toda vez.
pub fn impressao() -> Impressao {
    let fixa = IMPRESSAO
        .get_or_init(|| {
            let cpuid = ler_cpuid();
            let arquitetura = motor::classificar_arquitetura(&cpuid);
            let maquina = plano::detectar();
            Impressao {
                cpu: maquina.cpu.clone(),
                fabricante: motor::fabricante_do_cpuid(&cpuid.fabricante_bruto),
                arquitetura,
                controle: motor::controle(&cpuid, arquitetura),
                cpuid,
                topologia: ler_topologia(),
                formato: if maquina.notebook { motor::Formato::Notebook } else { motor::Formato::Desktop },
                na_tomada: None,
                build_do_windows: maquina.build_do_windows,
                modern_standby: maquina.modern_standby,
                plano_ativo: None,
                plano_ativo_nome: None,
            }
        })
        .clone();

    let ativo = power::active_scheme().ok();
    Impressao {
        na_tomada: match plano::alimentacao() {
            plano::Alimentacao::Tomada => Some(true),
            plano::Alimentacao::Bateria => Some(false),
            plano::Alimentacao::NaoSei => None,
        },
        plano_ativo_nome: ativo.as_deref().and_then(nome_do_plano),
        plano_ativo: ativo,
        ..fixa
    }
}

// =========================================================== enumeração

pub fn enumerar_plano(guid: &str) -> Result<Enumeracao, String> {
    if !plano::e_guid(guid) {
        return Err(format!("`{}` não é um GUID de plano de energia.", guid));
    }
    let saida = shell::run_checked("powercfg", &["/qh", guid])?;
    let e = motor::enumerar(&saida);
    if e.configuracoes.is_empty() {
        return Err("O `powercfg /qh` não devolveu nenhum ajuste que desse para ler.".to_string());
    }
    Ok(e)
}

/// O plano base de todo candidato: o Equilibrado deste Windows.
pub fn enumerar_base() -> Result<Enumeracao, String> {
    enumerar_plano(EQUILIBRADO_GUID)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bateria {
    pub autoajuste: Vec<Candidato>,
    pub escada_de_epp: Vec<Candidato>,
    pub dispositivos: Vec<Candidato>,
    pub controlados: Vec<String>,
}

pub fn bateria_de_candidatos(i: &Impressao, base: &Enumeracao) -> Bateria {
    let autoajuste = motor::candidatos_de_autoajuste(i, base);
    let sobre = autoajuste.get(2).map(|c| c.parametros).unwrap_or(Parametros::WINDOWS);
    let escada_de_epp = motor::escada_de_epp(i, base);
    let dispositivos = motor::variacoes_de_dispositivo(i, base, sobre);
    let todos: Vec<Candidato> = autoajuste.iter().chain(&escada_de_epp).chain(&dispositivos).cloned().collect();
    let mut controlados = motor::apelidos_controlados(&todos);
    // O que o plano OTIMIZA antigo escrevia em todo PC também volta ao base.
    for antigo in [motor::apelidos::PROCTHROTTLEMIN, motor::apelidos::CPMINCORES, motor::apelidos::PERFBOOSTMODE, motor::apelidos::PERFEPP] {
        if base.por_alias(antigo).is_some() && !controlados.iter().any(|c| c == antigo) {
            controlados.push(antigo.to_string());
        }
    }
    Bateria { autoajuste, escada_de_epp, dispositivos, controlados }
}

// ============================================================== backup

fn pasta() -> PathBuf {
    let base = std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
    base.join("pc-optimizer").join("energia")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValorGuardado {
    pub plano: String,
    pub subgrupo: String,
    pub ajuste: String,
    pub alias: Option<String>,
    pub ac: Option<u32>,
    pub dc: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    pub plano_anterior: String,
    pub nome_anterior: Option<String>,
    pub capturado_em: u64,
    pub valores: Vec<ValorGuardado>,
}

pub fn ler_backup() -> Option<Backup> {
    std::fs::read_to_string(pasta().join("backup.json")).ok().and_then(|t| serde_json::from_str(&t).ok())
}

/// Guarda o plano ativo e TODOS os valores dele antes da primeira mudança.
///
/// Se o ativo já é o OTIMIZA, o backup existente continua valendo: o plano
/// "anterior" é o de antes do Otimiza, não o próprio Otimiza.
fn garantir_backup(nosso: Option<&str>) -> Result<Backup, String> {
    let ativo = power::active_scheme()?;
    if nosso.is_some_and(|n| n.eq_ignore_ascii_case(&ativo)) {
        if let Some(b) = ler_backup() {
            return Ok(b);
        }
    }

    let e = enumerar_plano(&ativo)?;
    let backup = Backup {
        plano_anterior: ativo.clone(),
        nome_anterior: nome_do_plano(&ativo),
        capturado_em: crate::modules::changelog::now_timestamp(),
        valores: e
            .configuracoes
            .iter()
            .map(|c| ValorGuardado {
                plano: ativo.clone(),
                subgrupo: c.subgrupo.clone(),
                ajuste: c.guid.clone(),
                alias: c.alias.clone(),
                ac: c.ac,
                dc: c.dc,
            })
            .collect(),
    };

    std::fs::create_dir_all(pasta()).map_err(|e| format!("Não foi possível criar a pasta do backup: {}", e))?;
    let texto = serde_json::to_string_pretty(&backup).map_err(|e| e.to_string())?;
    std::fs::write(pasta().join("backup.json"), texto).map_err(|e| format!("Não foi possível gravar o backup: {}", e))?;
    Ok(backup)
}

// ============================================================ escrita

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aplicacao {
    pub plano: String,
    pub candidato: String,
    pub escritos: usize,
    /// Apelidos que o Windows aceitou e, relidos, não ficaram no valor.
    pub divergentes: Vec<String>,
    pub recusados: Vec<String>,
    pub ativo: bool,
}

fn exigir_admin() -> Result<(), String> {
    if registry::is_elevated() {
        Ok(())
    } else {
        Err("Mudar o plano de energia exige abrir o Otimiza como administrador.".to_string())
    }
}

fn plano_otimiza() -> Result<Option<String>, String> {
    Ok(plano::achar_na_lista(&plano::listar_planos()?, NOME_DO_PLANO))
}

/// Escreve um candidato no plano OTIMIZA, relê, ativa e confere.
pub fn aplicar(candidato: &Candidato, controlados: &[String]) -> Result<Aplicacao, String> {
    exigir_admin()?;

    let existente = plano_otimiza()?;
    garantir_backup(existente.as_deref())?;

    let guid = match existente {
        Some(g) => g,
        None => plano::criar_plano(EQUILIBRADO_GUID)?,
    };

    let base = enumerar_base()?;
    let atual = enumerar_plano(&guid)?;
    let escrita = motor::plano_de_escrita(candidato, controlados, &base, &atual);

    let mut recusados = Vec::new();
    for m in &escrita {
        for (lado, valor) in [("-setacvalueindex", m.ac), ("-setdcvalueindex", m.dc)] {
            let Some(v) = valor else { continue };
            let v = v.to_string();
            let args = [lado, guid.as_str(), m.subgrupo.as_str(), m.guid.as_str(), v.as_str()];
            match shell::run("powercfg", &args) {
                Ok(s) if s.success => {}
                Ok(s) => {
                    crate::utils::Logger::warn(&format!("motor de energia: recusado `powercfg {}`: {}", args.join(" "), s.stderr.trim()));
                    recusados.push(m.alias.clone());
                }
                Err(e) => {
                    crate::utils::Logger::warn(&format!("motor de energia: `powercfg {}` falhou: {}", args.join(" "), e));
                    recusados.push(m.alias.clone());
                }
            }
        }
    }
    recusados.dedup();

    // Reativar é o que faz o Windows carregar os valores novos do plano ativo.
    power::set_active_scheme(&guid)?;
    let ativo = power::active_scheme().map(|a| a.eq_ignore_ascii_case(&guid)).unwrap_or(false);

    let relido = enumerar_plano(&guid)?;
    let divergentes: Vec<String> = motor::divergencias(&escrita, &relido)
        .into_iter()
        .filter(|a| !recusados.contains(a))
        .collect();

    crate::utils::Logger::info(&format!(
        "motor de energia: candidato {} em {} — {} escritos, {} divergentes, {} recusados, ativo={}",
        candidato.id,
        guid,
        escrita.len(),
        divergentes.len(),
        recusados.len(),
        ativo
    ));

    Ok(Aplicacao { plano: guid, candidato: candidato.id.clone(), escritos: escrita.len(), divergentes, recusados, ativo })
}

// ============================================================== voltas

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Restauracao {
    pub plano_ativo: String,
    /// Valores do plano restaurado que não batem com o backup. Vazio é a
    /// prova de que ele voltou exatamente como estava.
    pub divergentes: Vec<String>,
    pub plano_otimiza_apagado: bool,
}

fn apagar_plano_otimiza() -> bool {
    match plano_otimiza() {
        Ok(Some(g)) => shell::run_checked("powercfg", &["-delete", &g]).is_ok(),
        _ => false,
    }
}

/// RESTORE PREVIOUS PLAN.
pub fn restaurar_anterior() -> Result<Restauracao, String> {
    exigir_admin()?;
    let backup = ler_backup().ok_or("Não há backup do plano anterior: o motor nunca mudou o plano nesta máquina.")?;

    power::set_active_scheme(&backup.plano_anterior)
        .map_err(|e| format!("O plano anterior não pôde ser ativado ({}). Use \"Restaurar padrão do Windows\".", e))?;
    let ativo = power::active_scheme()?;
    if !ativo.eq_ignore_ascii_case(&backup.plano_anterior) {
        return Err(format!("O Windows aceitou, mas o plano ativo continua {}.", ativo));
    }

    let relido = enumerar_plano(&ativo)?;
    let divergentes = backup
        .valores
        .iter()
        .filter(|v| {
            let agora = relido.configuracoes.iter().find(|c| c.guid == v.ajuste && c.subgrupo == v.subgrupo);
            agora.map(|c| (c.ac, c.dc)) != Some((v.ac, v.dc))
        })
        .map(|v| v.alias.clone().unwrap_or_else(|| v.ajuste.clone()))
        .collect();

    let apagado = apagar_plano_otimiza();
    let _ = std::fs::remove_file(pasta().join("backup.json"));
    Ok(Restauracao { plano_ativo: ativo, divergentes, plano_otimiza_apagado: apagado })
}

/// RESTORE WINDOWS DEFAULT: ativa o Equilibrado e apaga o plano OTIMIZA.
pub fn restaurar_padrao_windows() -> Result<Restauracao, String> {
    exigir_admin()?;
    power::set_active_scheme(EQUILIBRADO_GUID)?;
    let ativo = power::active_scheme()?;
    if !ativo.eq_ignore_ascii_case(EQUILIBRADO_GUID) {
        return Err(format!("O Windows aceitou, mas o plano ativo continua {}.", ativo));
    }
    let apagado = apagar_plano_otimiza();
    Ok(Restauracao { plano_ativo: ativo, divergentes: Vec::new(), plano_otimiza_apagado: apagado })
}

// ======================================================== contadores (PDH)

const PDH_OK: u32 = 0;

fn largo(texto: &str) -> Vec<u16> {
    texto.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Contadores de desempenho pelo NOME EM INGLÊS (`PdhAddEnglishCounterW`),
/// que é o mesmo em qualquer idioma do Windows.
pub struct Amostrador {
    consulta: isize,
    desempenho: isize,
    frequencia: isize,
    uso: isize,
    limite: isize,
    flags: isize,
    estacionamento: isize,
    temperatura: isize,
}

unsafe impl Send for Amostrador {}

impl Amostrador {
    pub fn novo() -> Option<Self> {
        use windows_sys::Win32::System::Performance::{PdhAddEnglishCounterW, PdhCollectQueryData, PdhOpenQueryW};

        let mut consulta: isize = 0;
        if unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut consulta) } != PDH_OK {
            return None;
        }
        let adicionar = |caminho: &str| -> isize {
            let mut h: isize = 0;
            let c = largo(caminho);
            if unsafe { PdhAddEnglishCounterW(consulta, c.as_ptr(), 0, &mut h) } == PDH_OK { h } else { 0 }
        };
        let a = Amostrador {
            consulta,
            desempenho: adicionar(r"\Processor Information(_Total)\% Processor Performance"),
            frequencia: adicionar(r"\Processor Information(_Total)\Processor Frequency"),
            uso: adicionar(r"\Processor Information(_Total)\% Processor Time"),
            limite: adicionar(r"\Processor Information(_Total)\% Performance Limit"),
            flags: adicionar(r"\Processor Information(_Total)\Performance Limit Flags"),
            estacionamento: adicionar(r"\Processor Information(*)\Parking Status"),
            temperatura: adicionar(r"\Thermal Zone Information(*)\Temperature"),
        };
        unsafe { PdhCollectQueryData(consulta) };
        Some(a)
    }

    fn valor(&self, contador: isize) -> Option<f64> {
        use windows_sys::Win32::System::Performance::{PdhGetFormattedCounterValue, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE};
        if contador == 0 {
            return None;
        }
        let mut v: PDH_FMT_COUNTERVALUE = unsafe { std::mem::zeroed() };
        let r = unsafe { PdhGetFormattedCounterValue(contador, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut v) };
        (r == PDH_OK).then(|| unsafe { v.Anonymous.doubleValue })
    }

    fn lista(&self, contador: isize) -> Vec<(String, f64)> {
        use windows_sys::Win32::System::Performance::{PdhGetFormattedCounterArrayW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_MORE_DATA};
        if contador == 0 {
            return Vec::new();
        }
        let mut tamanho: u32 = 0;
        let mut itens: u32 = 0;
        let r = unsafe { PdhGetFormattedCounterArrayW(contador, PDH_FMT_DOUBLE, &mut tamanho, &mut itens, std::ptr::null_mut()) };
        if r != PDH_MORE_DATA || tamanho == 0 {
            return Vec::new();
        }
        let item = std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let mut buffer = vec![0u8; tamanho as usize + item];
        let r = unsafe {
            PdhGetFormattedCounterArrayW(contador, PDH_FMT_DOUBLE, &mut tamanho, &mut itens, buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W)
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

    pub fn coletar(&self) -> AmostraCpu {
        use windows_sys::Win32::System::Performance::PdhCollectQueryData;
        unsafe { PdhCollectQueryData(self.consulta) };

        let nucleos: Vec<(String, f64)> = self.lista(self.estacionamento).into_iter().filter(|(n, _)| !n.contains("_Total")).collect();
        let temperaturas: Vec<f64> = self
            .lista(self.temperatura)
            .into_iter()
            .map(|(_, k)| k - 273.15)
            // Zona que devolve 0 K ou valor absurdo não é sensor de verdade.
            .filter(|c| (5.0..=120.0).contains(c))
            .collect();

        AmostraCpu {
            desempenho_pct: self.valor(self.desempenho).unwrap_or(0.0),
            frequencia_mhz: self.valor(self.frequencia).unwrap_or(0.0),
            uso_pct: self.valor(self.uso).unwrap_or(0.0),
            limite_pct: self.valor(self.limite).unwrap_or(100.0),
            flags: self.valor(self.flags).map(|f| f as u64).unwrap_or(0),
            nucleos_acordados: (!nucleos.is_empty()).then(|| nucleos.iter().filter(|(_, v)| *v == 0.0).count() as u32),
            nucleos_total: (!nucleos.is_empty()).then_some(nucleos.len() as u32),
            temperatura_c: temperaturas.into_iter().reduce(f64::max),
        }
    }
}

impl Drop for Amostrador {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Performance::PdhCloseQuery;
        unsafe { PdhCloseQuery(self.consulta) };
    }
}

/// Amostra os contadores a cada `intervalo` até `parar` virar verdadeiro.
pub fn amostrar_enquanto(parar: std::sync::Arc<std::sync::atomic::AtomicBool>, intervalo: Duration) -> Vec<AmostraCpu> {
    use std::sync::atomic::Ordering;
    let Some(a) = Amostrador::novo() else { return Vec::new() };
    let mut v = Vec::new();
    while !parar.load(Ordering::Relaxed) {
        std::thread::sleep(intervalo);
        v.push(a.coletar());
    }
    v
}

/// Uma leitura instantânea, para o mapa de resposta.
pub fn amostra_agora() -> Option<AmostraCpu> {
    let a = Amostrador::novo()?;
    std::thread::sleep(Duration::from_millis(500));
    Some(a.coletar())
}

// ============================================================== a rajada

const FATIA: Duration = Duration::from_millis(2);
const FATIAS_POR_RAJADA: usize = 200;
const RAJADAS: usize = 5;
const OCIOSO: Duration = Duration::from_millis(1500);

#[inline(never)]
fn bloco_de_trabalho(estado: &mut u64) {
    for _ in 0..2_000 {
        *estado ^= *estado << 13;
        *estado ^= *estado >> 7;
        *estado ^= *estado << 17;
        *estado = estado.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// POWER RESPONSE TEST: ocioso → rajada curta → ocioso → rajada…
///
/// Cada fatia de 5 ms conta quantos blocos de trabalho fixos couberam nela.
/// A CPU que demora a subir o desempenho faz menos blocos nas primeiras
/// fatias. Uma thread comum, sem afinidade e sem prioridade alterada: é o
/// mesmo caminho que a thread de um jogo percorre.
pub fn teste_de_rajada() -> (Option<motor::RespostaMedida>, Vec<Vec<f64>>) {
    let rajadas: Vec<Vec<f64>> = std::thread::spawn(|| {
        let mut estado: u64 = 0x2545_F491_4F6C_DD1D;
        let mut todas = Vec::with_capacity(RAJADAS);
        for _ in 0..RAJADAS {
            std::thread::sleep(OCIOSO);
            let mut fatias = Vec::with_capacity(FATIAS_POR_RAJADA);
            for _ in 0..FATIAS_POR_RAJADA {
                let inicio = Instant::now();
                let mut blocos = 0u64;
                while inicio.elapsed() < FATIA {
                    bloco_de_trabalho(&mut estado);
                    blocos += 1;
                }
                fatias.push(blocos as f64 * FATIA.as_secs_f64() / inicio.elapsed().as_secs_f64());
            }
            todas.push(fatias);
        }
        todas
    })
    .join()
    .unwrap_or_default();

    (motor::analisar_rajadas(&rajadas, FATIA.as_millis() as f64), rajadas)
}

// ================================================================ medição

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicaoDoCandidato {
    pub resultado: motor::ResultadoDoCandidato,
    /// A curva média da rajada, em % do sustentado, para o mapa de resposta.
    pub curva_pct: Vec<f64>,
    pub aplicacao: Option<Aplicacao>,
}

fn curva_media(rajadas: &[Vec<f64>]) -> Vec<f64> {
    let Some(n) = rajadas.iter().map(|r| r.len()).min() else { return Vec::new() };
    if n == 0 {
        return Vec::new();
    }
    let media: Vec<f64> = (0..n).map(|i| rajadas.iter().map(|r| r[i]).sum::<f64>() / rajadas.len() as f64).collect();
    let mut cauda = media[n * 7 / 10..].to_vec();
    cauda.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sustentado = cauda[cauda.len() / 2];
    if sustentado <= 0.0 {
        return Vec::new();
    }
    media.iter().map(|v| ((v / sustentado * 100.0) * 10.0).round() / 10.0).collect()
}

/// Mede o que está ativo agora: rajada (com contadores) e, se houver jogo,
/// `repeticoes` medições de quadros com contadores e uso de GPU.
pub fn medir_atual(id: &str, processo: Option<&str>, segundos: u64, repeticoes: u32) -> Result<MedicaoDoCandidato, String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Dá ao Windows alguns segundos para o plano novo valer de fato.
    std::thread::sleep(Duration::from_secs(3));

    let parar = Arc::new(AtomicBool::new(false));
    let p2 = parar.clone();
    let amostrador = std::thread::spawn(move || amostrar_enquanto(p2, Duration::from_millis(250)));
    let (resposta, rajadas) = teste_de_rajada();
    parar.store(true, Ordering::Relaxed);
    let mut amostras = amostrador.join().unwrap_or_default();

    let mut quadros = None;
    let mut fps_repeticoes = Vec::new();
    let mut gpu = None;
    let mut uso = None;

    if let Some(nome) = processo.filter(|p| !p.trim().is_empty()) {
        let (pid, executavel) = super::frames::encontrar_processo(nome)
            .ok_or_else(|| format!("Não encontrei `{}` aberto. Abra o jogo para medir com ele.", nome))?;

        amostras.clear();
        let mut intervalos_todos = Vec::new();
        let mut fps_soma = 0.0;
        for _ in 0..repeticoes.clamp(1, 5) {
            let parar = Arc::new(AtomicBool::new(false));
            let p2 = parar.clone();
            let amostrador = std::thread::spawn(move || amostrar_enquanto(p2, Duration::from_millis(500)));
            let gargalo = std::thread::spawn(move || super::bottleneck::analisar(segundos));
            let medido = super::frames::medir_par(pid, &executavel, None, segundos);
            parar.store(true, Ordering::Relaxed);
            amostras.extend(amostrador.join().unwrap_or_default());
            if let Ok(g) = gargalo.join() {
                gpu = Some(g.gpu_percent);
                uso = Some(g.cpu_total);
            }
            let (m, _) = medido?;
            fps_repeticoes.push((m.resumo.fps * 10.0).round() / 10.0);
            fps_soma += m.resumo.fps;
            intervalos_todos.extend(m.intervalos_ms);
        }
        quadros = motor::resumir_quadros(fps_soma / fps_repeticoes.len() as f64, &intervalos_todos);
    }

    Ok(MedicaoDoCandidato {
        resultado: motor::ResultadoDoCandidato {
            candidato: id.to_string(),
            resposta,
            cpu: motor::resumir_cpu(&amostras),
            quadros,
            fps_repeticoes,
            gpu_pct: gpu,
            uso_cpu_pct: uso,
        },
        curva_pct: curva_media(&rajadas),
        aplicacao: None,
    })
}

// =========================================================== perfis de jogo

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfilDeJogo {
    pub executavel: String,
    pub parametros: Parametros,
    pub escolha: Option<motor::Escolha>,
    pub quando: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dinamico {
    pub ligado: bool,
}

fn ler_json<T: for<'a> Deserialize<'a> + Default>(nome: &str) -> T {
    std::fs::read_to_string(pasta().join(nome)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn gravar_json<T: Serialize>(nome: &str, valor: &T) -> Result<(), String> {
    std::fs::create_dir_all(pasta()).map_err(|e| e.to_string())?;
    let texto = serde_json::to_string_pretty(valor).map_err(|e| e.to_string())?;
    std::fs::write(pasta().join(nome), texto).map_err(|e| format!("Não foi possível gravar {}: {}", nome, e))
}

pub fn perfis_de_jogo() -> Vec<PerfilDeJogo> {
    ler_json("jogos.json")
}

pub fn salvar_perfil_de_jogo(perfil: PerfilDeJogo) -> Result<Vec<PerfilDeJogo>, String> {
    let mut todos = perfis_de_jogo();
    todos.retain(|p| !p.executavel.eq_ignore_ascii_case(&perfil.executavel));
    todos.push(perfil);
    gravar_json("jogos.json", &todos)?;
    Ok(todos)
}

pub fn remover_perfil_de_jogo(executavel: &str) -> Result<Vec<PerfilDeJogo>, String> {
    let mut todos = perfis_de_jogo();
    todos.retain(|p| !p.executavel.eq_ignore_ascii_case(executavel));
    gravar_json("jogos.json", &todos)?;
    Ok(todos)
}

pub fn dinamico() -> Dinamico {
    ler_json("dinamico.json")
}

pub fn definir_dinamico(ligado: bool) -> Result<Dinamico, String> {
    let d = Dinamico { ligado };
    gravar_json("dinamico.json", &d)?;
    Ok(d)
}

/// Aplica parâmetros (de um perfil salvo ou do vencedor) nesta máquina.
pub fn aplicar_parametros(id: &str, parametros: Parametros) -> Result<Aplicacao, String> {
    let i = impressao();
    let base = enumerar_base()?;
    let bateria = bateria_de_candidatos(&i, &base);
    let candidato = motor::gerar(&i, &base, id, motor::Papel::B, parametros);
    let mut controlados = bateria.controlados;
    for m in &candidato.mudancas {
        if !controlados.contains(&m.alias) {
            controlados.push(m.alias.clone());
        }
    }
    aplicar(&candidato, &controlados)
}

/// Estado do vigia do modo dinâmico, guardado entre olhadas.
#[derive(Debug, Default)]
pub struct Vigia {
    pub estado: Option<motor::EstadoDinamico>,
    pub olhadas_sem_jogo: u32,
    pub jogo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventoDinamico {
    pub transicao: motor::Transicao,
    pub jogo: Option<String>,
    pub erro: Option<String>,
}

/// Uma olhada do modo dinâmico. Procura só os executáveis com perfil salvo —
/// uma listagem de processos, sem PowerShell — e troca o plano quando precisa.
pub fn olhar(vigia: &mut Vigia) -> Option<EventoDinamico> {
    if !dinamico().ligado || !registry::is_elevated() {
        return None;
    }
    let perfis = perfis_de_jogo();
    if perfis.is_empty() {
        return None;
    }

    let aberto = perfis.iter().find(|p| super::frames::encontrar_processo(p.executavel.trim_end_matches(".exe")).is_some());
    let estado = vigia.estado.unwrap_or(motor::EstadoDinamico::Normal);
    vigia.olhadas_sem_jogo = if aberto.is_some() { 0 } else { vigia.olhadas_sem_jogo + 1 };

    match motor::transicao(estado, aberto.is_some(), vigia.olhadas_sem_jogo) {
        motor::Transicao::Nenhuma => None,
        motor::Transicao::EntrarNoPerfilDeJogo => {
            let perfil = aberto?;
            let r = aplicar_parametros(&format!("jogo:{}", perfil.executavel), perfil.parametros);
            vigia.estado = Some(motor::EstadoDinamico::Jogo);
            vigia.jogo = Some(perfil.executavel.clone());
            Some(EventoDinamico { transicao: motor::Transicao::EntrarNoPerfilDeJogo, jogo: vigia.jogo.clone(), erro: r.err() })
        }
        motor::Transicao::VoltarAoNormal => {
            let normal = ler_backup().map(|b| b.plano_anterior).unwrap_or_else(|| EQUILIBRADO_GUID.to_string());
            let r = power::set_active_scheme(&normal);
            vigia.estado = Some(motor::EstadoDinamico::Normal);
            let jogo = vigia.jogo.take();
            Some(EventoDinamico { transicao: motor::Transicao::VoltarAoNormal, jogo, erro: r.err() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Leitura real desta máquina, sem escrever nada. Roda com
    /// `cargo test --lib -- --ignored motorenergia_maquina`.
    #[test]
    #[ignore]
    fn leitura_real_sem_escrever() {
        let i = impressao();
        println!("{:#?}", i);
        let base = enumerar_base().expect("enumera o Equilibrado");
        println!("ajustes no Equilibrado: {}", base.configuracoes.len());
        let b = bateria_de_candidatos(&i, &base);
        for c in &b.autoajuste {
            println!("{} {:?}: {:?} | ignorados {:?}", c.id, c.parametros, c.mudancas.iter().map(|m| (&m.alias, m.ac, m.dc)).collect::<Vec<_>>(), c.ignorados);
        }
        println!("amostra: {:?}", amostra_agora());
        let (r, _) = teste_de_rajada();
        println!("rajada: {:?}", r);
    }
}
