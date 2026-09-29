// Resultado para compartilhar (MASTER-PLAN, item 9): o texto que o cliente cola no Discord depois da prova no jogo.
// Sai SÓ de prova alternada válida e SEMPRE com a margem de erro: o que se compartilha é o que foi provado, inclusive
// quando não mudou nada ou quando piorou. Print de FPS solto qualquer programa mostra; isto mostra a conta.

use crate::modules::provaalternada::{Desfecho, Resultado};
use crate::modules::repeticoes::{Diferenca, Resumo};

/// Por que não há o que compartilhar.
#[derive(Debug, Clone, PartialEq)]
pub enum Recusa {
    SemProva,
    /// Configuração, gerador ou medidor mudou no meio, ou faltou rodada.
    ProvaSemComparacaoJusta,
    /// Faltou a margem de algum lado (menos rodadas que o mínimo).
    SemMargem,
}

impl Recusa {
    pub fn frase(&self) -> &'static str {
        match self {
            Recusa::SemProva => "Ainda não há prova no jogo nesta máquina. Faça a prova na área Otimizar.",
            Recusa::ProvaSemComparacaoJusta => {
                "A última prova não teve comparação justa (algo mudou no meio ou faltou rodada). Faça a prova de novo."
            }
            Recusa::SemMargem => "A última prova não tem rodadas suficientes para ter margem de erro. Faça a prova de novo.",
        }
    }
}

fn br(v: f64, casas: usize) -> String {
    format!("{:.*}", casas, v).replace('.', ",")
}

fn lado(r: &Resumo) -> Option<String> {
    let m = r.margem?;
    // Arredondada para CIMA: mostrar margem menor que a medida é afirmar precisão que não existe. Abaixo de 1 FPS,
    // uma casa (zero casas viraria "± 0").
    let margem = if m < 1.0 { (m * 10.0).ceil() / 10.0 } else { m.ceil() };
    Some(format!("{} ± {}", br(r.media, 0), br(margem, if m < 1.0 { 1 } else { 0 })))
}

fn nome_do_jogo(exe: &str) -> String {
    let e = exe.to_lowercase();
    if e.contains("fivem") {
        "FiveM".into()
    } else if e.contains("gta5") {
        "GTA V".into()
    } else {
        exe.trim_end_matches(".exe").trim_end_matches(".EXE").to_string()
    }
}

/// **Pura.** `hardware`: processador e placa, quando lidos. `data`: dd/mm/aaaa da prova.
pub fn texto(prova: Option<&Resultado>, hardware: (Option<&str>, Option<&str>), data: &str) -> Result<String, Recusa> {
    let p = prova.ok_or(Recusa::SemProva)?;
    if p.desfecho == Desfecho::SemComparacaoJusta {
        return Err(Recusa::ProvaSemComparacaoJusta);
    }
    let (sem, com) = (p.fps_sem.as_ref().ok_or(Recusa::SemMargem)?, p.fps_com.as_ref().ok_or(Recusa::SemMargem)?);
    let fps = format!("{} → {}", lado(sem).ok_or(Recusa::SemMargem)?, lado(com).ok_or(Recusa::SemMargem)?);
    let pct = match &p.diferenca_fps {
        Some(Diferenca::Real { pct: Some(x), .. }) => format!(" ({}{}%)", if *x >= 0.0 { "+" } else { "" }, br(*x, 1)),
        _ => String::new(),
    };
    let low = match (&p.low_sem, &p.low_com) {
        (Some(a), Some(b)) => match (lado(a), lado(b)) {
            (Some(x), Some(y)) => format!("\n1% piores quadros: {} → {}", x, y),
            _ => String::new(),
        },
        _ => String::new(),
    };
    let real = |d: &Option<Diferenca>, sinal: f64| matches!(d, Some(Diferenca::Real { delta, .. }) if delta * sinal > 0.0);
    let resultado = match p.desfecho {
        // `decidir` dá "ganhou" com FPS OU 1% pior: a frase diz qual número passou da margem.
        Desfecho::Ganhou => match (real(&p.diferenca_fps, 1.0), real(&p.diferenca_low, 1.0)) {
            (true, true) => "ganho no FPS e no 1% piores, além da margem de erro (intervalos de 95% sem sobreposição).",
            (true, false) => "ganho no FPS, além da margem de erro (intervalos de 95% sem sobreposição); o 1% piores ficou dentro da margem.",
            _ => "ganho no 1% piores quadros, além da margem de erro (intervalos de 95% sem sobreposição); o FPS médio ficou dentro da margem.",
        },
        // Com o desfazer falho, o plano que piorou CONTINUA na máquina: a frase não pode dizer que saiu.
        Desfecho::Piorou if p.falha_ao_desfazer.is_some() => {
            "o plano OTIMIZA piorou este jogo neste PC, e desfazer falhou: ele ainda está ligado."
        }
        Desfecho::Piorou => "o plano OTIMIZA piorou este jogo neste PC e foi desfeito.",
        Desfecho::Indistinguivel => "a diferença ficou dentro da margem de erro: não dá para dizer que mudou.",
        Desfecho::SemComparacaoJusta => unreachable!("recusado acima"),
    };
    let lado_a = if p.contra_o_equilibrado { "Equilibrado do Windows" } else { "plano de energia de antes" };
    let rodape = match hardware {
        (Some(c), Some(g)) => format!("PC: {} · {} · {}", c.trim(), g.trim(), data),
        (Some(x), None) | (None, Some(x)) => format!("PC: {} · {}", x.trim(), data),
        (None, None) => data.to_string(),
    };
    Ok(format!(
        "Otimiza · prova no jogo — {jogo}\n\
         {lado_a} × plano OTIMIZA, {rodadas} rodadas de {seg} s alternadas com o jogo aberto.\n\
         FPS do jogo: {fps}{pct}{low}\n\
         Resultado: {resultado}\n\
         {rodape}\n\
         Medido com PresentMon. Outro PC pode ter outro resultado.",
        jogo = nome_do_jogo(&p.jogo),
        rodadas = p.rodadas.len(),
        seg = p.segundos_por_rodada,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resumo(media: f64, margem: Option<f64>) -> Resumo {
        Resumo { id: "fps".into(), n: 4, media, mediana: media, desvio: Some(1.0), margem }
    }

    fn prova(desfecho: Desfecho, margem: Option<f64>) -> Resultado {
        Resultado {
            jogo: "FiveM_b3258_GTAProcess.exe".into(),
            quando: 0,
            segundos_por_rodada: 45,
            rodadas: Vec::new(),
            fps_sem: Some(resumo(140.0, margem)),
            fps_com: Some(resumo(152.0, margem)),
            low_sem: Some(resumo(70.0, margem)),
            low_com: Some(resumo(76.0, margem)),
            diferenca_fps: Some(Diferenca::Real { delta: 12.0, pct: Some(8.57), folga: 4.0 }),
            diferenca_low: None,
            desfecho,
            leitura: String::new(),
            contra_o_equilibrado: false,
            ficou_com_otimiza: true,
            falha_ao_desfazer: None,
        }
    }

    #[test]
    fn prova_valida_sai_com_a_margem_e_o_ganho() {
        let t = texto(Some(&prova(Desfecho::Ganhou, Some(3.0))), (Some("Ryzen 5 5600"), Some("RTX 3060")), "26/09/2026").unwrap();
        assert!(t.contains("140 ± 3 → 152 ± 3 (+8,6%)"), "{t}");
        assert!(t.contains("1% piores quadros: 70 ± 3 → 76 ± 3"), "{t}");
        assert!(t.contains("FiveM") && t.contains("Ryzen 5 5600 · RTX 3060"), "{t}");
    }

    #[test]
    fn sem_diferenca_nao_vira_ganho() {
        let mut p = prova(Desfecho::Indistinguivel, Some(3.0));
        p.diferenca_fps = Some(Diferenca::Indistinguivel { delta: 1.0, sobreposicao: 5.0 });
        let t = texto(Some(&p), (None, None), "26/09/2026").unwrap();
        assert!(t.contains("não dá para dizer que mudou"), "{t}");
        assert!(!t.contains("ganho"), "{t}");
    }

    #[test]
    fn prova_injusta_sem_margem_ou_ausente_e_recusada() {
        assert_eq!(texto(None, (None, None), ""), Err(Recusa::SemProva));
        assert_eq!(texto(Some(&prova(Desfecho::SemComparacaoJusta, Some(3.0))), (None, None), ""), Err(Recusa::ProvaSemComparacaoJusta));
        assert_eq!(texto(Some(&prova(Desfecho::Ganhou, None)), (None, None), ""), Err(Recusa::SemMargem));
    }

    #[test]
    fn piora_tambem_se_compartilha_como_piora() {
        let t = texto(Some(&prova(Desfecho::Piorou, Some(3.0))), (None, None), "26/09/2026").unwrap();
        assert!(t.contains("piorou este jogo neste PC e foi desfeito"), "{t}");
    }

    #[test]
    fn desfazer_que_falhou_nao_sai_como_desfeito() {
        let mut p = prova(Desfecho::Piorou, Some(3.0));
        p.falha_ao_desfazer = Some("acesso negado".into());
        let t = texto(Some(&p), (None, None), "26/09/2026").unwrap();
        assert!(t.contains("ainda está ligado") && !t.contains("foi desfeito"), "{t}");
    }

    #[test]
    fn ganho_so_no_1_pct_nao_vira_ganho_de_fps() {
        let mut p = prova(Desfecho::Ganhou, Some(3.0));
        p.diferenca_fps = Some(Diferenca::Indistinguivel { delta: -1.0, sobreposicao: 5.0 });
        p.diferenca_low = Some(Diferenca::Real { delta: 6.0, pct: Some(8.0), folga: 1.0 });
        let t = texto(Some(&p), (None, None), "26/09/2026").unwrap();
        assert!(t.contains("ganho no 1% piores") && t.contains("FPS médio ficou dentro da margem"), "{t}");
    }

    #[test]
    fn a_margem_arredonda_para_cima() {
        assert_eq!(lado(&resumo(64.0, Some(6.2))).unwrap(), "64 ± 7");
        assert_eq!(lado(&resumo(64.0, Some(0.42))).unwrap(), "64 ± 0,5");
    }

    /// Laboratório: o texto da prova REAL desta máquina. `cargo test --release --lib compartilhar_real -- --ignored --nocapture`
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "laboratório: lê a prova guardada desta máquina"]
    fn compartilhar_real() {
        let p = crate::modules::provaalternada::guardada();
        println!("{}", texto(p.as_ref(), (Some("processador"), Some("placa")), "data").unwrap_or_else(|r| r.frase().to_string()));
    }
}
