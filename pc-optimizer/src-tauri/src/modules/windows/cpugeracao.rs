// A geração do processador e o que ela muda na energia: o EPP só é obedecido com Speed Shift (Intel a partir da
// 6ª geração). A geração NÃO decide nada, só EXPLICA: quem decide é a leitura
// (`planoenergia::governa_o_processador`), porque tabela de modelo envelhece e erra. Nome ilegível dá `None`, e o
// produto não diz nada sobre geração.

use serde::{Deserialize, Serialize};

use super::planoenergia::{FabricanteDaCpu, GovernoDoProcessador};

/// Skylake (2015), quando a Intel introduziu o Speed Shift.
pub const PRIMEIRA_GERACAO_COM_SPEED_SHIFT: u32 = 6;

/// `i5-3470` é 3ª, `i3-10100F` é 10ª: os três últimos dígitos são o chip, o resto é a geração.
pub fn geracao_intel(nome: &str) -> Option<u32> {
    let baixo = nome.to_lowercase();

    // Só Core i: Pentium, Celeron, Xeon e Atom têm outros esquemas, e este inventaria números.
    let marcador = ["i3-", "i5-", "i7-", "i9-"]
        .iter()
        .find_map(|m| baixo.find(m).map(|i| i + m.len()))?;

    let digitos: String = baixo[marcador..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();

    // Fora de 4 ou 5 dígitos é "não sei".
    if digitos.len() < 4 || digitos.len() > 5 {
        return None;
    }

    digitos[..digitos.len() - 3].parse::<u32>().ok()
}

/// Devolve a SÉRIE, não a família: a série 5000 tem Zen 2 rebatizados em portáteis.
pub fn serie_ryzen(nome: &str) -> Option<u32> {
    let baixo = nome.to_lowercase();
    let pos = baixo.find("ryzen")?;

    baixo[pos..]
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| p.len() == 4)
        .find_map(|p| p.parse::<u32>().ok())
        .map(|modelo| modelo / 1000)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum OQueGoverna {
    OEpp { porque: String },
    OEstadoMinimo { porque: String },
    /// Aplica os dois sem prometer qual pesa.
    NaoDeuParaSaber { porque: String },
}

/// `governo` decide; a geração só explica, e quando discordam vale a leitura.
pub fn o_que_governa(
    governo: GovernoDoProcessador,
    fabricante: FabricanteDaCpu,
    nome_da_cpu: &str,
) -> OQueGoverna {
    let detalhe = detalhe_da_geracao(governo, fabricante, nome_da_cpu);

    match governo {
        GovernoDoProcessador::OProcessador => OQueGoverna::OEpp {
            porque: format!(
                "O Windows respondeu que o processador escolhe a própria frequência nesta \
                 máquina. Nesse modo, o ajuste que manda é a preferência entre energia e \
                 desempenho (EPP), e o estado mínimo do processador quase não muda nada.{detalhe}"
            ),
        },
        GovernoDoProcessador::OWindows => OQueGoverna::OEstadoMinimo {
            porque: format!(
                "O Windows respondeu que é ELE quem escolhe a frequência nesta máquina. Aqui \
                 o estado mínimo do processador governa de verdade, e é ele que evita a \
                 frequência cair entre um quadro e outro.{detalhe}"
            ),
        },
        GovernoDoProcessador::NaoDeuParaLer => OQueGoverna::NaoDeuParaSaber {
            porque: format!(
                "Não deu para ler quem escolhe a frequência nesta máquina. Os dois ajustes \
                 são aplicados assim mesmo — nenhum faz mal no caso do outro —, e o Otimiza \
                 não vai prometer qual deles está pesando aqui.{detalhe}"
            ),
        },
    }
}

/// Combina os dois fatos: "o Windows escolhe" + "tem Speed Shift" soavam contradição; o certo é "o modo bom
/// existe e está desligado no plano desta máquina", que é um achado.
fn detalhe_da_geracao(
    governo: GovernoDoProcessador,
    fabricante: FabricanteDaCpu,
    nome: &str,
) -> String {
    let windows_no_comando = matches!(governo, GovernoDoProcessador::OWindows);

    match fabricante {
        FabricanteDaCpu::Intel => match geracao_intel(nome) {
            Some(g) if g >= PRIMEIRA_GERACAO_COM_SPEED_SHIFT && windows_no_comando => format!(
                " Vale saber: este é um Intel de {g}ª geração e TEM Speed Shift — o modo em \
                 que o próprio processador escolhe a frequência, que costuma responder mais \
                 rápido. Ele está desligado no plano de energia desta máquina. O Otimiza não \
                 liga isso sozinho: trocar quem comanda o processador por cima do que o \
                 fabricante entregou é decisão de quem usa, não efeito de um clique."
            ),
            Some(g) if g >= PRIMEIRA_GERACAO_COM_SPEED_SHIFT => format!(
                " Este é um Intel de {g}ª geração, que tem Speed Shift — e ele está ativo \
                 aqui, que é o modo em que a preferência de energia manda."
            ),
            Some(g) => format!(
                " Este é um Intel de {g}ª geração. O Speed Shift chegou na 6ª, então neste \
                 processador a preferência de energia (EPP) é gravada e conferida, mas não \
                 tem a quem obedecer — quem governa aqui é o estado mínimo."
            ),
            None => String::new(),
        },
        FabricanteDaCpu::Amd => match serie_ryzen(nome) {
            Some(s) => format!(
                " Este é um Ryzen da série {s}000. Nos Ryzen recentes o próprio processador \
                 negocia a frequência com o Windows, pelo CPPC."
            ),
            None => String::new(),
        },
        FabricanteDaCpu::Outro => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processador_moderno_com_o_windows_no_comando_e_um_achado() {
        let OQueGoverna::OEstadoMinimo { porque } = o_que_governa(
            GovernoDoProcessador::OWindows,
            FabricanteDaCpu::Intel,
            "Intel(R) Core(TM) i3-10100F CPU @ 3.60GHz",
        ) else {
            panic!("era o estado mínimo");
        };

        assert!(porque.contains("10ª geração"));
        assert!(
            porque.contains("está desligado"),
            "o cliente precisa saber que o modo bom existe e não está em uso: {porque}"
        );
        assert!(
            porque.contains("não liga isso sozinho"),
            "e precisa saber que o Otimiza não vai mexer nisso por conta própria"
        );
    }

    #[test]
    fn le_a_geracao_dos_processadores_de_verdade() {
        assert_eq!(
            geracao_intel("Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz"),
            Some(3),
            "o i5-3470 do cliente é de 3ª geração"
        );
        assert_eq!(
            geracao_intel("Intel(R) Core(TM) i3-10100F CPU @ 3.60GHz"),
            Some(10),
            "o desta máquina é de 10ª"
        );
    }

    #[test]
    fn le_as_geracoes_de_quatro_e_cinco_digitos() {
        assert_eq!(geracao_intel("Intel Core i7-8700K"), Some(8));
        assert_eq!(geracao_intel("Intel Core i5-12400F"), Some(12));
        assert_eq!(geracao_intel("Intel(R) Core(TM) i9-14900K"), Some(14));
        assert_eq!(geracao_intel("intel core i3-4130"), Some(4));
    }

    #[test]
    fn linha_que_nao_e_core_i_nao_vira_geracao() {
        assert_eq!(geracao_intel("Intel(R) Pentium(R) Gold G6400"), None);
        assert_eq!(geracao_intel("Intel(R) Celeron(R) N4020"), None);
        assert_eq!(geracao_intel("Intel(R) Xeon(R) E5-2680 v4"), None);
        assert_eq!(geracao_intel("AMD Ryzen 5 5600X"), None);
        assert_eq!(geracao_intel(""), None);
    }

    #[test]
    fn modelo_fora_do_esquema_conhecido_e_nao_sei() {
        assert_eq!(geracao_intel("Intel Core i5-999"), None);
        assert_eq!(geracao_intel("Intel Core i5-1234567"), None);
        assert_eq!(geracao_intel("Intel Core i5-"), None);
    }

    #[test]
    fn le_a_serie_do_ryzen() {
        assert_eq!(serie_ryzen("AMD Ryzen 5 3600 6-Core Processor"), Some(3));
        assert_eq!(serie_ryzen("AMD Ryzen 7 5800X3D 8-Core Processor"), Some(5));
        assert_eq!(serie_ryzen("AMD Ryzen 5 7600 6-Core Processor"), Some(7));
        assert_eq!(serie_ryzen("Intel Core i5-12400"), None);
    }

    #[test]
    fn a_leitura_manda_e_a_geracao_so_explica() {
        let r = o_que_governa(
            GovernoDoProcessador::OProcessador,
            FabricanteDaCpu::Intel,
            "Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz",
        );

        assert!(
            matches!(r, OQueGoverna::OEpp { .. }),
            "a leitura disse que o processador governa; a geração não pode derrubar isso"
        );
    }

    #[test]
    fn o_processador_antigo_e_explicado_ao_cliente() {
        let OQueGoverna::OEstadoMinimo { porque } = o_que_governa(
            GovernoDoProcessador::OWindows,
            FabricanteDaCpu::Intel,
            "Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz",
        ) else {
            panic!("era para o estado mínimo governar");
        };

        assert!(porque.contains("3ª geração"));
        assert!(porque.contains("Speed Shift chegou na 6ª"));
        assert!(
            porque.contains("não tem a quem obedecer"),
            "o cliente precisa saber que aquele ajuste não rende NELE"
        );
    }

    #[test]
    fn o_processador_moderno_ganha_outra_frase() {
        let OQueGoverna::OEpp { porque } = o_que_governa(
            GovernoDoProcessador::OProcessador,
            FabricanteDaCpu::Intel,
            "Intel(R) Core(TM) i5-12400F",
        ) else {
            panic!("era o EPP");
        };

        assert!(porque.contains("12ª geração"));
        assert!(porque.contains("Speed Shift"));
        assert!(!porque.contains("não tem a quem obedecer"));
    }

    #[test]
    fn nome_ilegivel_nao_acrescenta_palpite() {
        let r = o_que_governa(
            GovernoDoProcessador::OWindows,
            FabricanteDaCpu::Outro,
            "Processador desconhecido",
        );

        let OQueGoverna::OEstadoMinimo { porque } = r else {
            panic!("era o estado mínimo");
        };

        assert!(porque.contains("é ELE quem escolhe"), "a leitura continua explicada");
        assert!(!porque.contains("geração"), "não inventou geração");
    }

    #[test]
    fn sem_saber_quem_governa_o_produto_nao_promete() {
        let OQueGoverna::NaoDeuParaSaber { porque } = o_que_governa(
            GovernoDoProcessador::NaoDeuParaLer,
            FabricanteDaCpu::Amd,
            "AMD Ryzen 5 5600X",
        ) else {
            panic!("era a lacuna");
        };

        assert!(porque.contains("não vai prometer"));
        assert!(porque.contains("série 5000"), "a geração continua explicando");
    }
}
