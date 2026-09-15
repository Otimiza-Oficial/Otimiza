// A nota de jogo desta máquina
//
// O PEDIDO ERA CLARO E É O CONTRÁRIO DO QUE O MERCADO FAZ: a nota tem que pesar
// mais o **1% pior** do que a média. E está certo.
//
// FPS médio é o número que aparece no anúncio porque é o maior. Só que ninguém
// sente média: a pessoa sente a travada. Uma máquina a 120 de média com o 1%
// pior em 35 joga PIOR que uma a 90 de média com o 1% pior em 70 — a primeira
// engasga na curva, a segunda é lisa. Quem joga sabe disso na mão; quem vende
// otimizador finge que não.
//
// Então o peso é 60% no 1% pior e 40% na média. Não é meio a meio: meio a meio
// deixaria a média disfarçar o engasgo, que é exatamente o que este produto
// existe para não fazer.
//
// ─────────────────────────────────────────────────────────────────────────
// O QUE ESTA NOTA **NÃO** É
//
// Não é comparação com outras máquinas. Não existe um banco de dados de "PCs
// como o seu" aqui, e inventar um seria o mesmo que inventar número. A nota
// descreve ESTA máquina neste jogo, e serve para uma coisa só: comparar o antes
// com o depois na mesma máquina.
//
// Não é nota de hardware. Um PC excelente num servidor de RP lotado tira nota
// baixa, e está certo — a nota é da EXPERIÊNCIA, não da peça.
//
// E ela não sai sem amostra confiável. `MedicaoAutomatica::confiavel` é falso
// quando a janela foi curta demais para o 1% pior significar alguma coisa, e
// uma nota calculada em cima disso seria um número bonito sem lastro.

use serde::{Deserialize, Serialize};

/// O peso do 1% pior. Ver o cabeçalho: não é meio a meio de propósito.
pub const PESO_DO_UM_POR_CENTO: f64 = 0.6;

/// FPS a partir do qual cada metade vale nota cheia.
///
/// Cento e quarenta para a média e setenta para o 1% pior. Os dois números são
/// a MESMA exigência dita de dois jeitos: num jogo saudável o 1% pior fica em
/// torno da metade da média, então quem alcança 140 de média com 70 de 1% pior
/// está com o quadro estável. Acima disso a nota satura — porque acima disso a
/// diferença deixa de ser sentida, e continuar somando ponto seria transformar
/// a nota num placar de quem tem a placa mais cara.
pub const MEDIA_PARA_NOTA_CHEIA: f64 = 140.0;
pub const UM_POR_CENTO_PARA_NOTA_CHEIA: f64 = 70.0;

/// A nota, e de que ela é feita.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum Nota {
    /// De 0 a 100.
    Calculada {
        nota: u32,
        fps_medio: f64,
        low_1pct: f64,
        /// A parte da nota que veio de cada metade, para a tela poder mostrar
        /// POR QUE a nota é essa em vez de só estampar um número.
        pontos_do_um_por_cento: f64,
        pontos_da_media: f64,
    },
    /// Sem amostra confiável. NÃO é nota zero: zero seria uma afirmação sobre
    /// uma máquina que não foi medida.
    SemAmostra,
}

/// Quanto um valor medido rende, de 0 a 1, saturando no alvo.
///
/// **Função pura.** Linear até o alvo e depois constante. Linear porque é o que
/// a pessoa consegue conferir na cabeça — "dobrei o FPS, dobrei os pontos" —, e
/// uma curva esperta aqui só serviria para a nota subir bonito num gráfico.
fn fracao(medido: f64, alvo: f64) -> f64 {
    if alvo <= 0.0 || medido <= 0.0 {
        return 0.0;
    }

    (medido / alvo).min(1.0)
}

/// A nota desta medição.
///
/// **Função pura**, e é onde a regra do peso vive.
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

/// A frase que acompanha a nota.
///
/// Regra de produto, e por isso mora aqui com teste. A tela recebe pronta.
///
/// Ela NUNCA compara com outras máquinas, e nunca diz "ruim": diz o que está
/// segurando a experiência, que é acionável.
pub fn explicar(nota: &Nota) -> String {
    let Nota::Calculada { nota, fps_medio, low_1pct, .. } = nota else {
        return "Ainda não houve medição longa o bastante para uma nota. Ela aparece \
                depois de alguns minutos de jogo — antes disso o 1% pior não significa \
                nada, e uma nota sem lastro seria só um número bonito."
            .to_string();
    };

    // O 1% pior perto da metade da média é o comportamento saudável. Bem abaixo
    // disso é engasgo, e é o que a pessoa sente.
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

    /// A REGRA CENTRAL, com os dois casos do cabeçalho. A máquina de 90 com o
    /// quadro liso tem que tirar MAIS que a de 120 que engasga — senão a nota
    /// está medindo o anúncio e não o jogo.
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

    /// Sem amostra confiável não há nota. Zero seria uma afirmação sobre uma
    /// máquina que não foi medida.
    #[test]
    fn sem_amostra_confiavel_nao_ha_nota() {
        assert_eq!(calcular(200.0, 150.0, false), Nota::SemAmostra);
        assert_eq!(calcular(0.0, 0.0, true), Nota::SemAmostra);
    }

    /// Acima do alvo a nota satura. Sem isso, um PC de doze mil reais tiraria
    /// 300 e a nota viraria placar de quem tem a placa mais cara.
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

    /// Os números do cliente que motivou este trabalho: 15 a 20 FPS no FiveM.
    /// A nota precisa ser baixa, mas o diagnóstico precisa apontar o FPS e não
    /// o engasgo — com 15 de média e 9 no 1% pior, a proporção é saudável e o
    /// problema é a máquina não dar conta, não a travada.
    #[test]
    fn a_maquina_lenta_mas_estavel_nao_e_acusada_de_engasgo() {
        let frase = explicar(&calcular(18.0, 12.0, true));

        assert!(!frase.contains("é o engasgo"), "acusou engasgo onde não há: {frase}");
        assert!(frase.contains("o que segura a nota é o FPS"));
    }
}
