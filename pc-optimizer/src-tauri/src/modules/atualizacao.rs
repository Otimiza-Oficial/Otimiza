// O aviso de versão nova dentro do programa.
//
// Até agora, quem descobria que existia uma versão nova era só quem estava no
// Discord do dono: um bot manda mensagem direta. A partir de agora o produto
// também é vendido pelo WhatsApp, fora do Discord inteiramente — e para esse
// cliente o Otimiza é um programa que NUNCA se atualiza. Ele fica parado na
// versão que comprou, para sempre, inclusive quando ela tem um defeito que já
// foi corrigido. Foi o que aconteceu na semana passada: um cliente relatou um
// problema que já tinha sido consertado numa versão que ele não sabia existir.
//
// ESTE MÓDULO PERGUNTA AO GITHUB. NÃO É AVISADO POR NINGUÉM.
//
// A mesma decisão do bot em `avisoDeVersao.js`: aceitar aviso de fora exigiria
// que alguém ALCANÇASSE o programa pela internet, e o Otimiza roda no PC do
// cliente sem servidor e sem porta aberta. Perguntar também sobrevive ao PC
// desligado — a próxima vez que o programa abrir, a pergunta acontece de novo.
//
// A COMPARAÇÃO É PURA E TESTÁVEL; A CONSULTA AO GITHUB É SEPARADA.
//
// É essa separação que permite testar a regra de comparação — equanto conta
// como "versão maior", o prefixo `v`, 1.10 sendo maior que 1.9 — sem depender
// de rede nenhuma. Misturar as duas coisas numa função só faria o teste
// precisar de internet, ou de um servidor falso, para provar uma regra que não
// tem nada a ver com rede.

use serde::Serialize;

/// O repositório que o instalador é publicado — o mesmo que o link fixo do
/// bot usa (ver `.github/workflows/release.yml`).
const REPOSITORIO: &str = "Otimiza-Oficial/Otimiza";

/// Quanto este programa espera antes de desistir da consulta.
///
/// Curto de propósito: esta pergunta acontece na abertura do programa, junto
/// com o resto do carregamento da tela, e uma internet ruim não pode prender
/// o cliente esperando um aviso que ele nem pediu.
const TIMEOUT_SEGUNDOS: u64 = 10;

/// O resultado de comparar a versão instalada com a publicada.
///
/// TRÊS VARIANTES, E NÃO UM BOOL, PORQUE "NÃO SEI" NÃO É "NÃO HÁ VERSÃO NOVA".
///
/// Uma consulta que falhou, ou que voltou algo ilegível, não pode virar
/// `NaoHaNova` — isso é o produto fingindo que perguntou e recebeu "está tudo
/// em dia" quando na verdade não perguntou nada de verdade. `NaoSei` é o
/// resultado honesto: a tela não mostra aviso nenhum, mas também não afirma
/// que está atualizado.
///
/// A variante se chamava `Igual`, e o nome mentia sobre um caso real: uma
/// instalada MAIOR que a publicada — o que acontece toda vez que o dono roda
/// a versão de desenvolvimento — também cai aqui, e não é igualdade nenhuma.
/// Nada quebrava por causa disso hoje, porque só `HaVersaoNova` faz a tela
/// agir; mas um nome que afirma o que a função não verificou é exatamente o
/// tipo de rótulo que engana quem for reusar a função depois.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Comparacao {
    /// Não há versão mais nova para oferecer: a instalada é igual ou maior.
    NaoHaNova,
    HaVersaoNova,
    NaoSei,
}

/// Quebra uma versão em números, para comparar por VALOR e não por texto.
///
/// Aceita o prefixo `v` (as tags do repositório são `v1.3.0`) e ignora espaço
/// nas pontas. Uma versão vazia, ou com um pedaço que não é número, devolve
/// `None` — é o "ilegível" que vira `Comparacao::NaoSei`.
fn normalizar(versao: &str) -> Option<Vec<u32>> {
    let sem_prefixo = versao.trim().trim_start_matches(['v', 'V']);

    if sem_prefixo.is_empty() {
        return None;
    }

    let partes: Vec<u32> = sem_prefixo
        .split('.')
        .map(|pedaco| pedaco.trim().parse::<u32>())
        .collect::<Result<_, _>>()
        .ok()?;

    if partes.is_empty() {
        None
    } else {
        Some(partes)
    }
}

/// Compara duas versões PELO VALOR de cada número, não pelo texto.
///
/// POR QUE NÃO DÁ PARA COMPARAR COMO TEXTO
///
/// "1.10.0" é MAIOR que "1.9.0", mas como texto "1.10.0" vem ANTES de
/// "1.9.0" — o caractere `1` é menor que `9` na segunda posição, e a
/// comparação de string para no primeiro caractere que difere sem nunca olhar
/// o resto do número. Comparar como texto faria o cliente parar de ser
/// avisado bem quando o produto passa da versão 1.9 — justamente quando ele
/// mais amadureceu.
///
/// Uma versão instalada MAIOR que a publicada não é aviso, e não é erro:
/// acontece em toda compilação local, que sai com a versão do `Cargo.toml`
/// antes de qualquer release existir.
pub fn comparar(instalada: &str, publicada: &str) -> Comparacao {
    let (Some(mut numeros_instalada), Some(mut numeros_publicada)) =
        (normalizar(instalada), normalizar(publicada))
    else {
        return Comparacao::NaoSei;
    };

    // Versões com números diferentes de partes ("1.5" contra "1.5.0") são
    // completadas com zero à direita antes de comparar, para que a ausência
    // de um terceiro número não seja lida como "menor".
    let tamanho = numeros_instalada.len().max(numeros_publicada.len());
    numeros_instalada.resize(tamanho, 0);
    numeros_publicada.resize(tamanho, 0);

    if numeros_publicada > numeros_instalada {
        Comparacao::HaVersaoNova
    } else {
        Comparacao::NaoHaNova
    }
}

/// O que o GitHub respondeu sobre a versão mais nova publicada.
#[derive(Debug, Clone, Serialize)]
pub struct UltimaVersao {
    pub versao: String,
    pub pagina: Option<String>,
}

/// Pergunta ao GitHub qual é a versão mais nova publicada.
///
/// `None` quando a consulta falha — e falha de rede NÃO é "não há versão
/// nova": é silêncio. Uma internet instável, ou o GitHub fora do ar por um
/// minuto, não pode virar "está tudo em dia" para o cliente.
///
/// Consulta anônima, sem chave e sem servidor nosso no meio — o mesmo
/// endereço e o mesmo raciocínio do bot em `avisoDeVersao.js`: o programa
/// PERGUNTA, e não é avisado.
pub async fn consultar_ultima() -> Option<UltimaVersao> {
    let url = format!("https://api.github.com/repos/{REPOSITORIO}/releases/latest");

    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SEGUNDOS))
        // O GitHub exige User-Agent em toda chamada da API — sem ele a
        // resposta é 403, e um 403 sem explicação pareceria um bug nosso.
        .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;

    let resposta = cliente
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;

    if !resposta.status().is_success() {
        return None;
    }

    let corpo: serde_json::Value = resposta.json().await.ok()?;
    let versao = corpo.get("tag_name").and_then(|v| v.as_str())?.trim();

    if versao.is_empty() {
        return None;
    }

    Some(UltimaVersao {
        versao: versao.to_string(),
        pagina: corpo
            .get("html_url")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_avisa_quando_a_publicada_e_maior() {
        assert_eq!(comparar("1.5.0", "1.5.0"), Comparacao::NaoHaNova);
        assert_eq!(comparar("1.5.0", "1.6.0"), Comparacao::HaVersaoNova);
        assert_eq!(comparar("1.5.0", "v1.6.0"), Comparacao::HaVersaoNova);

        // 1.10 é MAIOR que 1.9. Comparar como texto diria o contrário, e o
        // cliente deixaria de ser avisado exatamente quando o produto amadurece.
        assert_eq!(comparar("1.9.0", "1.10.0"), Comparacao::HaVersaoNova);

        // Uma versão instalada MAIOR que a publicada acontece em compilação
        // local. Não é aviso, e não é erro.
        assert_eq!(comparar("1.6.0", "1.5.0"), Comparacao::NaoHaNova);
    }

    #[test]
    fn resposta_ilegivel_nao_vira_ha_versao_nova() {
        // Falha de rede ou resposta estranha não pode virar aviso: uma faixa
        // dizendo "atualize" sem haver o que atualizar é o produto mentindo.
        assert_eq!(comparar("1.5.0", ""), Comparacao::NaoSei);
        assert_eq!(comparar("1.5.0", "sei lá"), Comparacao::NaoSei);
    }
}
