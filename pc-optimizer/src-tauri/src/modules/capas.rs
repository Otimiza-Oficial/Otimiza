// As capas dos jogos, tiradas da própria máquina
//
// O PEDIDO E A RESPOSTA
//
// O pedido foi "coloque a foto dos jogos de verdade, não essas letras". Certo:
// uma grade de iniciais coloridas é um substituto, e um substituto é o que se
// usa quando não há o de verdade.
//
// O caminho que NÃO serve é embutir as capas no instalador. A arte de cada
// jogo é de quem fez o jogo; empacotá-la num produto que se vende é distribuir
// material de terceiro, e nenhuma quantidade de utilidade muda isso.
//
// O caminho que serve — e que entrega exatamente o que foi pedido — é que AS
// CAPAS JÁ ESTÃO NO COMPUTADOR DO CLIENTE. A Steam baixa a capa de cada jogo da
// biblioteca dele e guarda em disco para desenhar a própria grade. Ler dali é
// mostrar ao cliente uma imagem que já é dele, posta ali pelo programa que ele
// instalou. Nada é distribuído, nada é baixado por nós, e a capa é a de
// verdade.
//
// É o mesmo que qualquer frente de biblioteca de jogos faz.
//
// ONDE A STEAM GUARDA
//
// Mudou de lugar entre versões, e as duas formas convivem em máquinas que
// atualizaram por cima:
//
//   appcache/librarycache/<appid>_library_600x900.jpg   (formato antigo)
//   appcache/librarycache/<appid>/library_600x900.jpg   (formato novo)
//
// A ordem da busca é: retrato primeiro, porque é o formato da grade; o
// cabeçalho horizontal depois, porque é melhor que bloco de letra; e o logotipo
// por último. Um `header.jpg` esticado num bloco 3:4 fica feio — mas feio com a
// arte certa ainda diz mais que "CS" escrito num quadrado.
//
// E QUANDO O JOGO NÃO ESTÁ INSTALADO
//
// Aí não há capa em lugar nenhum desta máquina — e a grade fica quase toda de
// letras, porque a maior parte dos títulos conhecidos não está instalada.
//
// Para esses, a capa é buscada UMA VEZ na CDN pública da Steam, pelo número do
// jogo, e guardada aqui do lado. É a mesma imagem que a Steam serve para
// qualquer página de loja; a diferença para o caso acima é que ela chega pela
// rede em vez de já estar no disco.
//
// ISSO TEM DUAS CONSEQUÊNCIAS QUE PRECISAM ESTAR DITAS:
//
//   1. O programa faz uma requisição de rede para desenhar a biblioteca. Uma
//      por jogo, uma única vez na vida da instalação.
//   2. A imagem é da Steam. Ela é exibida como capa do jogo que ela representa
//      — que é para o que ela existe —, e não é redistribuída: fica no cache
//      desta máquina como qualquer imagem que um navegador guarda.
//
// O NÚMERO DO JOGO É O PONTO FRÁGIL. Um número errado não dá erro: dá a capa
// de OUTRO jogo no bloco, com toda a confiança. Por isso cada um deles foi
// conferido contra a API pública da Steam antes de entrar no catálogo.
//
// Jogo sem número — Valorant, Fortnite, League, Minecraft, Roblox, Tarkov,
// FiveM, que não estão na Steam — continua com o bloco de cor.

use std::path::{Path, PathBuf};

/// Os arquivos de capa que a Steam pode ter, em ordem de preferência.
///
/// Caminhos RELATIVOS à pasta da Steam, e função pura: a lista é a decisão de
/// produto (qual imagem serve melhor a uma grade em retrato), e ela precisa ser
/// testável sem ter Steam instalada na máquina de quem roda os testes.
pub fn candidatos_steam(appid: u32) -> Vec<String> {
    let base = "appcache/librarycache";

    // Retrato primeiro: é o formato da grade. Depois o cabeçalho, que é
    // horizontal e vai ficar cortado — e ainda assim diz mais que uma letra.
    [
        format!("{base}/{appid}/library_600x900.jpg"),
        format!("{base}/{appid}/library_600x900.png"),
        format!("{base}/{appid}_library_600x900.jpg"),
        format!("{base}/{appid}_library_600x900.png"),
        format!("{base}/{appid}/header.jpg"),
        format!("{base}/{appid}_header.jpg"),
        format!("{base}/{appid}/library_hero.jpg"),
        format!("{base}/{appid}_library_hero.jpg"),
    ]
    .to_vec()
}

/// O primeiro candidato que existe no disco.
pub fn procurar_steam(raiz: &Path, appid: u32) -> Option<PathBuf> {
    candidatos_steam(appid)
        .into_iter()
        .map(|rel| raiz.join(rel.replace('/', "\\")))
        .find(|caminho| caminho.is_file())
}

/// Quanto uma capa pode pesar para virar dado embutido na tela.
///
/// Quatro megabytes. Uma capa de biblioteca tem entre 50 e 300 KB; qualquer
/// coisa muito acima disso não é capa, é outra imagem que foi parar ali — e
/// mandar dezenas de megabytes por dentro da mensagem da tela trava a
/// interface justamente enquanto ela tenta parecer rápida.
pub const TETO_DA_CAPA_BYTES: u64 = 4 * 1024 * 1024;

/// O tipo de imagem, pelo fim do nome.
///
/// Pela EXTENSÃO e não pelo conteúdo: o arquivo é da Steam, não de origem
/// desconhecida, e abrir cada um para farejar o cabeçalho custaria uma leitura
/// a mais por jogo sem mudar a resposta.
pub fn tipo_de(caminho: &Path) -> Option<&'static str> {
    match caminho
        .extension()
        .and_then(|e| e.to_str())?
        .to_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// Lê a capa e devolve uma URL de dados pronta para a tela.
///
/// `None` quando o arquivo não existe, não é imagem conhecida, ou passa do
/// teto. Nenhum desses casos é erro: é uma capa a menos, e o bloco de cor
/// cobre.
pub fn ler_como_url(caminho: &Path) -> Option<String> {
    let tipo = tipo_de(caminho)?;

    let tamanho = std::fs::metadata(caminho).ok()?.len();
    if tamanho == 0 || tamanho > TETO_DA_CAPA_BYTES {
        return None;
    }

    let bytes = std::fs::read(caminho).ok()?;
    Some(format!("data:{tipo};base64,{}", base64(&bytes)))
}

/// Base64 padrão, sem trazer dependência para isto.
///
/// São trinta linhas contra uma caixa a mais no instalador e mais uma
/// atualização de segurança para acompanhar. A codificação não muda desde
/// 1987; o custo de manutenção disto é zero.
fn base64(bytes: &[u8]) -> String {
    const TABELA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut saida = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for pedaco in bytes.chunks(3) {
        let b0 = pedaco[0] as u32;
        let b1 = *pedaco.get(1).unwrap_or(&0) as u32;
        let b2 = *pedaco.get(2).unwrap_or(&0) as u32;
        let junto = (b0 << 16) | (b1 << 8) | b2;

        saida.push(TABELA[(junto >> 18) as usize & 63] as char);
        saida.push(TABELA[(junto >> 12) as usize & 63] as char);

        // O enchimento não é enfeite: sem ele, quem decodifica não sabe se os
        // últimos bits são dado ou sobra.
        saida.push(if pedaco.len() > 1 {
            TABELA[(junto >> 6) as usize & 63] as char
        } else {
            '='
        });
        saida.push(if pedaco.len() > 2 {
            TABELA[junto as usize & 63] as char
        } else {
            '='
        });
    }

    saida
}

// ------------------------------------------------- a capa que vem da rede

/// De onde a Steam serve a arte pública de cada jogo.
///
/// O mesmo endereço que a página de loja usa. Retrato primeiro, cabeçalho
/// depois — a mesma ordem da busca em disco, e pela mesma razão.
pub fn enderecos_da_steam(appid: u32) -> Vec<String> {
    let base = "https://cdn.cloudflare.steamstatic.com/steam/apps";

    vec![
        format!("{base}/{appid}/library_600x900_2x.jpg"),
        format!("{base}/{appid}/library_600x900.jpg"),
        format!("{base}/{appid}/header.jpg"),
    ]
}

/// Onde a capa baixada fica guardada nesta máquina.
///
/// Ao lado dos outros dados do produto. Uma pasta por conta própria porque ela
/// pode crescer alguns megabytes, e quem for limpar precisa saber o que está
/// apagando.
pub fn pasta_do_cache() -> std::path::PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    base.join("pc-optimizer").join("capas")
}

/// A capa deste jogo, de onde quer que ela venha.
///
/// A ordem é a do custo: o cache da Steam é leitura de disco; o cache do
/// produto também; a rede é a última. Uma vez baixada, a capa nunca é baixada
/// de novo.
///
/// `raiz_steam` é onde a Steam está instalada, quando está.
pub async fn obter(appid: u32, raiz_steam: Option<&Path>) -> Option<String> {
    // 1. O cache da própria Steam. Melhor fonte: já é do cliente e não custa
    //    rede nenhuma.
    if let Some(raiz) = raiz_steam {
        if let Some(arquivo) = procurar_steam(raiz, appid) {
            return ler_como_url(&arquivo);
        }
    }

    // 2. O que já foi baixado antes.
    let guardada = pasta_do_cache().join(format!("{appid}.jpg"));
    if guardada.is_file() {
        return ler_como_url(&guardada);
    }

    // 3. A rede, uma vez só.
    let bytes = baixar(appid).await?;

    // Gravar é o que impede a próxima abertura de baixar de novo. Falhar aqui
    // não perde a capa desta vez — só faz a próxima abrir buscando de novo.
    if let Some(pasta) = guardada.parent() {
        let _ = std::fs::create_dir_all(pasta);
    }
    let _ = std::fs::write(&guardada, &bytes);

    Some(format!("data:image/jpeg;base64,{}", base64(&bytes)))
}

/// Baixa a primeira capa que responder.
async fn baixar(appid: u32) -> Option<Vec<u8>> {
    let cliente = reqwest::Client::builder()
        // Tempo curto: isto é enfeite de tela. Uma biblioteca que demora dez
        // segundos para desenhar porque a rede caiu é pior que uma biblioteca
        // de letras.
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .ok()?;

    for endereco in enderecos_da_steam(appid) {
        let Ok(resposta) = cliente.get(&endereco).send().await else {
            continue;
        };

        if !resposta.status().is_success() {
            continue;
        }

        let Ok(bytes) = resposta.bytes().await else {
            continue;
        };

        // Resposta vazia ou gigante não é capa. O mesmo teto da leitura em
        // disco, pela mesma razão.
        if bytes.is_empty() || bytes.len() as u64 > TETO_DA_CAPA_BYTES {
            continue;
        }

        return Some(bytes.to_vec());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_endereco_publico_pede_o_retrato_primeiro() {
        let lista = enderecos_da_steam(730);

        assert!(lista[0].contains("library_600x900"), "{lista:?}");
        assert!(lista.iter().all(|u| u.starts_with("https://")), "só https");
        assert!(
            lista.iter().all(|u| u.contains("/730/")),
            "o número tem de entrar"
        );
    }

    /// A capa que vem da rede, de verdade.
    ///
    /// Pega um jogo que NÃO está instalado nesta máquina — que é o caso da
    /// maior parte da grade, e o que sobrou de letras depois da primeira
    /// versão. Rede fora não reprova: o teste diz e sai.
    #[tokio::test]
    async fn a_capa_de_jogo_nao_instalado_vem_da_rede_e_fica_guardada() {
        // Marvel Rivals. Número conferido contra a API pública da Steam.
        const APPID: u32 = 2767030;

        let guardada = pasta_do_cache().join(format!("{APPID}.jpg"));
        let _ = std::fs::remove_file(&guardada);

        // Sem raiz de Steam: obriga o caminho da rede, que é o que se testa.
        let Some(url) = obter(APPID, None).await else {
            println!("sem rede, ou a CDN não respondeu");
            return;
        };

        assert!(url.starts_with("data:image/jpeg;base64,"), "{}", &url[..40]);
        assert!(url.len() > 10_000, "capa curta demais para ser imagem");

        // E ficou guardada: a próxima abertura não baixa de novo.
        assert!(guardada.is_file(), "a capa baixada tem de ficar em disco");

        // A segunda chamada tem de vir do disco, e não da rede. Não dá para
        // medir isso daqui sem cronômetro frágil — o que dá para afirmar é que
        // ela devolve a MESMA imagem.
        let de_novo = obter(APPID, None).await.expect("segunda leitura");
        assert_eq!(url, de_novo);

        let _ = std::fs::remove_file(&guardada);
    }

    #[test]
    fn o_retrato_vem_antes_do_cabecalho() {
        let lista = candidatos_steam(271590);

        let retrato = lista
            .iter()
            .position(|c| c.contains("600x900"))
            .expect("retrato");
        let cabecalho = lista
            .iter()
            .position(|c| c.contains("header"))
            .expect("cabeçalho");

        assert!(retrato < cabecalho, "a grade é em retrato: {lista:?}");
    }

    /// As duas formas de guardar convivem em máquina que atualizou por cima.
    #[test]
    fn as_duas_formas_da_steam_sao_procuradas() {
        let lista = candidatos_steam(730);

        assert!(
            lista.iter().any(|c| c.contains("/730/library_600x900")),
            "formato novo"
        );
        assert!(
            lista.iter().any(|c| c.contains("730_library_600x900")),
            "formato antigo"
        );
    }

    #[test]
    fn so_imagem_conhecida_vira_capa() {
        assert_eq!(tipo_de(Path::new("a/b/capa.jpg")), Some("image/jpeg"));
        assert_eq!(tipo_de(Path::new("a/b/capa.JPEG")), Some("image/jpeg"));
        assert_eq!(tipo_de(Path::new("a/b/capa.png")), Some("image/png"));
        assert_eq!(tipo_de(Path::new("a/b/capa.exe")), None);
        assert_eq!(tipo_de(Path::new("a/b/semextensao")), None);
    }

    #[test]
    fn arquivo_que_nao_existe_nao_vira_capa() {
        let fora = std::env::temp_dir().join("otimiza-capa-que-nao-existe.jpg");
        let _ = std::fs::remove_file(&fora);

        assert_eq!(ler_como_url(&fora), None);
    }

    #[test]
    fn arquivo_vazio_nao_vira_capa() {
        let vazio = std::env::temp_dir().join("otimiza-capa-vazia.png");
        std::fs::write(&vazio, b"").expect("escrever");

        assert_eq!(ler_como_url(&vazio), None);
        let _ = std::fs::remove_file(&vazio);
    }

    #[test]
    fn a_url_sai_com_o_tipo_certo() {
        let arquivo = std::env::temp_dir().join("otimiza-capa-teste.png");
        std::fs::write(&arquivo, b"conteudo qualquer").expect("escrever");

        let url = ler_como_url(&arquivo).expect("url");
        assert!(url.starts_with("data:image/png;base64,"), "{url}");

        let _ = std::fs::remove_file(&arquivo);
    }

    /// Os vetores do RFC 4648. Se o enchimento sair errado, a tela mostra uma
    /// imagem cortada — e o defeito só aparece em algumas capas, que é o pior
    /// tipo de defeito.
    #[test]
    fn a_codificacao_bate_com_o_padrao() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    /// A capa de verdade, no cache de verdade desta máquina.
    ///
    /// Não afirma que existe Steam aqui — máquina sem Steam sai sem reprovar.
    /// Afirma o CONTRATO: onde houver cache, o primeiro candidato encontrado
    /// tem de virar uma URL de imagem que a tela consiga desenhar.
    ///
    /// É o teste que pega o erro que nenhum outro pega: a Steam mudar o nome
    /// dos arquivos e a grade voltar a ser um campo de letras sem ninguém
    /// entender por quê.
    #[cfg(target_os = "windows")]
    #[test]
    fn a_capa_de_verdade_vira_url_desenhavel() {
        let Some(raiz) = crate::modules::windows::jogos::varrer().raiz_steam else {
            return;
        };

        let cache = raiz.join("appcache").join("librarycache");
        let Ok(entradas) = std::fs::read_dir(&cache) else {
            return;
        };

        // Um appid qualquer que o cache já tenha preenchido.
        let achou = entradas
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                e.file_name()
                    .to_str()?
                    .split('_')
                    .next()?
                    .parse::<u32>()
                    .ok()
            })
            .find_map(|appid| procurar_steam(&raiz, appid));

        let Some(arquivo) = achou else {
            println!("cache de capas vazio nesta máquina");
            return;
        };

        println!("capa encontrada: {}", arquivo.display());
        let url = ler_como_url(&arquivo).expect("a capa encontrada tem de virar url");

        assert!(
            url.starts_with("data:image/"),
            "{}",
            &url[..40.min(url.len())]
        );
        assert!(url.len() > 1000, "capa curta demais para ser imagem");
    }

    /// Bytes altos: um JPEG é cheio deles, e um erro de sinal aqui estragaria
    /// toda capa real sem estragar nenhum teste de texto.
    #[test]
    fn bytes_altos_sobrevivem() {
        assert_eq!(base64(&[0xff, 0xd8, 0xff]), "/9j/");
        assert_eq!(base64(&[0x00, 0x00, 0x00]), "AAAA");
    }
}
