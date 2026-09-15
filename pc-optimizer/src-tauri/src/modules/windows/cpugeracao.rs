// A geração do processador, e o que ela muda no plano de energia
//
// O pedido diz, com todas as letras: **"NÃO EXISTE UM ÚNICO CONJUNTO DE VALORES
// PERFEITO PARA TODOS OS PCs."** Está certo, e o exemplo concreto apareceu
// atendendo um cliente de verdade.
//
// O PC dele: i5-3470, de 2012. O Otimiza grava ali o EPP — a preferência entre
// energia e desempenho, que é o ajuste que comanda a frequência nos
// processadores modernos. Só que o EPP só é obedecido por processador com Speed
// Shift, que a Intel introduziu na 6ª geração. Naquela máquina, o EPP é escrito,
// conferido, e ignorado pelo silício.
//
// Isso não é um defeito: escrever não faz mal, e o Windows aceita o valor. O
// defeito seria o produto dizer que aquilo vai render quando não vai.
//
// ─────────────────────────────────────────────────────────────────────────
// A REGRA QUE SEPARA ESTE MÓDULO DE UM PALPITE
//
// **A geração NÃO decide nada. Ela só EXPLICA.**
//
// Quem decide é a leitura: `planoenergia::governa_o_processador` pergunta ao
// Windows se o processador está em modo autônomo, e essa resposta vem da
// máquina, não de uma tabela de modelos. A geração entra depois, para dizer ao
// cliente POR QUE a resposta é aquela.
//
// A distinção importa porque tabela de modelo envelhece e erra: processador
// novo com nome esquisito, engenharia de amostra, máquina virtual. Uma tabela
// que DECIDE erra calada; uma que EXPLICA, quando erra, produz no máximo um
// texto impreciso ao lado de um número que continua certo.
//
// ─────────────────────────────────────────────────────────────────────────
// O QUE É LIDO, E DE ONDE
//
// Do nome que o próprio processador declara — `Intel(R) Core(TM) i5-3470 CPU @
// 3.20GHz`. O esquema de numeração da Intel é público e estável há mais de dez
// anos: no `i5-3470`, o dígito antes dos três últimos é a geração.
//
// Conferido nesta máquina: `Intel(R) Core(TM) i3-10100F` → 10ª geração.
//
// Quando não dá para ler com confiança, a resposta é `None` — e `None` faz o
// produto não dizer nada sobre geração, em vez de chutar um número.

use serde::{Deserialize, Serialize};

use super::planoenergia::{FabricanteDaCpu, GovernoDoProcessador};

/// A geração Intel a partir da qual o EPP é obedecido.
///
/// Sexta — Skylake, 2015 —, quando a Intel introduziu o Speed Shift. Antes
/// dela, quem governa a frequência é o Windows pelos estados mínimo e máximo.
pub const PRIMEIRA_GERACAO_COM_SPEED_SHIFT: u32 = 6;

/// A geração de um processador Intel, pelo nome que ele declara.
///
/// **Função pura.** `None` quando não dá para ler com confiança — e `None` faz
/// o produto calar sobre geração, em vez de chutar.
///
/// O esquema: `i5-3470` é 3ª, `i7-8700K` é 8ª, `i3-10100F` é 10ª, `i5-12400` é
/// 12ª. O número do modelo tem 4 ou 5 dígitos; os três últimos identificam o
/// chip, e o que sobra na frente é a geração.
pub fn geracao_intel(nome: &str) -> Option<u32> {
    let baixo = nome.to_lowercase();

    // Só as linhas Core i. Pentium, Celeron, Xeon e Atom têm outros esquemas de
    // numeração, e aplicar este aqui produziria números inventados.
    let marcador = ["i3-", "i5-", "i7-", "i9-"]
        .iter()
        .find_map(|m| baixo.find(m).map(|i| i + m.len()))?;

    let digitos: String = baixo[marcador..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();

    // Quatro ou cinco dígitos. Menos que isso não é modelo; mais, não é o
    // esquema que este código conhece — e nos dois casos a resposta é não sei.
    if digitos.len() < 4 || digitos.len() > 5 {
        return None;
    }

    digitos[..digitos.len() - 3].parse::<u32>().ok()
}

/// A família Zen de um Ryzen, pelo nome.
///
/// **Função pura.** `Ryzen 5 3600` é Zen 2; `Ryzen 5 5600X` é Zen 3; `Ryzen 5
/// 7600` é Zen 4. O primeiro dígito do modelo dá a série, e a série mapeia para
/// a família — com uma exceção conhecida que este código NÃO tenta adivinhar:
/// a série 5000 tem modelos Zen 2 rebatizados em portáteis. Por isso o resultado
/// é a SÉRIE, e o texto fala em série, não em família.
pub fn serie_ryzen(nome: &str) -> Option<u32> {
    let baixo = nome.to_lowercase();
    let pos = baixo.find("ryzen")?;

    // Depois de "ryzen 5 " vem o modelo. Pula o dígito da linha (3, 5, 7, 9) e
    // pega o primeiro número de quatro dígitos.
    baixo[pos..]
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| p.len() == 4)
        .find_map(|p| p.parse::<u32>().ok())
        .map(|modelo| modelo / 1000)
}

/// O que a geração muda, para este cliente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum OQueGoverna {
    /// O EPP manda: processador recente escolhendo a própria frequência.
    OEpp { porque: String },
    /// O estado mínimo manda: o Windows escolhe.
    OEstadoMinimo { porque: String },
    /// Não deu para saber, e o produto aplica os dois sem prometer qual pesa.
    NaoDeuParaSaber { porque: String },
}

/// Junta a LEITURA com a geração.
///
/// **Função pura**, e a ordem dos argumentos conta a hierarquia: `governo` vem
/// primeiro porque é ele que decide. A geração só entra para explicar, e quando
/// as duas discordam, quem vale é a leitura — ela veio da máquina, a geração
/// veio de um nome.
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

/// A frase que a geração acrescenta, quando ela é legível.
///
/// Sempre começa com espaço e termina com ponto, para colar no fim das frases
/// acima sem o chamador precisar saber disso.
/// A FRASE PRECISA COMBINAR OS DOIS FATOS, e a primeira versão não combinava.
///
/// Rodando contra esta máquina — i3-10100F, com o Windows governando —, o texto
/// saía assim:
///
/// > "O Windows respondeu que é ELE quem escolhe a frequência. (…) Este é um
/// > Intel de 10ª geração, que TEM Speed Shift."
///
/// As duas metades estão certas e juntas soam como contradição. O que elas
/// significam é melhor que isso, e é o que o cliente precisa saber: **este
/// processador tem o modo bom e ele está desligado no plano de energia desta
/// máquina.** Isso é um achado, não um detalhe.
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

    /// O processador do cliente que motivou este módulo, e o desta máquina.

    /// O CASO DESTA MÁQUINA, e o que a primeira versão errava.
    ///
    /// i3-10100F com o Windows governando: as duas metades da frase estavam
    /// certas e juntas soavam como contradição — "o Windows escolhe" + "este
    /// processador tem Speed Shift". O que elas significam é melhor: o modo bom
    /// existe e está desligado.
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

    /// Pentium, Celeron, Xeon e Atom têm outros esquemas de numeração. Aplicar
    /// o esquema do Core i neles produziria um número inventado — e um número
    /// inventado numa frase sobre a máquina do cliente é exatamente o que este
    /// produto não faz.
    #[test]
    fn linha_que_nao_e_core_i_nao_vira_geracao() {
        assert_eq!(geracao_intel("Intel(R) Pentium(R) Gold G6400"), None);
        assert_eq!(geracao_intel("Intel(R) Celeron(R) N4020"), None);
        assert_eq!(geracao_intel("Intel(R) Xeon(R) E5-2680 v4"), None);
        assert_eq!(geracao_intel("AMD Ryzen 5 5600X"), None);
        assert_eq!(geracao_intel(""), None);
    }

    /// Modelo com contagem de dígitos fora do esquema conhecido é `None`, e não
    /// um palpite. É o mesmo princípio dos três estados do resto do produto.
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

    /// A REGRA CENTRAL DO MÓDULO: quem decide é a LEITURA, não a geração.
    ///
    /// Com um processador antigo e o Windows respondendo "o processador
    /// escolhe", vale a resposta do Windows — ela veio da máquina, a geração
    /// veio de um nome.
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

    /// E no caso do cliente — processador antigo, Windows no comando — a frase
    /// precisa DIZER que o EPP não tem a quem obedecer ali.
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

    /// Processador moderno: a frase muda, e não fala em ajuste inerte.
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

    /// Nome ilegível não acrescenta frase nenhuma — e não estraga a explicação
    /// da leitura, que continua inteira.
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

    /// E a lacuna não vira promessa: sem saber quem governa, o produto aplica
    /// os dois e DIZ que não sabe qual pesa.
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
