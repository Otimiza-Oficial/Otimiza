// A nota de jogo desta máquina: 60% no 1% pior, 40% na média, porque ninguém sente média, sente travada (120
// de média com 35 no 1% pior joga pior que 90 com 70). Não compara com outras máquinas nem avalia peça: serve
// para o antes e depois nesta máquina. Sem amostra confiável, não sai.

use serde::{Deserialize, Serialize};

pub const PESO_DO_UM_POR_CENTO: f64 = 0.6;

/// 140 de média e 70 no 1% pior são a mesma exigência (o 1% pior saudável fica em torno da metade da média).
/// Acima disso a nota satura: a diferença deixa de ser sentida.
pub const MEDIA_PARA_NOTA_CHEIA: f64 = 140.0;
pub const UM_POR_CENTO_PARA_NOTA_CHEIA: f64 = 70.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum Nota {
    Calculada {
        nota: u32,
        fps_medio: f64,
        low_1pct: f64,
        /// Para a tela mostrar POR QUE a nota é essa.
        pontos_do_um_por_cento: f64,
        pontos_da_media: f64,
    },
    /// NÃO é nota zero: zero seria afirmar algo sobre uma máquina que não foi medida.
    SemAmostra,
}

/// Linear até o alvo, para a pessoa conferir de cabeça; curva esperta só faria a nota subir bonito.
fn fracao(medido: f64, alvo: f64) -> f64 {
    if alvo <= 0.0 || medido <= 0.0 {
        return 0.0;
    }

    (medido / alvo).min(1.0)
}

pub fn calcular(fps_medio: f64, low_1pct: f64, confiavel: bool) -> Nota {
    if !confiavel || fps_medio <= 0.0 {
        return Nota::SemAmostra;
    }

    let pontos_do_um_por_cento =
        fracao(low_1pct, UM_POR_CENTO_PARA_NOTA_CHEIA) * PESO_DO_UM_POR_CENTO * 100.0;

    let pontos_da_media =
        fracao(fps_medio, MEDIA_PARA_NOTA_CHEIA) * (1.0 - PESO_DO_UM_POR_CENTO) * 100.0;

    Nota::Calculada {
        nota: (pontos_do_um_por_cento + pontos_da_media).round() as u32,
        fps_medio,
        low_1pct,
        pontos_do_um_por_cento,
        pontos_da_media,
    }
}

/// Nunca compara com outras máquinas nem diz "ruim": diz o que está segurando a experiência.
pub fn explicar(nota: &Nota) -> String {
    let Nota::Calculada { nota, fps_medio, low_1pct, .. } = nota else {
        return "Ainda não houve medição longa o bastante para uma nota. Ela aparece \
                depois de alguns minutos de jogo — antes disso o 1% pior não significa \
                nada, e uma nota sem lastro seria só um número bonito."
            .to_string();
    };

    let proporcao = if *fps_medio > 0.0 { low_1pct / fps_medio } else { 0.0 };

    let diagnostico = if proporcao < 0.4 {
        "O que segura esta nota não é o FPS médio, é o engasgo: os piores quadros estão \
         muito abaixo da média, e é isso que aparece como travada no meio do jogo."
    } else if *fps_medio < 60.0 {
        "O quadro está estável, e o que segura a nota é o FPS em si — aqui o limite é a \
         máquina ou o quanto o jogo está pedindo dela."
    } else {
        "Quadro estável e FPS folgado: esta máquina está entregando o que tem."
    };

    format!(
        "Nota {nota} de 100, com {fps_medio:.0} FPS de média e {low_1pct:.0} no 1% pior. \
         O 1% pior pesa mais que a média nesta conta, porque é ele que a pessoa sente. \
         {diagnostico} A nota descreve esta máquina neste jogo, e serve para comparar \
         antes e depois — não para comparar com o PC de outra pessoa."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nota_de(fps: f64, low: f64) -> u32 {
        match calcular(fps, low, true) {
            Nota::Calculada { nota, .. } => nota,
            Nota::SemAmostra => panic!("era para ter calculado"),
        }
    }

    /// A de 90 com quadro liso tem que tirar MAIS que a de 120 que engasga.
    #[test]
    fn quadro_liso_a_90_vale_mais_que_engasgado_a_120() {
        let engasgada = nota_de(120.0, 35.0);
        let lisa = nota_de(90.0, 70.0);

        assert!(
            lisa > engasgada,
            "a nota lisa ({lisa}) precisa ganhar da engasgada ({engasgada})"
        );
    }

    #[test]
    fn o_um_por_cento_pesa_mais_que_a_media() {
        let Nota::Calculada { pontos_do_um_por_cento, pontos_da_media, .. } =
            calcular(140.0, 70.0, true)
        else {
            panic!("era para ter calculado");
        };

        assert!(
            pontos_do_um_por_cento > pontos_da_media,
            "o peso do 1% pior foi invertido"
        );
        assert_eq!(nota_de(140.0, 70.0), 100, "os dois alvos batidos são nota cheia");
    }

    #[test]
    fn sem_amostra_confiavel_nao_ha_nota() {
        assert_eq!(calcular(200.0, 150.0, false), Nota::SemAmostra);
        assert_eq!(calcular(0.0, 0.0, true), Nota::SemAmostra);
    }

    #[test]
    fn acima_do_alvo_a_nota_nao_passa_de_cem() {
        assert_eq!(nota_de(600.0, 400.0), 100);
    }

    #[test]
    fn a_explicacao_aponta_o_engasgo_quando_e_engasgo() {
        let frase = explicar(&calcular(120.0, 30.0, true));

        assert!(frase.contains("engasgo"), "o diagnóstico errou o alvo: {frase}");
        assert!(frase.contains("120") && frase.contains("30"));
    }

    #[test]
    fn a_explicacao_nao_compara_com_outras_maquinas() {
        let frase = explicar(&calcular(90.0, 60.0, true));

        assert!(
            frase.contains("não para comparar com o PC de outra pessoa"),
            "a nota precisa dizer o que ela NÃO é"
        );
    }

    #[test]
    fn sem_nota_a_frase_explica_por_que_e_nao_acusa_a_maquina() {
        let frase = explicar(&Nota::SemAmostra);

        assert!(frase.contains("Ainda não houve medição"));
        assert!(!frase.contains("ruim"));
    }

    /// O cliente que motivou o trabalho (15 a 20 FPS no FiveM): nota baixa, mas o problema apontado é o FPS, não o
    /// engasgo.
    #[test]
    fn a_maquina_lenta_mas_estavel_nao_e_acusada_de_engasgo() {
        let frase = explicar(&calcular(18.0, 12.0, true));

        assert!(!frase.contains("é o engasgo"), "acusou engasgo onde não há: {frase}");
        assert!(frase.contains("o que segura a nota é o FPS"));
    }
}
