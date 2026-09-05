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
///
/// ESTA FOLGA FOI PENSADA PARA A FATIA DO FIRMWARE, QUE É DE POUCOS MiB — não
/// para arredondamento de BAR, que é de gigabytes. Ver `piso_potencia_de_dois`.
const FOLGA: f64 = 0.90;

/// A maior potência de dois que cabe na VRAM.
///
/// O BAR do PCIe tem tamanho em potência de dois. Numa placa cuja VRAM NÃO é
/// potência de dois — e são justamente as do nosso público: RTX 3060 de 12 GiB,
/// RTX 3080 de 10 GiB, RTX 4070 de 12 GiB — o firmware pode não ter como abrir
/// uma janela do tamanho exato da memória e parar no degrau de baixo: 8192 MiB
/// para 12288 de VRAM. Isso dá 0,67, reprova na `FOLGA`, e o cliente QUE JÁ
/// LIGOU O rBAR leria "está desligado, entre na BIOS".
///
/// SE O FIRMWARE ARREDONDA ASSIM É DÚVIDA NOSSA, NÃO FATO MEDIDO: a máquina do
/// dono é uma GTX 1650, com o BAR1 preso em 256 MiB, e não dá para observar o
/// caso aqui. O que sustenta a regra é que ela é segura nos dois cenários: se o
/// firmware NÃO arredondar, o BAR1 vem do tamanho da VRAM e a `FOLGA` já
/// aprovava sozinha; se arredondar, esta linha aprova. E ela só chega perto de
/// disparar por engano num número que placa SEM rBAR nunca produz — sem o
/// recurso o BAR1 fica na janela clássica de 256 MiB, muito abaixo do degrau.
fn piso_potencia_de_dois(vram_mib: u64) -> u64 {
    if vram_mib == 0 {
        return 0;
    }
    1u64 << (u64::BITS - 1 - vram_mib.leading_zeros())
}

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

    if (bar1 as f64) >= (vram as f64) * FOLGA || bar1 >= piso_potencia_de_dois(vram) {
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
        // A PLACA DE VÍDEO ACEITAR NÃO BASTA: o Resizable BAR exige a plataforma
        // inteira — placa-mãe e BIOS também. Só que "Above 4G Decoding" existe em
        // placa-mãe MUITO ANTERIOR ao rBAR, então o cliente com uma RTX 3060 numa
        // Z170 ENCONTRA a opção, liga, reinicia, e nada muda. É pior do que o caso
        // da GTX 1650: lá ele não acha nada e desiste; aqui ele mexe no firmware e
        // sai com o PC igual e a impressão de que o produto errou.
        //
        // Detectar a placa-mãe exigiria ler o firmware. AVISAR NÃO EXIGE NADA —
        // é uma oração, e a regra do produto é sobre o que o cliente LÊ.
        EstadoDoRbar::DesligadoESuportado => {
            "O Resizable BAR está desligado, e a sua placa de vídeo aceita. Ele \
             deixa o processador enxergar a memória da placa de vídeo inteira de \
             uma vez, e costuma render alguns quadros. Para ativar, entre na BIOS \
             e procure por \"Resizable BAR\" ou \"Above 4G Decoding\" — não dá para \
             ligar por programa nenhum. Se você não encontrar essa opção, a sua \
             placa-mãe é anterior a essa tecnologia e não há o que fazer."
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
        // "PLACA AMD" E "DRIVER ANTIGO" SÃO DE NATUREZAS OPOSTAS e não cabem na
        // mesma frase: a primeira é permanente e conhecida, a segunda é passageira
        // e tem conserto. Juntas, mandam o dono de Radeon atualizar driver à toa.
        // O cabeçalho deste módulo promete que a tela diz "ainda não cobrimos sua
        // placa" — é isso que ela tem de dizer.
        EstadoDoRbar::NaoSei => {
            "Não consegui verificar o Resizable BAR nesta máquina: ainda não \
             cobrimos placas AMD. Isso não quer dizer que esteja certo nem errado."
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

        // FIM DA SEÇÃO. No relatório do `nvidia-smi` só cabeçalho vem sem `:`.
        // Sem fechar aqui, uma saída truncada — seção `BAR1 Memory Usage` sem a
        // linha `Total` — escorregaria para a seção seguinte e devolveria o
        // `Total : 0 MiB` do `Conf Compute Protected Memory Usage` como se fosse
        // o BAR1. É o primo do engano que este módulo existe para evitar: número
        // de outra seção passando por BAR1. Cai para `None`, que vira `NaoSei`.
        if !limpa.is_empty() && !limpa.contains(':') {
            return None;
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

/// Tira o modelo e a VRAM do CSV do `nvidia-smi`. PURA, irmã de `bar1_da_saida`.
///
/// Recebe o `sucesso` em vez de olhar o `CommandOutput` porque É EXATAMENTE ISSO
/// QUE PRECISA SER PROVADO: comando que existe mas falha devolve stdout que pode
/// até ter cara de resposta, e ignorar o código de saída faria uma leitura que
/// não aconteceu virar identidade de placa. Testar isso sem esta separação
/// exigiria injetar o `shell`, que sairia do desenho dos módulos vizinhos.
fn identidade_da_saida(sucesso: bool, stdout: &str) -> (String, Option<u64>) {
    if !sucesso {
        return (String::new(), None);
    }

    let linha = stdout.lines().next().unwrap_or_default();
    match linha.split_once(',') {
        Some((nome, mib)) => (nome.trim().to_string(), mib.trim().parse::<u64>().ok()),
        None => (String::new(), None),
    }
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
        Ok(saida) => identidade_da_saida(saida.success, &saida.stdout),
        // SEM `nvidia-smi` NÃO HÁ PLACA NVIDIA VISÍVEL, e isso não é erro: é o
        // caso do cliente com AMD, que a tela trata dizendo que ainda não é
        // coberto.
        Err(_) => return (None, None, String::new()),
    };

    // Sem identidade não vale pagar a segunda chamada: o veredito já é `NaoSei`.
    if modelo.is_empty() && vram.is_none() {
        return (None, None, String::new());
    }

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
    fn vram_zerada_e_leitura_falha_e_nao_placa_ligada() {
        // VRAM zero nao existe em placa nenhuma: e o `nvidia-smi` devolvendo
        // lixo. Sem esta guarda a conta passa (0 >= 0) e o produto anunciaria
        // "Resizable BAR ligado" para uma leitura que nao aconteceu -- de novo
        // o "nao sei" virando "esta tudo bem".
        assert_eq!(
            avaliar(Some(0), Some(0), "NVIDIA GeForce RTX 3060"),
            EstadoDoRbar::NaoSei
        );
    }

    #[test]
    fn a_fronteira_da_folga_e_presa_dos_dois_lados() {
        // O MEIO DO CAMINHO NAO PODE VIRAR "ESTA TUDO BEM".
        //
        // Os testes de antes prendiam so os extremos: 7800/8192 aprovava e
        // 4096/8192 reprovava, o que deixa toda a faixa entre ~0,07 e 0,90
        // solta. Baixar a `FOLGA` para 0,50 numa refatoracao futura faria um
        // BAR1 de 4096 numa VRAM de 8192 -- metade da janela -- passar a valer
        // "Ligado. Nada a fazer aqui.", sem nada acusar.
        //
        // 8192 * 0,90 = 7372,8. Um MiB de cada lado prende a constante.
        let placa = "NVIDIA GeForce RTX 3060 Ti";
        assert_eq!(avaliar(Some(7373), Some(8192), placa), EstadoDoRbar::Ligado);
        assert_eq!(
            avaliar(Some(7372), Some(8192), placa),
            EstadoDoRbar::DesligadoESuportado
        );
        assert_eq!(
            avaliar(Some(4096), Some(8192), placa),
            EstadoDoRbar::DesligadoESuportado,
            "meia janela virou rBAR ligado"
        );
    }

    #[test]
    fn vram_que_nao_e_potencia_de_dois_nao_acusa_desligado_a_toa() {
        // AS PLACAS DO NOSSO PUBLICO: RTX 3060 de 12 GiB, RTX 3080 de 10 GiB,
        // RTX 4070 de 12 GiB. O BAR do PCIe tem tamanho em potencia de dois, e
        // 12288 nao e. Se o firmware parar no degrau de baixo, o BAR1 vem 8192 e
        // a conta da `FOLGA` da 0,67 -- reprova. O cliente QUE JA LIGOU o rBAR
        // leria "esta desligado, entre na BIOS".
        //
        // A `FOLGA` de 10% foi escolhida para a fatia que o firmware reserva,
        // que e de poucos MiB. Arredondamento de BAR e de gigabytes.
        assert_eq!(
            avaliar(Some(8192), Some(12288), "NVIDIA GeForce RTX 3060"),
            EstadoDoRbar::Ligado
        );
        assert_eq!(
            avaliar(Some(8192), Some(10240), "NVIDIA GeForce RTX 3080"),
            EstadoDoRbar::Ligado
        );

        // E o degrau NAO pode afrouxar quem ja estava certo: sem rBAR o BAR1
        // fica na janela classica de 256 MiB, muito abaixo de qualquer degrau.
        assert_eq!(
            avaliar(Some(256), Some(12288), "NVIDIA GeForce RTX 3060"),
            EstadoDoRbar::DesligadoESuportado
        );
        assert_eq!(
            avaliar(Some(256), Some(4096), "NVIDIA GeForce GTX 1650"),
            EstadoDoRbar::DesligadoSemSuporte
        );
        // Em VRAM que JA e potencia de dois o degrau coincide com a VRAM e nao
        // afrouxa nada: 4096 de 8192 continua desligado.
        assert_eq!(piso_potencia_de_dois(8192), 8192);
        assert_eq!(piso_potencia_de_dois(12288), 8192);
        assert_eq!(piso_potencia_de_dois(10240), 8192);
        assert_eq!(piso_potencia_de_dois(0), 0);
    }

    #[test]
    fn identidade_de_comando_que_falhou_nao_vira_placa() {
        // O `nvidia-smi` pode existir e ainda assim falhar -- driver a meio
        // caminho de uma atualizacao, por exemplo. Ignorar o codigo de saida
        // faria o stdout de um comando que deu errado virar identidade de placa,
        // e a regra do produto diz que leitura que nao aconteceu nunca vira
        // resposta.
        assert_eq!(
            identidade_da_saida(false, "NVIDIA GeForce RTX 4090, 24576"),
            (String::new(), None)
        );
        assert_eq!(
            identidade_da_saida(true, "NVIDIA GeForce RTX 4090, 24576"),
            ("NVIDIA GeForce RTX 4090".to_string(), Some(24576))
        );
        // Saida vazia ou sem virgula nao inventa placa nenhuma.
        assert_eq!(identidade_da_saida(true, ""), (String::new(), None));
        assert_eq!(identidade_da_saida(true, "erro"), (String::new(), None));
    }

    #[test]
    fn secao_do_bar1_truncada_nao_pega_numero_da_seguinte() {
        // Saida cortada: a secao `BAR1 Memory Usage` sem a linha `Total`. Sem
        // fechar a secao no cabecalho seguinte, a varredura escorrega e devolve
        // o `Total : 0 MiB` da memoria protegida como se fosse o BAR1 -- numero
        // de outra secao passando por BAR1, que e o engano que este modulo
        // existe para evitar.
        let saida = "\
    BAR1 Memory Usage
        Used                              : 2 MiB
    Conf Compute Protected Memory Usage
        Total                             : 0 MiB";

        assert_eq!(bar1_da_saida(saida), None, "leu o Total da seção seguinte");
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

        // O numero da serie so quer dizer alguma coisa DENTRO da numeracao da
        // NVIDIA. Sem exigir "RTX" no nome, uma Radeon RX 7900 leria 7 e seria
        // dada como suportada -- e o cliente receberia conselho de BIOS baseado
        // numa regra que nao vale para a placa dele.
        assert!(!suporta_rbar("AMD Radeon RX 7900 XTX"));
        assert!(!suporta_rbar("Intel Arc A770"));
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
    fn a_nota_com_suporte_avisa_que_a_placa_mae_tambem_conta() {
        // A ARMADILHA DA GTX 1650 DE NOVO, AGORA PELA PLACA-MAE.
        //
        // Cliente com RTX 3060 numa Z170: a placa de video aceita, o estado e
        // `DesligadoESuportado`, e a frase manda procurar "Above 4G Decoding".
        // Essa opcao EXISTE em placa-mae muito anterior ao rBAR -- ele acha,
        // liga, reinicia, e nada muda, porque o recurso exige a plataforma
        // inteira. Pior que a GTX 1650: la ele nao acha nada e desiste; aqui ele
        // mexe no firmware e sai achando que o produto errou.
        //
        // Detectar a placa-mae exige ler o firmware. AVISAR nao exige nada.
        let nota = nota_de(EstadoDoRbar::DesligadoESuportado).to_lowercase();
        assert!(
            nota.contains("placa-mãe"),
            "a frase manda mexer na BIOS sem dizer que a placa-mãe também decide: {}",
            nota
        );
        assert!(
            nota.contains("não encontrar"),
            "faltou dizer o que fazer quando a opção não está lá: {}",
            nota
        );
    }

    #[test]
    fn a_nota_do_nao_sei_nao_manda_atualizar_driver_a_toa() {
        // "Placa AMD" e "driver antigo" sao de naturezas opostas: uma e
        // permanente e conhecida, a outra e passageira e tem conserto. Juntas,
        // o dono de Radeon le "nao consegui verificar" e vai atualizar driver
        // atras de um resultado que nunca vai aparecer. O cabecalho do modulo
        // promete "ainda nao cobrimos sua placa" -- e isso que a tela diz.
        let nota = nota_de(EstadoDoRbar::NaoSei).to_lowercase();
        assert!(nota.contains("ainda não cobrimos"), "{}", nota);
        assert!(
            !nota.contains("driver"),
            "mandou o dono de Radeon caçar driver: {}",
            nota
        );
    }

    #[test]
    fn cada_estado_tem_a_sua_propria_frase() {
        // Quatro estados, quatro frases. Repetir texto entre estados e o mesmo
        // erro de sempre com outra roupa: seria "nao sei" saindo com cara de
        // "esta ligado, nada a fazer", ou o contrario.
        let frases = [
            nota_de(EstadoDoRbar::Ligado),
            nota_de(EstadoDoRbar::DesligadoESuportado),
            nota_de(EstadoDoRbar::DesligadoSemSuporte),
            nota_de(EstadoDoRbar::NaoSei),
        ];

        for (i, uma) in frases.iter().enumerate() {
            assert!(!uma.trim().is_empty(), "estado {} saiu calado", i);
            for outra in frases.iter().skip(i + 1) {
                assert_ne!(uma, outra, "duas frases iguais para estados diferentes");
            }
        }

        // A do "ligado" nunca pode confessar ignorancia, nem a do "nao sei"
        // pode dar a placa por resolvida.
        assert!(!frases[0].to_lowercase().contains("não consegui"));
        assert!(frases[3].to_lowercase().contains("não consegui"));
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

        // E o veredito precisa ser sobre A PLACA QUE ESTA AQUI. Sem isto, um
        // modelo trocado no caminho entre a leitura e a decisao passaria
        // despercebido -- e o cliente com GTX 1650 receberia o conselho de
        // BIOS da RTX. Em maquina sem `nvidia-smi` (a esteira, placa AMD) o
        // modelo vem vazio e o estado ja e `NaoSei`, entao nada a conferir.
        if !relatorio.modelo.trim().is_empty() {
            match relatorio.estado {
                EstadoDoRbar::DesligadoESuportado => {
                    assert!(suporta_rbar(&relatorio.modelo), "{}", relatorio.modelo)
                }
                EstadoDoRbar::DesligadoSemSuporte => {
                    assert!(!suporta_rbar(&relatorio.modelo), "{}", relatorio.modelo)
                }
                // Placa sem rBAR tem BAR1 preso em 256 MiB: nao existe placa
                // sem suporte com o BAR1 do tamanho da VRAM. Se der "ligado"
                // numa GTX, o numero lido nao e o BAR1 -- que e exatamente o
                // engano das varias linhas `Total` do `nvidia-smi`, aqui pego
                // no fim da linha, com a placa de verdade.
                EstadoDoRbar::Ligado => {
                    assert!(suporta_rbar(&relatorio.modelo), "{}", relatorio.modelo)
                }
                _ => {}
            }
        } else {
            assert_eq!(relatorio.estado, EstadoDoRbar::NaoSei);
        }
    }
}
