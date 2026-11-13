// A PLACA DE REDE COM PROVA (Plano de refino, R2). Moderação de interrupção, Ethernet de economia (EEE) e controle de
// fluxo são as dicas de "rede para jogo" mais copiadas, e cada placa reage de um jeito. Aqui cada uma é TESTADA nesta
// placa: 20 s de ping parado, muda, espera a placa voltar, mais 20 s. Só fica o que baixou a variação do ping sem
// aumentar a perda nem o ping; o resto volta na hora. O que fica entra no histórico, com desfazer.
//
// Cuidado escrito na tela: mudar uma propriedade reinicia a placa (a internet cai alguns segundos), e a moderação
// desligada faz o processador atender cada pacote (jogo preso no processador pode perder um pouco de FPS: o portão da
// partida confere). Não roda com jogo aberto.

use serde::Serialize;

use super::rede_partida::JanelaDeRede;

/// (palavra do registro, valor do teste, nome na tela). O valor do teste é "desligado" nas quatro.
pub const CANDIDATOS: &[(&str, &str, &str)] = &[
    ("*InterruptModeration", "0", "Moderação de interrupção"),
    ("*EEE", "0", "Ethernet de economia (EEE)"),
    ("EEELinkAdvertisement", "0", "Ethernet de economia (EEE)"),
    ("EnableGreenEthernet", "0", "Ethernet verde"),
    ("*FlowControl", "0", "Controle de fluxo"),
];

/// A variação precisa cair pelo menos isto para o ajuste ficar: menos que isso é ruído de 20 s.
pub const MELHORA_MINIMA_PCT: f64 = 15.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "DesfechoDaPlaca"))]
pub enum Desfecho {
    Ficou,
    Voltou,
    /// A placa não voltou a responder depois da troca: desfeito na hora.
    PlacaNaoVoltou,
    /// O Windows recusou a troca (driver não aceita o valor).
    Recusado,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "TentativaNaPlaca"))]
pub struct Tentativa {
    pub nome: String,
    pub desfecho: Desfecho,
    pub variacao_antes_ms: Option<f64>,
    pub variacao_depois_ms: Option<f64>,
    pub frase: String,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct ProvaDaPlaca {
    pub placa: String,
    pub tentativas: Vec<Tentativa>,
    pub frase: String,
}

/// **Pura.** O ajuste fica? Variação menor por `MELHORA_MINIMA_PCT`, sem perda maior e sem ping mais de 2 ms acima.
pub fn vale_manter(antes: &JanelaDeRede, depois: &JanelaDeRede) -> bool {
    let caiu = antes.variacao_ms > 0.0 && (antes.variacao_ms - depois.variacao_ms) / antes.variacao_ms * 100.0 >= MELHORA_MINIMA_PCT;
    caiu && depois.perda_pct <= antes.perda_pct + 0.5 && depois.ping_ms <= antes.ping_ms + 2.0
}

/// **Pura.** O valor do teste só serve se o driver aceita (lista vazia: o driver não informou, tenta).
pub fn aceita(valor: &str, aceitos: &[String]) -> bool {
    aceitos.is_empty() || aceitos.iter().any(|a| a == valor)
}

#[cfg(target_os = "windows")]
mod medicao {
    use super::*;
    use crate::modules::windows::pacote;
    use std::time::Duration;

    const JANELA: Duration = Duration::from_secs(20);

    fn janela(nome: &str) -> Option<JanelaDeRede> {
        let ecos = crate::plataforma::fila::ecos(JANELA);
        crate::plataforma::rede_partida::resumir_janela(nome, 0, &ecos)
    }

    /// Espera a placa voltar (até 25 s): um eco que responde.
    fn voltou() -> bool {
        for _ in 0..25 {
            std::thread::sleep(Duration::from_secs(1));
            if crate::plataforma::fila::ecos(Duration::from_millis(900)).iter().any(|e| e.is_some()) {
                // Mais 3 s para a placa assentar antes de medir.
                std::thread::sleep(Duration::from_secs(3));
                return true;
            }
        }
        false
    }

    /// Bloqueia uns 50 s por propriedade que a placa tem. Grava no histórico o que ficou.
    pub fn provar(log: &mut crate::modules::changelog::ChangeLog) -> Result<ProvaDaPlaca, String> {
        use crate::modules::changelog::{now_timestamp, AppliedOptimization, ChangeRecord};
        if crate::plataforma::sessao::atual().is_some() {
            return Err("Feche o jogo antes: o teste reinicia a placa de rede e a internet cai alguns segundos.".to_string());
        }
        if !crate::modules::windows::registry::is_elevated() {
            return Err("O teste muda a placa de rede: abra o Otimiza como administrador.".to_string());
        }
        let placa = pacote::placa_da_rota().ok_or(
            "Não achei a placa física por onde sai a internet (VPN ligada, ou sem rede). Desligue a VPN e tente de novo.",
        )?;
        let mut tentativas = Vec::new();
        let mut vistas: Vec<&str> = Vec::new();
        for (palavra, teste, nome) in CANDIDATOS {
            let Some((atual, aceitos)) = pacote::ler_propriedade(&placa, palavra) else {
                continue;
            };
            // Intel e Realtek dão nomes diferentes para o mesmo EEE: só a primeira que a placa tiver.
            if vistas.contains(nome) || atual == *teste || !aceita(teste, &aceitos) {
                continue;
            }
            vistas.push(nome);
            let Some(antes) = janela(&placa) else {
                return Err("O ping parou de responder antes do teste: confira a internet e tente de novo.".to_string());
            };
            crate::utils::Logger::info(&format!("placa com prova: {} {} → {}", placa, palavra, teste));
            if let Err(e) = pacote::definir_propriedade(&placa, palavra, teste) {
                tentativas.push(Tentativa {
                    nome: nome.to_string(),
                    desfecho: Desfecho::Recusado,
                    variacao_antes_ms: Some(antes.variacao_ms),
                    variacao_depois_ms: None,
                    frase: format!("O driver da placa recusou: {}", e),
                });
                continue;
            }
            if !voltou() {
                let _ = pacote::definir_propriedade(&placa, palavra, &atual);
                let _ = voltou();
                tentativas.push(Tentativa {
                    nome: nome.to_string(),
                    desfecho: Desfecho::PlacaNaoVoltou,
                    variacao_antes_ms: Some(antes.variacao_ms),
                    variacao_depois_ms: None,
                    frase: "A placa não voltou a responder com a mudança: desfeito na hora.".to_string(),
                });
                continue;
            }
            let depois = janela(&placa);
            let fica = depois.as_ref().is_some_and(|d| vale_manter(&antes, d));
            if fica {
                let change = ChangeRecord::PropriedadeDaPlaca {
                    placa: placa.clone(),
                    palavra: palavra.to_string(),
                    anterior: atual.clone(),
                };
                if let Err(e) = log.record(AppliedOptimization {
                    optimization_id: format!("placa_prova:{}", palavra.trim_start_matches('*')),
                    name: format!("{} desligada na placa {} (provado no ping)", nome, placa),
                    timestamp: now_timestamp(),
                    changes: vec![change],
                }) {
                    let _ = pacote::definir_propriedade(&placa, palavra, &atual);
                    let _ = voltou();
                    return Err(format!("O histórico não gravou ({}); a mudança foi desfeita.", e));
                }
            } else {
                let _ = pacote::definir_propriedade(&placa, palavra, &atual);
                let _ = voltou();
            }
            let (va, vd) = (antes.variacao_ms, depois.as_ref().map(|d| d.variacao_ms));
            tentativas.push(Tentativa {
                nome: nome.to_string(),
                desfecho: if fica { Desfecho::Ficou } else { Desfecho::Voltou },
                variacao_antes_ms: Some(va),
                variacao_depois_ms: vd,
                frase: match (fica, vd) {
                    (true, Some(d)) => format!("Ficou: a variação do ping caiu de {:.1} para {:.1} ms.", va, d),
                    (false, Some(d)) => format!("Voltou como estava: variação de {:.1} para {:.1} ms, sem melhora que passe do ruído.", va, d),
                    _ => "Voltou como estava: o ping não respondeu depois da troca.".to_string(),
                },
            });
        }
        let ficaram = tentativas.iter().filter(|t| t.desfecho == Desfecho::Ficou).count();
        let frase = if tentativas.is_empty() {
            "Esta placa já está como o teste deixaria, ou o driver não mostra essas opções. Nada a mudar.".to_string()
        } else if ficaram == 0 {
            "Nenhuma mudança melhorou o ping nesta placa: tudo voltou como estava.".to_string()
        } else {
            format!("{} de {} mudanças melhoraram o ping nesta placa e ficaram (no histórico, com desfazer).", ficaram, tentativas.len())
        };
        Ok(ProvaDaPlaca { placa, tentativas, frase })
    }
}

#[cfg(target_os = "windows")]
pub use medicao::provar;

#[cfg(test)]
mod tests {
    use super::*;

    fn j(ping: f64, variacao: f64, perda: f64) -> JanelaDeRede {
        JanelaDeRede { jogo: String::new(), quando: 0, ping_ms: ping, variacao_ms: variacao, perda_pct: perda }
    }

    #[test]
    fn so_fica_o_que_baixa_a_variacao_sem_piorar_o_resto() {
        assert!(vale_manter(&j(20.0, 6.0, 0.0), &j(20.0, 3.0, 0.0)));
        assert!(!vale_manter(&j(20.0, 6.0, 0.0), &j(20.0, 5.5, 0.0)), "ruído");
        assert!(!vale_manter(&j(20.0, 6.0, 0.0), &j(20.0, 3.0, 4.0)), "perdeu pacote");
        assert!(!vale_manter(&j(20.0, 6.0, 0.0), &j(25.0, 3.0, 0.0)), "ping subiu");
        assert!(!vale_manter(&j(20.0, 0.0, 0.0), &j(20.0, 0.0, 0.0)), "nada para melhorar");
    }

    #[test]
    fn o_valor_do_teste_precisa_ser_aceito_pelo_driver() {
        assert!(aceita("0", &["0".into(), "1".into()]));
        assert!(!aceita("0", &["1".into(), "2".into()]));
        assert!(aceita("0", &[]));
    }

    #[test]
    fn so_propriedades_da_lista_permitida() {
        for (palavra, valor, _) in CANDIDATOS {
            assert!(crate::modules::windows::pacote::PROPRIEDADES_PERMITIDAS.contains(palavra), "{palavra}");
            assert!(crate::modules::windows::pacote::palavra_segura(valor));
        }
    }
}
