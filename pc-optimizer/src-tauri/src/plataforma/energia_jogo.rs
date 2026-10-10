// ENERGIA DE JOGO. Enquanto o jogo está aberto, o processador sobe de frequência sem hesitar; quando ele fecha, volta
// ao que estava. Medido no PC do dono (i3-10100F, plano OTIMIZA): núcleos já acordados e boost já agressivo, mas a
// preferência entre energia e desempenho (EPP) em 33 — o processador espera a carga se firmar antes de subir, e num jogo
// preso no processador cada espera é um quadro atrasado.
//
// Sem a medição longa do motor de energia (aba Energia): só um ajuste, o que comanda a frequência, escrito no plano
// ATIVO com o valor de antes guardado em disco. Volta quando o jogo fecha e, se o Otimiza cair no meio, na abertura
// seguinte. Notebook: só na tomada (na bateria, economia é o que a pessoa quer).

use crate::modules::windows::motorenergia::{Configuracao, Controle, Enumeracao, Formato};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Os apelidos do `powercfg` (os mesmos de `motorenergia::apelidos`).
const EPP: &str = "PERFEPP";
const MINIMO: &str = "PROCTHROTTLEMIN";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guardado {
    pub alias: String,
    pub subgrupo: String,
    pub ajuste: String,
    pub antes_ac: Option<u32>,
    pub antes_dc: Option<u32>,
    pub novo_ac: Option<u32>,
    pub novo_dc: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aplicado {
    pub plano: String,
    pub jogo: String,
    pub valores: Vec<Guardado>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Estado {
    pub ligado: bool,
    /// Presente só com o jogo aberto e o ajuste escrito: é o caminho de volta.
    pub aplicado: Option<Aplicado>,
}

/// Para a tela.
#[derive(Debug, Clone, Serialize)]
pub struct Painel {
    pub ligado: bool,
    pub agindo_em: Option<String>,
    /// Por que não dá para ligar aqui (sem administrador, por exemplo).
    pub impedimento: Option<String>,
    /// O que vai mudar, em palavras, medido no plano ativo.
    pub o_que_muda: Option<String>,
}

/// **Pura.** O que escrever com o jogo aberto. Com Speed Shift/CPPC quem comanda a frequência é o EPP (0 = desempenho
/// total); sem ele, o estado mínimo. Notebook só no lado da tomada. Já no valor: nada a escrever.
pub fn alvos(controle_por_hardware: bool, notebook: bool, plano: &Enumeracao) -> Vec<Guardado> {
    let (alias, alvo) = if controle_por_hardware {
        (EPP, 0)
    } else {
        (MINIMO, 100)
    };
    let Some(c) = plano.por_alias(alias) else {
        return Vec::new();
    };
    let novo_ac = (c.ac != Some(alvo)).then_some(alvo);
    let novo_dc = (!notebook && c.dc != Some(alvo)).then_some(alvo);
    if novo_ac.is_none() && novo_dc.is_none() {
        return Vec::new();
    }
    vec![guardado(c, alias, novo_ac, novo_dc)]
}

fn guardado(c: &Configuracao, alias: &str, novo_ac: Option<u32>, novo_dc: Option<u32>) -> Guardado {
    Guardado {
        alias: alias.to_string(),
        subgrupo: c.subgrupo.clone(),
        ajuste: c.guid.clone(),
        antes_ac: c.ac,
        antes_dc: c.dc,
        novo_ac,
        novo_dc,
    }
}

/// **Pura.** A frase do que muda, com os números lidos.
pub fn descrever(valores: &[Guardado]) -> Option<String> {
    let v = valores.first()?;
    let antes = v
        .antes_ac
        .map(|x| x.to_string())
        .unwrap_or_else(|| "?".into());
    Some(match v.alias.as_str() {
        EPP => format!(
            "Preferência de energia do processador: {} → 0 (desempenho total) só com o jogo aberto. Ele sobe de frequência na hora em vez de esperar a carga se firmar.",
            antes
        ),
        _ => format!(
            "Estado mínimo do processador: {}% → 100% só com o jogo aberto. A frequência não cai entre um quadro e outro.",
            antes
        ),
    })
}

fn pasta() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("energia")
}

fn arquivo() -> PathBuf {
    pasta().join("energia_de_jogo.json")
}

/// Sem escolha gravada, vem ligada em PC fraco (a mesma régua do catálogo, `Condicao::PcFraco`): é onde o processador
/// demorar a subir mais vira tranco. Quem desligou fica desligado: a escolha gravada vence.
pub fn ler() -> Estado {
    let gravado = std::fs::read_to_string(arquivo()).ok();
    match gravado.and_then(|t| serde_json::from_str(&t).ok()) {
        Some(e) => e,
        None => Estado {
            ligado: pc_fraco(),
            aplicado: None,
        },
    }
}

/// **Pura.** Até 8 GB de memória ou até 4 núcleos lógicos.
pub fn e_pc_fraco(memoria_gb: f64, nucleos_logicos: usize) -> bool {
    (memoria_gb > 0.0 && memoria_gb <= 8.5) || (nucleos_logicos > 0 && nucleos_logicos <= 4)
}

fn pc_fraco() -> bool {
    let mut s = sysinfo::System::new();
    s.refresh_memory();
    let gb = s.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let nucleos = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    e_pc_fraco(gb, nucleos)
}

fn gravar(e: &Estado) -> Result<(), String> {
    std::fs::create_dir_all(pasta()).map_err(|e| e.to_string())?;
    let texto = serde_json::to_string_pretty(e).map_err(|e| e.to_string())?;
    std::fs::write(arquivo(), texto)
        .map_err(|e| format!("Não foi possível gravar a energia de jogo: {}", e))
}

#[cfg(windows)]
fn escrever(plano: &str, v: &Guardado, ac: Option<u32>, dc: Option<u32>) -> Result<(), String> {
    use crate::modules::windows::shell;
    for (lado, valor) in [("-setacvalueindex", ac), ("-setdcvalueindex", dc)] {
        let Some(valor) = valor else { continue };
        let valor = valor.to_string();
        let args = [
            lado,
            plano,
            v.subgrupo.as_str(),
            v.ajuste.as_str(),
            valor.as_str(),
        ];
        let s = shell::run("powercfg", &args)?;
        if !s.success {
            return Err(format!(
                "o Windows recusou `powercfg {}`: {}",
                args.join(" "),
                s.stderr.trim()
            ));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn impedimento() -> Option<String> {
    (!crate::modules::windows::registry::is_elevated()).then(|| {
        "Mudar a energia do processador exige abrir o Otimiza como administrador.".to_string()
    })
}

#[cfg(windows)]
fn ler_maquina() -> Result<(bool, bool, String, Enumeracao), String> {
    let i = crate::modules::windows::motorenergia_maquina::impressao();
    let hw = matches!(i.controle, Controle::HardwareHwp | Controle::HardwareCppc);
    let notebook = i.formato == Formato::Notebook;
    let ativo = crate::modules::windows::power::active_scheme()?;
    let plano = crate::modules::windows::motorenergia_maquina::enumerar_plano(&ativo)?;
    Ok((hw, notebook, ativo, plano))
}

#[cfg(windows)]
pub fn painel() -> Painel {
    let e = ler();
    let impedimento = impedimento();
    let o_que_muda = match &e.aplicado {
        Some(a) => descrever(&a.valores),
        None => ler_maquina()
            .ok()
            .and_then(|(hw, nb, _, p)| descrever(&alvos(hw, nb, &p))),
    };
    Painel {
        ligado: e.ligado,
        agindo_em: e.aplicado.map(|a| a.jogo),
        impedimento,
        o_que_muda,
    }
}

#[cfg(windows)]
pub fn definir(ligado: bool) -> Result<Painel, String> {
    if ligado {
        if let Some(i) = impedimento() {
            return Err(i);
        }
    }
    let mut e = ler();
    e.ligado = ligado;
    gravar(&e)?;
    if !ligado {
        devolver()?;
    } else if let Some(j) = super::sessao::atual() {
        // O vigia só age quando o jogo muda: ligado com ele já aberto, entra agora.
        entrar(&j.nome)?;
    }
    Ok(painel())
}

/// Com o jogo aberto: escreve e guarda o de antes. Já aplicado: nada.
#[cfg(windows)]
pub fn entrar(jogo: &str) -> Result<Option<String>, String> {
    let mut e = ler();
    if !e.ligado || e.aplicado.is_some() || impedimento().is_some() {
        return Ok(None);
    }
    let (hw, notebook, ativo, plano) = ler_maquina()?;
    let valores = alvos(hw, notebook, &plano);
    if valores.is_empty() {
        return Ok(None);
    }
    // Guardado ANTES de escrever: se cair no meio, a abertura seguinte devolve.
    e.aplicado = Some(Aplicado {
        plano: ativo.clone(),
        jogo: jogo.to_string(),
        valores: valores.clone(),
    });
    gravar(&e)?;
    for v in &valores {
        escrever(&ativo, v, v.novo_ac, v.novo_dc)?;
    }
    // Reativar é o que faz o Windows carregar o valor novo.
    crate::modules::windows::power::set_active_scheme(&ativo)?;
    Ok(descrever(&valores))
}

/// Devolve o que foi escrito. Chamado quando o jogo fecha, ao desligar e na abertura do Otimiza.
#[cfg(windows)]
pub fn devolver() -> Result<bool, String> {
    let mut e = ler();
    let Some(a) = e.aplicado.clone() else {
        return Ok(false);
    };
    for v in &a.valores {
        let ac = v.novo_ac.and(v.antes_ac);
        let dc = v.novo_dc.and(v.antes_dc);
        escrever(&a.plano, v, ac, dc)?;
    }
    if crate::modules::windows::power::active_scheme()
        .is_ok_and(|g| g.eq_ignore_ascii_case(&a.plano))
    {
        crate::modules::windows::power::set_active_scheme(&a.plano)?;
    }
    e.aplicado = None;
    gravar(&e)?;
    Ok(true)
}

/// Um passo do vigia: entra com o jogo aberto, devolve com ele fechado.
#[cfg(windows)]
pub fn passo(jogo_aberto: Option<&str>) -> Option<String> {
    let e = ler();
    match (jogo_aberto, e.aplicado.is_some()) {
        (Some(jogo), false) if e.ligado => match entrar(jogo) {
            Ok(Some(frase)) => Some(format!("Energia de jogo ligada para {}. {}", jogo, frase)),
            Ok(None) => None,
            Err(erro) => Some(format!("Energia de jogo: {}", erro)),
        },
        (None, true) => match devolver() {
            Ok(_) => Some("Energia de jogo: o processador voltou ao ajuste de antes.".to_string()),
            Err(erro) => Some(format!(
                "Energia de jogo: não consegui devolver ({}). Tento de novo.",
                erro
            )),
        },
        _ => None,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn conf(alias: &str, ac: u32, dc: u32) -> Configuracao {
        Configuracao {
            subgrupo: "54533251-82be-4824-96c1-47b60b740d00".into(),
            subgrupo_alias: Some("SUB_PROCESSOR".into()),
            guid: "g".into(),
            alias: Some(alias.into()),
            minimo: Some(0),
            maximo: Some(100),
            incremento: Some(1),
            unidade: None,
            possiveis: Vec::new(),
            ac: Some(ac),
            dc: Some(dc),
        }
    }

    fn plano(c: Vec<Configuracao>) -> Enumeracao {
        Enumeracao {
            plano: None,
            configuracoes: c,
        }
    }

    #[test]
    fn no_pc_do_dono_o_epp_33_vai_a_zero_nos_dois_lados() {
        let p = plano(vec![conf(EPP, 33, 50), conf(MINIMO, 5, 5)]);
        let v = alvos(true, false, &p);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].alias, EPP);
        assert_eq!((v[0].novo_ac, v[0].novo_dc), (Some(0), Some(0)));
        assert_eq!((v[0].antes_ac, v[0].antes_dc), (Some(33), Some(50)));
        assert!(descrever(&v).unwrap().contains("33 → 0"));
    }

    #[test]
    fn vem_ligada_no_pc_do_dono_e_nao_em_pc_folgado() {
        assert!(e_pc_fraco(7.9, 8), "i3-10100F com 8 GB");
        assert!(e_pc_fraco(16.0, 4));
        assert!(!e_pc_fraco(16.0, 12));
        assert!(!e_pc_fraco(0.0, 0), "sem leitura não liga");
    }

    #[test]
    fn notebook_so_mexe_na_tomada() {
        let p = plano(vec![conf(EPP, 33, 50)]);
        let v = alvos(true, true, &p);
        assert_eq!((v[0].novo_ac, v[0].novo_dc), (Some(0), None));
    }

    #[test]
    fn sem_speed_shift_quem_manda_e_o_minimo() {
        let p = plano(vec![conf(EPP, 33, 33), conf(MINIMO, 5, 5)]);
        let v = alvos(false, false, &p);
        assert_eq!(v[0].alias, MINIMO);
        assert_eq!(v[0].novo_ac, Some(100));
    }

    #[test]
    fn ja_no_valor_nao_escreve_nada() {
        let p = plano(vec![conf(EPP, 0, 0)]);
        assert!(alvos(true, false, &p).is_empty());
        assert!(alvos(true, false, &plano(Vec::new())).is_empty());
    }
}

#[cfg(all(test, windows))]
mod na_maquina {
    /// Só lê: `cargo test --lib -- --ignored energia_jogo::na_maquina`.
    #[test]
    #[ignore]
    fn painel_real_sem_escrever() {
        println!("{:#?}", super::painel());
    }
}
