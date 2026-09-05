// Resizable BAR
//
// O Resizable BAR deixa o processador enxergar de uma vez a memória inteira da
// placa de vídeo, em vez de espiá-la por uma janela de 256 MiB. Rende alguns
// quadros em jogo — e, como o XMP e os limites de potência do `firmware.rs`, é
// coisa que só a BIOS liga. Nenhum programa liga por fora, e este módulo, como
// o irmão, LÊ e explica: não escreve nada em lugar nenhum.
//
// A razão de existir separado é que aqui SÃO DUAS PERGUNTAS, e confundi-las é o
// erro que a máquina do dono revelou: "está ligado?" e "esta placa suporta?".
// Medido lá: GTX 1650, 4096 MiB de VRAM, BAR1 de 256 MiB — desligado. Só que a
// NVIDIA habilitou Resizable BAR a partir da série RTX 3000: na GTX 1650 não há
// nada para ligar. Um produto que só olhasse "está desligado" mandaria o cliente
// vasculhar a BIOS atrás de uma opção que não existe para ele.
//
// Detectar sem entender é pior do que não detectar.
//
// E vale aqui a terceira regra do produto: "não consegui verificar" nunca vira
// "está tudo bem". Placa AMD, driver antigo, `nvidia-smi` ausente — tudo isso é
// `NaoSei`, e a tela diz que não sabe.

use super::shell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoDoRbar {
    Ligado,
    DesligadoESuportado,
    DesligadoSemSuporte,
    NaoSei,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatorioDoRbar {
    pub estado: EstadoDoRbar,
    /// O modelo lido da placa. Vazio quando não deu para ler.
    pub modelo: String,
    /// A frase que o cliente lê.
    pub nota: String,
}

/// Quanto o BAR1 pode ficar abaixo da VRAM e ainda contar como ligado.
///
/// Dez por cento. Com rBAR ligado o BAR1 cobre a VRAM inteira, mas o firmware
/// reserva uma fatia para si e o número não bate na vírgula. Exigir igualdade
/// exata leria "ligado" como "desligado" em placa perfeitamente configurada.
const FOLGA: f64 = 0.90;

/// Se a família da placa tem Resizable BAR.
///
/// A NVIDIA habilitou rBAR a partir da série RTX 3000. GTX de qualquer geração
/// e RTX 2000 não têm — e é por isso que "está desligado" não basta como
/// resposta: numa GTX 1650 não há nada para ligar.
pub fn suporta_rbar(modelo: &str) -> bool {
    let m = modelo.to_uppercase();

    if !m.contains("RTX") {
        return false;
    }

    // O primeiro dígito da série: RTX 3060 -> 3. RTX 2060 -> 2.
    m.split_whitespace()
        .find_map(|palavra| {
            let digitos: String = palavra.chars().filter(|c| c.is_ascii_digit()).collect();
            digitos.chars().next()
        })
        .and_then(|d| d.to_digit(10))
        .map(|serie| serie >= 3)
        .unwrap_or(false)
}

/// Decide o estado. PURA — é o que permite provar os quatro casos sem placa de
/// vídeo nenhuma.
pub fn avaliar(bar1_mib: Option<u64>, vram_mib: Option<u64>, modelo: &str) -> EstadoDoRbar {
    let (Some(bar1), Some(vram)) = (bar1_mib, vram_mib) else {
        return EstadoDoRbar::NaoSei;
    };

    // Sem saber QUAL é a placa não dá para responder a segunda pergunta, e
    // responder só a primeira é justamente o defeito que este módulo evita.
    if modelo.trim().is_empty() || vram == 0 {
        return EstadoDoRbar::NaoSei;
    }

    if (bar1 as f64) >= (vram as f64) * FOLGA {
        return EstadoDoRbar::Ligado;
    }

    if suporta_rbar(modelo) {
        EstadoDoRbar::DesligadoESuportado
    } else {
        EstadoDoRbar::DesligadoSemSuporte
    }
}

/// A frase que o cliente lê.
pub fn nota_de(estado: EstadoDoRbar) -> String {
    match estado {
        EstadoDoRbar::Ligado => "O Resizable BAR está ligado. Nada a fazer aqui.".to_string(),
        EstadoDoRbar::DesligadoESuportado => {
            "O Resizable BAR está desligado, e a sua placa aceita. Ele deixa o \
             processador enxergar a memória da placa de vídeo inteira de uma vez, \
             e costuma render alguns quadros. Para ativar, entre na BIOS e procure \
             por \"Resizable BAR\" ou \"Above 4G Decoding\" — não dá para ligar por \
             programa nenhum."
                .to_string()
        }
        // SEM A PALAVRA "BIOS", DE PROPÓSITO. Mandar procurar uma opção que não
        // existe para esta placa faria o cliente perder tempo e concluir que o
        // produto não sabe do que fala.
        EstadoDoRbar::DesligadoSemSuporte => {
            "Esta placa de vídeo não tem Resizable BAR — a NVIDIA só passou a \
             oferecer a partir da série RTX 3000. Não há nada para ativar, e isso \
             não é problema no seu PC."
                .to_string()
        }
        EstadoDoRbar::NaoSei => {
            "Não consegui verificar o Resizable BAR nesta máquina. Isso acontece \
             com placa AMD e com driver antigo — e não quer dizer que esteja certo \
             nem errado."
                .to_string()
        }
    }
}

/// Tira o `BAR1 Total` do relatório do `nvidia-smi`. PURA, para ser testável
/// sem placa de vídeo.
///
/// O relatório tem VÁRIAS linhas `Total` — memória da placa, BAR1, memória
/// protegida. Pegar a primeira devolveria a VRAM no lugar do BAR1, e o produto
/// concluiria que o rBAR está sempre ligado, em toda máquina. Por isso a leitura
/// só começa DEPOIS de encontrar o cabeçalho `BAR1 Memory Usage`.
pub fn bar1_da_saida(saida: &str) -> Option<u64> {
    let mut dentro = false;

    for linha in saida.lines() {
        let limpa = linha.trim();

        if limpa.starts_with("BAR1 Memory Usage") {
            dentro = true;
            continue;
        }

        if !dentro {
            continue;
        }

        if let Some((chave, valor)) = limpa.split_once(':') {
            if chave.trim() == "Total" {
                return valor
                    .trim()
                    .trim_end_matches(" MiB")
                    .trim()
                    .parse::<u64>()
                    .ok();
            }
        }
    }

    None
}

/// Lê o BAR1 e a VRAM pelo `nvidia-smi`.
///
/// O `nvidia-smi` acompanha o driver, então está em toda máquina com placa
/// NVIDIA — mesma escolha do `nvapi64.dll`, e a mesma razão pela qual o
/// relatório em PDF usa o Edge: não acrescentar dependência ao instalador.
///
/// Máquina com placa AMD simplesmente não tem o programa, e isso vira `NaoSei` —
/// que a tela mostra como "ainda não cobrimos sua placa".
///
/// A saída de `nvidia-smi -q` é fixa e em inglês, independente do idioma do
/// Windows — ao contrário do DISM. Por isso aqui basta um padrão.
fn ler_do_nvidia_smi() -> (Option<u64>, Option<u64>, String) {
    // `--query-gpu` devolve CSV enxuto; `-q` devolve o relatório inteiro. São
    // duas chamadas porque o BAR1 só aparece no relatório inteiro, e o modelo e
    // a VRAM saem muito mais limpos do CSV.
    let identidade = shell::run(
        "nvidia-smi",
        &["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"],
    );

    let (modelo, vram) = match identidade {
        Ok(saida) if saida.success => {
            let linha = saida.stdout.lines().next().unwrap_or_default().to_string();
            match linha.split_once(',') {
                Some((nome, mib)) => (nome.trim().to_string(), mib.trim().parse::<u64>().ok()),
                None => (String::new(), None),
            }
        }
        // SEM `nvidia-smi` NÃO HÁ PLACA NVIDIA VISÍVEL, e isso não é erro: é o
        // caso do cliente com AMD, que a tela trata dizendo que ainda não é
        // coberto.
        _ => return (None, None, String::new()),
    };

    let bar1 = shell::run("nvidia-smi", &["-q"])
        .ok()
        .filter(|s| s.success)
        .and_then(|s| bar1_da_saida(&s.stdout));

    (bar1, vram, modelo)
}

/// O que a tela mostra: o estado, a placa e a frase.
pub fn analyze() -> RelatorioDoRbar {
    let (bar1, vram, modelo) = ler_do_nvidia_smi();
    let estado = avaliar(bar1, vram, &modelo);

    RelatorioDoRbar {
        estado,
        modelo,
        nota: nota_de(estado),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placa_sem_suporte_nao_vira_conselho_de_bios() {
        // O DEFEITO QUE A MAQUINA DO DONO REVELOU.
        //
        // Medido la: GTX 1650, 4096 MiB de VRAM, BAR1 de 256 MiB. Desligado. Mas
        // a NVIDIA so habilitou Resizable BAR da RTX 3000 para cima -- a 1650 NAO
        // TEM esse recurso. Um produto que so olhasse "esta desligado" mandaria o
        // cliente entrar na BIOS procurar uma opcao que nao vai adiantar nada.
        //
        // Detectar sem entender e pior do que nao detectar.
        assert_eq!(
            avaliar(Some(256), Some(4096), "NVIDIA GeForce GTX 1650"),
            EstadoDoRbar::DesligadoSemSuporte
        );
    }

    #[test]
    fn placa_com_suporte_e_desligada_vira_conselho() {
        assert_eq!(
            avaliar(Some(256), Some(8192), "NVIDIA GeForce RTX 3060 Ti"),
            EstadoDoRbar::DesligadoESuportado
        );
    }

    #[test]
    fn bar1_do_tamanho_da_vram_e_ligado() {
        // Ligado, o BAR1 passa a cobrir toda a VRAM. A folga de 10% cobre o que o
        // firmware reserva para si.
        assert_eq!(
            avaliar(Some(8192), Some(8192), "NVIDIA GeForce RTX 3060 Ti"),
            EstadoDoRbar::Ligado
        );
        assert_eq!(
            avaliar(Some(7800), Some(8192), "NVIDIA GeForce RTX 3060 Ti"),
            EstadoDoRbar::Ligado
        );
    }

    #[test]
    fn sem_leitura_e_nao_sei_e_nunca_esta_tudo_bem() {
        // A regra do produto. Nao conseguir ler o BAR1 -- driver antigo, placa
        // AMD, `nvidia-smi` ausente -- nao pode virar "esta ligado, nada a fazer".
        assert_eq!(
            avaliar(None, Some(8192), "NVIDIA GeForce RTX 3060 Ti"),
            EstadoDoRbar::NaoSei
        );
        assert_eq!(
            avaliar(Some(256), None, "NVIDIA GeForce RTX 3060 Ti"),
            EstadoDoRbar::NaoSei
        );
        assert_eq!(avaliar(Some(256), Some(8192), ""), EstadoDoRbar::NaoSei);
    }

    #[test]
    fn a_familia_da_placa_decide_o_suporte() {
        assert!(suporta_rbar("NVIDIA GeForce RTX 3060"));
        assert!(suporta_rbar("NVIDIA GeForce RTX 4070 Ti"));
        assert!(suporta_rbar("NVIDIA GeForce RTX 5080"));

        assert!(!suporta_rbar("NVIDIA GeForce GTX 1650"));
        assert!(!suporta_rbar("NVIDIA GeForce GTX 1080 Ti"));
        // RTX 2000 nao recebeu rBAR.
        assert!(!suporta_rbar("NVIDIA GeForce RTX 2060"));
    }

    #[test]
    fn a_nota_nunca_manda_mexer_na_bios_sem_suporte() {
        // CANARIO DE TELA. A frase e o que o cliente le, e mandar alguem procurar
        // na BIOS uma opcao que nao existe para a placa dele e o erro que este
        // modulo existe para nao cometer.
        let nota = nota_de(EstadoDoRbar::DesligadoSemSuporte);
        assert!(
            !nota.to_lowercase().contains("bios"),
            "mandou na BIOS a toa: {}",
            nota
        );

        let com_suporte = nota_de(EstadoDoRbar::DesligadoESuportado);
        assert!(
            com_suporte.to_lowercase().contains("bios"),
            "faltou dizer onde ativar"
        );
    }

    #[test]
    fn a_leitura_do_bar1_nao_pega_a_memoria_da_placa_por_engano() {
        // O relatorio do `nvidia-smi` tem VARIAS linhas "Total". Pegar a primeira
        // devolveria a VRAM no lugar do BAR1 -- e o produto concluiria que o
        // Resizable BAR esta SEMPRE ligado, em toda maquina.
        //
        // Este trecho e a saida real da maquina do dono, encurtada.
        let saida = "\
    FB Memory Usage
        Total                             : 4096 MiB
        Used                              : 900 MiB
    BAR1 Memory Usage
        Total                             : 256 MiB
        Used                              : 2 MiB
        Free                              : 254 MiB
    Conf Compute Protected Memory Usage
        Total                             : 0 MiB";

        assert_eq!(bar1_da_saida(saida), Some(256), "leu a linha Total errada");
    }

    #[test]
    fn saida_sem_bar1_e_nao_sei() {
        assert_eq!(bar1_da_saida("FB Memory Usage\n    Total : 4096 MiB"), None);
        assert_eq!(bar1_da_saida(""), None);
    }

    #[test]
    fn analisa_esta_maquina() {
        let relatorio = analyze();
        println!(
            "[{:?}] {} — {}",
            relatorio.estado, relatorio.modelo, relatorio.nota
        );

        // O que vale em qualquer maquina, com placa ou sem: a frase acompanha o
        // estado, e "nao sei" nunca sai calado.
        assert_eq!(relatorio.nota, nota_de(relatorio.estado));
        assert!(!relatorio.nota.trim().is_empty());
    }
}
