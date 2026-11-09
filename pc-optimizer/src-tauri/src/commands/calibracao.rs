// Calibração de energia por candidatos (Plano 1.1, Onda 3b): o roteiro que a tela antiga conduzia em TypeScript
// (`src/energia.ts`, `testarBateria`) agora roda aqui, num comando só, e a tela nova só mostra o progresso e o resultado.
//
// Roteiro, igual ao de antes: mede o plano em uso SEM escrever nada; testa cada candidato do autoajuste no plano OTIMIZA
// (o do cliente não é tocado; o backup vem antes); mede o padrão de novo a cada dois candidatos e no fim, para descontar
// o aquecimento (`corrigir_deriva`); escolhe; mede os vizinhos do vencedor e escolhe de novo; e VOLTA ao plano de antes.
// O vencedor só entra quando a pessoa aplicar (`energia_aplicar_calibrado`). Nenhum plano que tire FPS é recomendado: a
// regra mora em `motorenergia::escolher`.

use crate::modules::windows::motorenergia as motor;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static CANCELAR: AtomicBool = AtomicBool::new(false);
static RODANDO: AtomicBool = AtomicBool::new(false);
/// O vencedor da última calibração, até a pessoa aplicar ou calibrar de novo.
static VENCEDOR: Mutex<Option<motor::Parametros>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct PassoDaCalibracao {
    pub passo: u32,
    pub total: u32,
    pub frase: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct AchadoDaCalibracao {
    pub texto: String,
    /// `ok`, `atencao`, `perigo` ou `neutro`.
    pub tom: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct NotaDoCandidato {
    pub rotulo: String,
    pub nota: f64,
    pub vencedor: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct ResultadoDaCalibracao {
    /// O rótulo do vencedor; `None` quando nada ganhou com margem.
    pub vencedor: Option<String>,
    /// Ganhou o plano que a pessoa já usa: não há o que aplicar.
    pub fica_o_atual: bool,
    pub pode_aplicar: bool,
    pub confianca_pct: f64,
    pub fps_pct: Option<f64>,
    pub low_1_pct: Option<f64>,
    pub p99_pct: Option<f64>,
    pub temperatura_delta_c: Option<f64>,
    pub candidatos: Vec<NotaDoCandidato>,
    pub achados: Vec<AchadoDaCalibracao>,
    pub mediu_com_jogo: bool,
    /// Falha ao voltar ao plano de antes no fim (o resto deu certo).
    pub aviso: Option<String>,
}

/// **Pura.** O nome de cada papel na tela.
pub fn rotulo_do_papel(p: motor::Papel, notebook: bool) -> &'static str {
    use motor::Papel::*;
    match p {
        PadraoWindows => "Padrão do Windows",
        A if notebook => "Tomada competitivo",
        A => "Resposta máxima",
        B if notebook => "Tomada equilibrado",
        B => "Adaptativo",
        C => "Autonomia do processador",
        D => "Tudo no máximo",
        Refino => "Ajuste fino do vencedor",
        Epp => "Laboratório de EPP",
        Dispositivo => "Política de dispositivo",
    }
}

/// **Pura.** Cada achado do motor vira a frase da tela (os mesmos textos da tela antiga, mais curtos).
pub fn achado(a: motor::Achado) -> AchadoDaCalibracao {
    use motor::Achado::*;
    let (texto, tom) = match a {
        FirmwareLimitaPorTemperatura => (
            "O processador foi freado por temperatura durante a medida. Nenhum plano resolve isso: é resfriamento (pasta térmica, poeira, ventilação).",
            "perigo",
        ),
        FirmwareLimitaPorEnergia => (
            "O processador bateu no limite de energia do fabricante. O plano não passa desse teto, e o Otimiza não finge que passa.",
            "perigo",
        ),
        ClockReportadoSubiuEfetivoCaiu => (
            "Um candidato subiu o clock que o Windows mostra, mas o clock real caiu. Isso não contou como melhora.",
            "atencao",
        ),
        SemJogoMedido => (
            "Sem jogo aberto, a escolha se apoia no teste de quadros do Otimiza. Para a palavra final, calibre com o jogo aberto.",
            "atencao",
        ),
        TemperaturaIndisponivel => (
            "Este Windows não mostra a temperatura real do processador: o quanto cada plano esquenta ficou fora da nota.",
            "neutro",
        ),
        LimitesDeFirmwareIndisponiveis => (
            "Não deu para ler se o processador estava sendo freado pelo fabricante; a nota usa só o desempenho medido.",
            "neutro",
        ),
        AmostragemFalha => (
            "Os contadores do processador falharam em parte da medida. Repita a calibração com o PC sem outras tarefas.",
            "atencao",
        ),
        NenhumGanhouComMargem => (
            "Nenhum candidato ganhou do padrão do Windows com folga. A recomendação é o padrão.",
            "neutro",
        ),
        PlanoAtualJaEOMelhor => (
            "O plano que você já usa não perdeu para nenhum candidato em FPS nem no 1% pior. Ele fica: trocar seria tirar FPS.",
            "ok",
        ),
        CandidatoTirariaFps => (
            "Pelo menos um candidato deu menos FPS e foi descartado. Plano que tira FPS nunca é recomendado.",
            "neutro",
        ),
        QuadrosSinteticos => (
            "Sem jogo, o FPS veio do teste de quadros do Otimiza (trabalho parecido com o de um jogo).",
            "neutro",
        ),
        EmpateDesfeitoPelaTemperatura => ("Deu empate no desempenho; ganhou o que esquenta menos.", "ok"),
        PoucasRepeticoes => ("Uma medida por candidato. Com duas, a confiança sobe.", "atencao"),
        MedicaoInstavel => (
            "O FPS variou muito entre as medidas. Repita numa cena mais parada do jogo.",
            "atencao",
        ),
    };
    AchadoDaCalibracao { texto: texto.to_string(), tom: tom.to_string() }
}

/// **Pura.** A escolha do motor vira o resultado da tela. `rotulos`: id do candidato → nome na tela.
pub fn resumir(escolha: &motor::Escolha, rotulos: &[(String, String)], mediu_com_jogo: bool) -> ResultadoDaCalibracao {
    let nome = |id: &str| -> String {
        if id == "atual" {
            return "O plano que você usa hoje".to_string();
        }
        rotulos
            .iter()
            .find(|(i, _)| i == id)
            .map(|(_, r)| r.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let fica_o_atual = escolha.vencedor.as_deref() == Some("atual")
        || escolha.achados.contains(&motor::Achado::PlanoAtualJaEOMelhor);
    let mut candidatos: Vec<NotaDoCandidato> = escolha
        .notas
        .iter()
        .map(|(id, n)| NotaDoCandidato {
            rotulo: nome(id),
            nota: (n.total * 10.0).round() / 10.0,
            vencedor: escolha.vencedor.as_deref() == Some(id.as_str()),
        })
        .collect();
    candidatos.sort_by(|a, b| b.nota.total_cmp(&a.nota));
    ResultadoDaCalibracao {
        vencedor: escolha.vencedor.as_deref().map(nome),
        fica_o_atual,
        pode_aplicar: escolha.vencedor.is_some() && !fica_o_atual,
        confianca_pct: escolha.confianca_pct,
        fps_pct: escolha.fps_pct,
        low_1_pct: escolha.low_1_pct,
        p99_pct: escolha.p99_pct,
        temperatura_delta_c: escolha.temperatura_delta_c,
        candidatos,
        achados: escolha.achados.iter().map(|a| achado(*a)).collect(),
        mediu_com_jogo,
        aviso: None,
    }
}

/// O roteiro na máquina. `avisar` recebe cada passo. Devolve o resultado e os parâmetros do vencedor (se houver).
#[cfg(target_os = "windows")]
fn calibrar(
    processo: Option<String>,
    segundos: u64,
    repeticoes: u32,
    otimiza_no_historico: bool,
    avisar: &dyn Fn(PassoDaCalibracao),
) -> Result<(ResultadoDaCalibracao, Option<motor::Parametros>), String> {
    use crate::modules::windows::motorenergia_maquina as maquina;

    let impressao = maquina::impressao();
    let notebook = impressao.formato == motor::Formato::Notebook;
    let base_enum = maquina::enumerar_base()?;
    let bateria = maquina::bateria_de_candidatos(&impressao, &base_enum);
    let mut controlados = bateria.controlados.clone();
    let lista = bateria.autoajuste.clone();
    let Some(base) = lista.first().cloned() else {
        return Err("Este processador não tem candidatos de energia para testar.".to_string());
    };
    let com_jogo = processo.is_some();
    let cena = if com_jogo { "Continue jogando na mesma cena." } else { "Deixe o PC parado." };

    // Passos: atual + candidatos + bases extras + uns 3 de refino + base final do refino.
    let bases_extras = (2..lista.len()).filter(|i| i % 2 == 1).count() as u32 + 1;
    let mut total = 1 + lista.len() as u32 + bases_extras + 4;
    let passo = std::cell::Cell::new(0u32);
    let avancar = |frase: String, total: u32| {
        passo.set(passo.get() + 1);
        avisar(PassoDaCalibracao { passo: passo.get(), total, frase });
    };
    let cancelado = || CANCELAR.load(Ordering::SeqCst);

    let mut rotulos: Vec<(String, String)> = Vec::new();
    let mut resultados: Vec<motor::ResultadoDoCandidato> = Vec::new();

    avancar(
        format!(
            "Medindo o plano que você usa hoje ({}) antes de mudar qualquer coisa. {}",
            impressao.plano_ativo_nome.as_deref().unwrap_or("atual"),
            cena
        ),
        total,
    );
    let mut atual = maquina::medir_atual("atual", processo.as_deref(), segundos, repeticoes)?;
    atual.resultado.candidato = "atual".to_string();
    resultados.push(atual.resultado);

    maquina::marcar_teste_em_andamento(true);
    let mut testar = |c: &motor::Candidato, id: &str| -> Result<motor::ResultadoDoCandidato, String> {
        for m in &c.mudancas {
            if !controlados.contains(&m.alias) {
                controlados.push(m.alias.clone());
            }
        }
        maquina::aplicar(c, &controlados)?;
        let mut m = maquina::medir_atual(&c.id, processo.as_deref(), segundos, repeticoes)?;
        m.resultado.candidato = id.to_string();
        Ok(m.resultado)
    };

    let mut repeticoes_da_base = 1;
    let mut erro: Option<String> = None;
    let mut escolha: Option<motor::Escolha> = None;

    'bateria: {
        for (i, c) in lista.iter().enumerate() {
            if cancelado() {
                erro = Some("Calibração cancelada.".into());
                break 'bateria;
            }
            if i >= 2 && i % 2 == 1 {
                repeticoes_da_base += 1;
                avancar(format!("Conferindo o padrão de novo ({}ª vez) para descontar o aquecimento…", repeticoes_da_base), total);
                match testar(&base, &format!("{}#{}", base.id, repeticoes_da_base)) {
                    Ok(r) => resultados.push(r),
                    Err(e) => {
                        erro = Some(format!("Parou ao medir o padrão de novo: {e}"));
                        break 'bateria;
                    }
                }
            }
            let rotulo = rotulo_do_papel(c.papel, notebook).to_string();
            rotulos.push((c.id.clone(), rotulo.clone()));
            avancar(format!("Candidato {} de {}: {}. {}", i + 1, lista.len(), rotulo, cena), total);
            match testar(c, &c.id) {
                Ok(r) => resultados.push(r),
                Err(e) => {
                    erro = Some(format!("Parou no candidato \"{rotulo}\": {e}"));
                    break 'bateria;
                }
            }
        }
        if lista.len() > 1 {
            repeticoes_da_base += 1;
            avancar("Conferindo o padrão uma última vez…".into(), total);
            match testar(&base, &format!("{}#{}", base.id, repeticoes_da_base)) {
                Ok(r) => resultados.push(r),
                Err(e) => {
                    erro = Some(format!("Parou ao medir o padrão de novo: {e}"));
                    break 'bateria;
                }
            }
        }
        escolha = motor::escolher(&motor::corrigir_deriva(&resultados, &base.id), &base.id);

        // Refino em volta do vencedor (não quando ganhou o atual ou nada).
        let vencedor = escolha
            .as_ref()
            .and_then(|e| e.vencedor.clone())
            .and_then(|id| lista.iter().find(|c| c.id == id).cloned());
        if let Some(v) = vencedor {
            let refino: Vec<motor::Candidato> = motor::vizinhos_do_vencedor(v.parametros, &impressao)
                .into_iter()
                .enumerate()
                .map(|(i, p)| motor::gerar(&impressao, &base_enum, &format!("refino-{}", i + 1), motor::Papel::Refino, p))
                .collect();
            total = passo.get() + refino.len() as u32 + 1;
            for (i, c) in refino.iter().enumerate() {
                if cancelado() {
                    erro = Some("Calibração cancelada.".into());
                    break 'bateria;
                }
                let rotulo = format!("Ajuste fino {} do vencedor", i + 1);
                rotulos.push((c.id.clone(), rotulo.clone()));
                avancar(format!("{} de {}. {}", rotulo, refino.len(), cena), total);
                match testar(c, &c.id) {
                    Ok(r) => resultados.push(r),
                    Err(e) => {
                        erro = Some(format!("Parou no ajuste fino: {e}"));
                        break 'bateria;
                    }
                }
            }
            if !refino.is_empty() {
                repeticoes_da_base += 1;
                avancar("Conferindo o padrão no fim do ajuste fino…".into(), total);
                if let Ok(r) = testar(&base, &format!("{}#{}", base.id, repeticoes_da_base)) {
                    resultados.push(r);
                }
            }
            escolha = motor::escolher(&motor::corrigir_deriva(&resultados, &base.id), &base.id);
            for c in &refino {
                if escolha.as_ref().and_then(|e| e.vencedor.as_deref()) == Some(c.id.as_str()) {
                    *VENCEDOR.lock().unwrap_or_else(|e| e.into_inner()) = Some(c.parametros);
                }
            }
        }
    }

    // Sempre volta ao plano de antes: o vencedor só entra quando a pessoa aplicar.
    let volta = maquina::restaurar_anterior(otimiza_no_historico);
    maquina::marcar_teste_em_andamento(false);

    if let Some(e) = erro {
        return Err(e);
    }
    let escolha = escolha.ok_or("A medida do padrão do Windows não deu número; não há com o que comparar.")?;
    let parametros = {
        let guardado = *VENCEDOR.lock().unwrap_or_else(|e| e.into_inner());
        guardado.or_else(|| {
            escolha
                .vencedor
                .as_deref()
                .and_then(|id| lista.iter().find(|c| c.id == id))
                .map(|c| c.parametros)
        })
    };
    rotulos.push((base.id.clone(), rotulo_do_papel(base.papel, notebook).to_string()));
    let mut r = resumir(&escolha, &rotulos, com_jogo);
    if r.fica_o_atual {
        r.pode_aplicar = false;
    }
    if let Err(e) = volta {
        r.aviso = Some(format!("Não consegui voltar sozinho ao plano de antes: {e}. Use \"Voltar ao meu plano\"."));
    }
    let pode = r.pode_aplicar;
    Ok((r, parametros.filter(|_| pode)))
}

/// Roda a calibração inteira (uns 2 a 5 min) e manda cada passo no evento `energia:calibracao`. Exige administrador.
/// `processo`: o executável do jogo aberto, para medir com ele (sem jogo, mede pelo teste de quadros). `EXIGEM_LICENCA`:
/// escreve no plano OTIMIZA durante os testes.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn energia_calibrar(processo: Option<String>) -> Result<ResultadoDaCalibracao, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        use tauri::{Emitter, Manager};
        if !crate::modules::windows::registry::is_elevated() {
            return Err("A calibração troca o plano de energia: abra o Otimiza como administrador.".to_string());
        }
        if crate::modules::provaalternada::em_andamento() {
            return Err("A prova do Otimizar está rodando e troca o plano de energia. Espere ela terminar.".to_string());
        }
        if RODANDO.swap(true, Ordering::SeqCst) {
            return Err("Já tem uma calibração rodando.".to_string());
        }
        CANCELAR.store(false, Ordering::SeqCst);
        *VENCEDOR.lock().unwrap_or_else(|e| e.into_inner()) = None;

        let app = crate::aplicativo()?;
        let otimiza = app
            .state::<super::AppState>()
            .changes
            .lock()
            .await
            .is_applied("plano_otimiza");
        let processo = processo.map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
        let r = tokio::task::spawn_blocking(move || {
            calibrar(processo, 15, 1, otimiza, &|p| {
                let _ = app.emit("energia:calibracao", p);
            })
        })
        .await
        .map_err(|_| "A calibração parou no meio.".to_string());
        RODANDO.store(false, Ordering::SeqCst);
        let (resultado, parametros) = r??;
        // Comunidade (só se a pessoa participa): a receita que ganhou, medida COM o jogo.
        if let (true, true, Some(v), Some(j)) = (
            resultado.mediu_com_jogo,
            resultado.pode_aplicar,
            resultado.vencedor.clone(),
            crate::plataforma::sessao::atual(),
        ) {
            tauri::async_runtime::spawn(crate::plataforma::comunidade::enviar(serde_json::json!({
                "jogo": j.nome,
                "receita": format!("energia:{v}"),
                "ganho_fps_pct": resultado.fps_pct,
                "ganho_low_pct": resultado.low_1_pct,
            })));
        }
        *VENCEDOR.lock().unwrap_or_else(|e| e.into_inner()) = parametros;
        Ok(resultado)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = processo;
        Err(super::UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Pede para parar depois do passo em andamento (o plano de antes volta do mesmo jeito). `LIVRES`: é desfazer.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn energia_calibrar_cancelar() {
    CANCELAR.store(true, Ordering::SeqCst);
}

/// Aplica no plano OTIMIZA o vencedor da última calibração, relendo do Windows. `EXIGEM_LICENCA`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn energia_aplicar_calibrado() -> Result<String, String> {
    crate::modules::licenca::exigir()?;
    let Some(p) = *VENCEDOR.lock().unwrap_or_else(|e| e.into_inner()) else {
        return Err("Não há vencedor para aplicar: calibre de novo.".to_string());
    };
    tokio::task::spawn_blocking(move || {
        use crate::modules::windows::motorenergia_maquina as maquina;
        let a = maquina::aplicar_parametros("escolhido", p)?;
        maquina::marcar_teste_em_andamento(false);
        Ok(if !a.ativo {
            "Os valores foram escritos, mas o plano OTIMIZA não ficou ativo.".to_string()
        } else if a.divergentes.is_empty() {
            "Pronto: o vencedor está no plano OTIMIZA, conferido relendo do Windows.".to_string()
        } else {
            format!(
                "Aplicado no plano OTIMIZA, exceto {} valor(es) que o Windows não aceitou: {}.",
                a.divergentes.len(),
                a.divergentes.join(", ")
            )
        })
    })
    .await
    .map_err(|_| "A aplicação parou no meio.".to_string())?
}

/// Guarda o vencedor da última calibração como o perfil de energia de `executavel`: com o modo dinâmico ligado, o
/// Otimiza troca para ele quando esse jogo abre. Só grava no arquivo do Otimiza. `LIVRES`. Devolve os executáveis com perfil.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn energia_salvar_calibrado_para_o_jogo(executavel: String) -> Result<Vec<String>, String> {
    let executavel = executavel.trim().to_string();
    if executavel.is_empty() {
        return Err("Abra o jogo para guardar o perfil dele.".to_string());
    }
    let Some(parametros) = *VENCEDOR.lock().unwrap_or_else(|e| e.into_inner()) else {
        return Err("Não há vencedor para guardar: calibre de novo, com o jogo aberto.".to_string());
    };
    use crate::modules::windows::motorenergia_maquina as maquina;
    maquina::salvar_perfil_de_jogo(maquina::PerfilDeJogo {
        executavel,
        parametros,
        escolha: None,
        quando: crate::modules::changelog::now_timestamp(),
    })
    .map(|l| l.into_iter().map(|p| p.executavel).collect())
}

#[cfg(test)]
mod testes {
    use super::*;

    fn nota(total: f64) -> motor::Nota {
        motor::Nota {
            total,
            low_1: None,
            p99: None,
            fps: None,
            resposta: None,
            clock: None,
            termica: None,
            estabilidade: None,
        }
    }

    fn escolha(vencedor: Option<&str>, achados: Vec<motor::Achado>) -> motor::Escolha {
        motor::Escolha {
            notas: vec![("base".into(), nota(50.0)), ("a".into(), nota(61.24)), ("atual".into(), nota(55.0))],
            vencedor: vencedor.map(String::from),
            confianca_pct: 82.0,
            achados,
            fps_pct: Some(4.2),
            low_1_pct: Some(6.0),
            p99_pct: Some(-3.0),
            resposta_pct: None,
            temperatura_delta_c: Some(1.5),
        }
    }

    fn rotulos() -> Vec<(String, String)> {
        vec![("base".into(), "Padrão do Windows".into()), ("a".into(), "Resposta máxima".into())]
    }

    #[test]
    fn o_vencedor_vem_com_nome_e_as_notas_em_ordem() {
        let r = resumir(&escolha(Some("a"), vec![]), &rotulos(), true);
        assert_eq!(r.vencedor.as_deref(), Some("Resposta máxima"));
        assert!(r.pode_aplicar && !r.fica_o_atual);
        assert_eq!(r.candidatos[0].rotulo, "Resposta máxima");
        assert_eq!(r.candidatos[0].nota, 61.2);
        assert!(r.candidatos[0].vencedor);
        assert_eq!(r.candidatos[1].rotulo, "O plano que você usa hoje");
    }

    #[test]
    fn quando_o_atual_e_o_melhor_nao_ha_o_que_aplicar() {
        let r = resumir(&escolha(None, vec![motor::Achado::PlanoAtualJaEOMelhor]), &rotulos(), true);
        assert!(r.fica_o_atual && !r.pode_aplicar);
        assert_eq!(r.achados[0].tom, "ok");
    }

    #[test]
    fn freio_por_temperatura_e_dito_como_perigo() {
        let r = resumir(&escolha(Some("a"), vec![motor::Achado::FirmwareLimitaPorTemperatura]), &rotulos(), false);
        assert_eq!(r.achados[0].tom, "perigo");
        assert!(r.achados[0].texto.contains("resfriamento"));
        assert!(!r.mediu_com_jogo);
    }

    #[test]
    fn todo_papel_tem_nome() {
        use motor::Papel::*;
        for p in [PadraoWindows, A, B, C, D, Refino, Epp, Dispositivo] {
            assert!(!rotulo_do_papel(p, true).is_empty());
            assert!(!rotulo_do_papel(p, false).is_empty());
        }
    }
}
