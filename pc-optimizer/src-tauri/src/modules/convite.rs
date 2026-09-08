// O convite do Discord, resolvido na hora do clique
//
// POR QUE ISTO EXISTE
//
// Convite do Discord vence. O que estava embutido no produto foi conferido em
// 29/08/2026 e vence em 28/09/2026 — e enquanto se acreditou que este programa
// não tinha camada de rede, isso era tratado como sem conserto: quem já tinha
// instalado ficaria com um link morto e sem nenhum caminho até o dono.
//
// A premissa estava errada. O produto TEM rede: `modules::atualizacao` pergunta
// ao GitHub se saiu versão nova desde antes desta versão. Então o convite pode
// ser consertado remotamente, e quem já instalou é justamente quem mais precisa
// disso.
//
// POR QUE NO CLIQUE, E NÃO NA ABERTURA
//
// Quem nunca pede suporte não paga requisição nenhuma, e a abertura do programa
// não fica um milissegundo mais lenta — a 1.7 gastou uma versão inteira
// derrubando esse tempo de 3,7 s para 1,2 s, e seria estranho devolver parte
// dele para buscar um endereço que a maioria nunca vai usar.
//
// Em troca, quem clica espera um instante. Por isso o prazo é curto e a reserva
// é imediata: no pior caso ele abre o convite embutido, que é exatamente o que
// aconteceria se este módulo não existisse.
//
// O QUE ELE NÃO FAZ
//
// Não manda nada. É um GET anônimo a um arquivo público do próprio repositório,
// com o mesmo User-Agent da consulta de versão. Nenhum dado da máquina viaja.

use serde::Deserialize;

/// O arquivo público que carrega o convite atual.
///
/// Fica no mesmo repositório que o instalador, e é editável sem publicar versão
/// nova — que é o ponto: trocar o convite não pode exigir que o cliente
/// atualize o programa.
const ENDERECO: &str =
    "https://raw.githubusercontent.com/Otimiza-Oficial/Otimiza/main/.github/convite.json";

/// Prazo curto porque há alguém olhando a tela, com o dedo no botão.
///
/// A consulta de versão pode esperar dez segundos porque acontece sozinha, em
/// segundo plano. Esta acontece depois de um clique, e um clique que demora
/// dois segundos já parece travado.
const TIMEOUT_SEGUNDOS: u64 = 3;

#[derive(Debug, Deserialize)]
struct Publicado {
    discord: Option<String>,
}

/// O endereço parece um convite do Discord?
///
/// ESTA CONFERÊNCIA NÃO É PARANOIA. O valor vem da rede, e é usado para abrir
/// uma janela do navegador do cliente. Sem validar a forma, qualquer conteúdo
/// que chegasse neste arquivo — por engano de edição, ou por um dia ruim no
/// GitHub — viraria um endereço que o produto abre em nome dele.
///
/// A regra é estreita de propósito: só `https://discord.gg/<código>`. Não
/// aceita `http`, não aceita outro domínio, e não aceita caminho a mais.
///
/// Função pura para poder ser testada sem rede.
pub fn parece_convite(endereco: &str) -> bool {
    let Some(codigo) = endereco.strip_prefix("https://discord.gg/") else {
        return false;
    };

    !codigo.is_empty()
        && codigo.len() <= 32
        && codigo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Extrai o convite de um JSON publicado, se ele for válido.
///
/// Separada da rede para o teste poder cobrir o que interessa: JSON quebrado,
/// campo ausente, e valor que não é convite.
pub fn ler_publicado(json: &str) -> Option<String> {
    let publicado: Publicado = serde_json::from_str(json).ok()?;
    let endereco = publicado.discord?;

    parece_convite(&endereco).then_some(endereco)
}

/// Pergunta ao repositório qual é o convite de hoje.
///
/// `None` em qualquer tropeço — sem rede, arquivo fora do ar, JSON estranho,
/// endereço que não parece convite. Quem chama tem o embutido para usar, e
/// insistir aqui só faria o cliente esperar mais para chegar ao mesmo lugar.
pub async fn consultar() -> Option<String> {
    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SEGUNDOS))
        // Mesmo User-Agent da consulta de versão: é o único dado que sai daqui,
        // e ele diz o programa e a versão, nada da máquina.
        .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;

    let resposta = cliente.get(ENDERECO).send().await.ok()?;

    if !resposta.status().is_success() {
        return None;
    }

    ler_publicado(&resposta.text().await.ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aceita_um_convite_de_verdade() {
        assert!(parece_convite("https://discord.gg/fmeQVJphC"));
        assert!(parece_convite("https://discord.gg/otimiza-oficial"));
        assert!(parece_convite("https://discord.gg/a_b-C9"));
    }

    /// O valor vem da rede e abre uma janela no navegador do cliente. Tudo que
    /// não for exatamente um convite precisa ser recusado — a reserva embutida
    /// é sempre melhor que um endereço estranho.
    #[test]
    fn recusa_qualquer_coisa_que_nao_seja_convite() {
        for estranho in [
            "http://discord.gg/abc",              // sem TLS
            "https://discord.gg.exemplo.com/abc", // domínio parecido
            "https://exemplo.com/discord.gg/abc",
            "https://discord.gg/",                // sem código
            "https://discord.gg/abc/def",         // caminho a mais
            "https://discord.gg/abc?x=1",         // consulta pendurada
            "javascript:alert(1)",
            "",
        ] {
            assert!(
                !parece_convite(estranho),
                "{:?} nao pode ser aceito como convite",
                estranho
            );
        }
    }

    #[test]
    fn le_o_convite_do_json_publicado() {
        let json = r#"{"discord": "https://discord.gg/otimiza"}"#;

        assert_eq!(
            ler_publicado(json),
            Some("https://discord.gg/otimiza".to_string())
        );
    }

    /// Cada jeito de o arquivo estar errado devolve `None`, e quem chama cai na
    /// reserva embutida — que é o comportamento que o produto tinha antes.
    #[test]
    fn json_estranho_cai_na_reserva() {
        for ruim in [
            "",
            "não é json",
            "{}",
            r#"{"discord": null}"#,
            r#"{"discord": ""}"#,
            r#"{"discord": "https://exemplo.com/entrar"}"#,
            r#"{"outro_campo": "https://discord.gg/abc"}"#,
        ] {
            assert_eq!(ler_publicado(ruim), None, "{:?} deveria cair na reserva", ruim);
        }
    }
}
