// Resizable BAR: só a BIOS liga, e este módulo só LÊ e explica. Duas perguntas: "está ligado?" e "esta placa
// suporta?" (na NVIDIA, só RTX 3000 em diante). A GTX 1650 do dono mostra BAR1 de 256 MiB: "desligado" sozinho
// mandaria procurar na BIOS uma opção que não existe. AMD, driver antigo ou `nvidia-smi` ausente é `NaoSei`.

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
    pub modelo: String,
    pub nota: String,
}

/// O firmware reserva uma fatia de poucos MiB. Não cobre arredondamento de BAR (gigabytes): ver
/// `piso_potencia_de_dois`.
const FOLGA: f64 = 0.90;

/// O BAR tem tamanho em potência de dois: com 12 GiB de VRAM (RTX 3060, 4070) o firmware pode parar em 8192,
/// reprovar na `FOLGA` e dizer "desligado" a quem já ligou. Se arredonda assim é DÚVIDA, não medido; a regra é
/// segura nos dois casos, e sem rBAR o BAR1 fica em 256 MiB, muito abaixo do degrau.
fn piso_potencia_de_dois(vram_mib: u64) -> u64 {
    if vram_mib == 0 {
        return 0;
    }
    1u64 << (u64::BITS - 1 - vram_mib.leading_zeros())
}

pub fn suporta_rbar(modelo: &str) -> bool {
    let m = modelo.to_uppercase();

    if !m.contains("RTX") {
        return false;
    }

    m.split_whitespace()
        .find_map(|palavra| {
            let digitos: String = palavra.chars().filter(|c| c.is_ascii_digit()).collect();
            digitos.chars().next()
        })
        .and_then(|d| d.to_digit(10))
        .map(|serie| serie >= 3)
        .unwrap_or(false)
}

/// PURA: prova os quatro casos sem placa.
pub fn avaliar(bar1_mib: Option<u64>, vram_mib: Option<u64>, modelo: &str) -> EstadoDoRbar {
    let (Some(bar1), Some(vram)) = (bar1_mib, vram_mib) else {
        return EstadoDoRbar::NaoSei;
    };

    // Sem saber a placa não há a segunda pergunta, e responder só a primeira é o defeito que este módulo evita.
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

pub fn nota_de(estado: EstadoDoRbar) -> String {
    match estado {
        EstadoDoRbar::Ligado => "O Resizable BAR está ligado. Nada a fazer aqui.".to_string(),
        // Exige a plataforma inteira: "Above 4G Decoding" existe em placa-mãe muito anterior ao rBAR (RTX 3060 numa
        // Z170 liga, reinicia e nada muda). Detectar exigiria ler o firmware; avisar não exige nada.
        EstadoDoRbar::DesligadoESuportado => {
            "O Resizable BAR está desligado, e a sua placa de vídeo aceita. Ele \
             deixa o processador enxergar a memória da placa de vídeo inteira de \
             uma vez, e costuma render alguns quadros. Para ativar, entre na BIOS \
             e procure por \"Resizable BAR\" ou \"Above 4G Decoding\" — não dá para \
             ligar por programa nenhum. Se você não encontrar essa opção, a sua \
             placa-mãe é anterior a essa tecnologia e não há o que fazer."
                .to_string()
        }
        // Sem a palavra "BIOS" de propósito: a opção não existe para esta placa.
        EstadoDoRbar::DesligadoSemSuporte => {
            "Esta placa de vídeo não tem Resizable BAR — a NVIDIA só passou a \
             oferecer a partir da série RTX 3000. Não há nada para ativar, e isso \
             não é problema no seu PC."
                .to_string()
        }
        // "Placa AMD" é permanente, "driver antigo" tem conserto: juntas, mandam o dono de Radeon atualizar à toa.
        EstadoDoRbar::NaoSei => {
            "Não consegui verificar o Resizable BAR nesta máquina: ainda não \
             cobrimos placas AMD. Isso não quer dizer que esteja certo nem errado."
                .to_string()
        }
    }
}

/// Várias linhas `Total` no relatório: a primeira é a VRAM, e daria rBAR sempre ligado. Só lê DEPOIS de
/// `BAR1 Memory Usage`.
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

        // Só cabeçalho vem sem `:`: sem fechar a seção, uma saída truncada devolveria o `Total : 0 MiB` da memória
        // protegida como BAR1.
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

/// Recebe `sucesso` porque é isso que precisa ser provado: comando que falha pode ter stdout com cara de resposta.
/// Que `ler_do_nvidia_smi` passa `saida.success` de verdade é o canário `chamada_real_passa_o_sucesso_de_verdade`.
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

/// O `nvidia-smi` acompanha o driver: nenhuma dependência nova. Saída fixa em inglês em qualquer idioma.
fn ler_do_nvidia_smi() -> (Option<u64>, Option<u64>, String) {
    // Duas chamadas: o BAR1 só aparece no relatório inteiro (`-q`); modelo e VRAM saem limpos do CSV.
    let identidade = shell::run(
        "nvidia-smi",
        &["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"],
    );

    let (modelo, vram) = match identidade {
        Ok(saida) => identidade_da_saida(saida.success, &saida.stdout),
        // Sem `nvidia-smi` não é erro: é o cliente com AMD.
        Err(_) => return (None, None, String::new()),
    };

    if modelo.is_empty() && vram.is_none() {
        return (None, None, String::new());
    }

    let bar1 = shell::run("nvidia-smi", &["-q"])
        .ok()
        .filter(|s| s.success)
        .and_then(|s| bar1_da_saida(&s.stdout));

    (bar1, vram, modelo)
}

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
        // O defeito que a máquina do dono revelou: GTX 1650, BAR1 de 256 MiB, e a placa não tem rBAR.
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
        // VRAM zero é lixo do `nvidia-smi`: sem a guarda, 0 >= 0 anunciaria "ligado".
        assert_eq!(
            avaliar(Some(0), Some(0), "NVIDIA GeForce RTX 3060"),
            EstadoDoRbar::NaoSei
        );
    }

    #[test]
    fn a_fronteira_da_folga_e_presa_dos_dois_lados() {
        // Prende a `FOLGA`: 8192 * 0,90 = 7372,8, um MiB de cada lado. Baixá-la para 0,50 passaria meia janela como
        // ligada.
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
        assert_eq!(
            avaliar(Some(8192), Some(12288), "NVIDIA GeForce RTX 3060"),
            EstadoDoRbar::Ligado
        );
        assert_eq!(
            avaliar(Some(8192), Some(10240), "NVIDIA GeForce RTX 3080"),
            EstadoDoRbar::Ligado
        );

        assert_eq!(
            avaliar(Some(256), Some(12288), "NVIDIA GeForce RTX 3060"),
            EstadoDoRbar::DesligadoESuportado
        );
        assert_eq!(
            avaliar(Some(256), Some(4096), "NVIDIA GeForce GTX 1650"),
            EstadoDoRbar::DesligadoSemSuporte
        );
        assert_eq!(piso_potencia_de_dois(8192), 8192);
        assert_eq!(piso_potencia_de_dois(12288), 8192);
        assert_eq!(piso_potencia_de_dois(10240), 8192);
        assert_eq!(piso_potencia_de_dois(0), 0);
    }

    #[test]
    fn identidade_de_comando_que_falhou_nao_vira_placa() {
        assert_eq!(
            identidade_da_saida(false, "NVIDIA GeForce RTX 4090, 24576"),
            (String::new(), None)
        );
        assert_eq!(
            identidade_da_saida(true, "NVIDIA GeForce RTX 4090, 24576"),
            ("NVIDIA GeForce RTX 4090".to_string(), Some(24576))
        );
        assert_eq!(identidade_da_saida(true, ""), (String::new(), None));
        assert_eq!(identidade_da_saida(true, "erro"), (String::new(), None));
    }

    #[test]
    fn secao_do_bar1_truncada_nao_pega_numero_da_seguinte() {
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
        assert!(!suporta_rbar("NVIDIA GeForce RTX 2060"));

        // Sem exigir "RTX", uma Radeon RX 7900 leria 7 e seria dada como suportada.
        assert!(!suporta_rbar("AMD Radeon RX 7900 XTX"));
        assert!(!suporta_rbar("Intel Arc A770"));
    }

    #[test]
    fn a_nota_nunca_manda_mexer_na_bios_sem_suporte() {
        // CANÁRIO DE TELA: a frase não pode mandar procurar na BIOS o que não existe para a placa.
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
        // Quatro estados, quatro frases: repetir texto faria "não sei" sair com cara de "ligado".
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

        assert!(!frases[0].to_lowercase().contains("não consegui"));
        assert!(frases[3].to_lowercase().contains("não consegui"));
    }

    #[test]
    fn a_leitura_do_bar1_nao_pega_a_memoria_da_placa_por_engano() {
        // Saída real da máquina do dono, encurtada.
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

        assert_eq!(relatorio.nota, nota_de(relatorio.estado));
        assert!(!relatorio.nota.trim().is_empty());

        // O veredito precisa ser sobre A PLACA DAQUI.
        if !relatorio.modelo.trim().is_empty() {
            match relatorio.estado {
                EstadoDoRbar::DesligadoESuportado => {
                    assert!(suporta_rbar(&relatorio.modelo), "{}", relatorio.modelo)
                }
                EstadoDoRbar::DesligadoSemSuporte => {
                    assert!(!suporta_rbar(&relatorio.modelo), "{}", relatorio.modelo)
                }
                // Placa sem rBAR tem BAR1 em 256 MiB: "ligado" numa GTX é número de outra linha `Total`.
                EstadoDoRbar::Ligado => {
                    assert!(suporta_rbar(&relatorio.modelo), "{}", relatorio.modelo)
                }
                _ => {}
            }
        } else {
            assert_eq!(relatorio.estado, EstadoDoRbar::NaoSei);
        }
    }

    /// Só o código fora de `mod tests`: senão o `.contains` se acharia a si mesmo.
    fn codigo_fonte_deste_arquivo() -> String {
        let caminho = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("modules")
            .join("windows")
            .join("rbar.rs");
        let fonte = std::fs::read_to_string(&caminho)
            .unwrap_or_else(|e| panic!("não consegui ler {:?}: {}", caminho, e));
        fonte
            .split_once("#[cfg(test)]")
            .map(|(codigo, _testes)| codigo.to_string())
            .unwrap_or(fonte)
    }

    // Amarra `ler_do_nvidia_smi` a `saida.success`: trocar por `true` deixava a suíte verde. Prova fiação literal,
    // não semântica; fechar de vez pede o executor como parâmetro.
    #[test]
    fn chamada_real_passa_o_sucesso_de_verdade() {
        let fonte = codigo_fonte_deste_arquivo();

        assert!(
            fonte.contains("identidade_da_saida(saida.success, &saida.stdout)"),
            "ler_do_nvidia_smi precisa chamar `identidade_da_saida(saida.success, \
             &saida.stdout)` -- passar qualquer outra coisa no lugar de \
             `saida.success` (um `true` fixo, por exemplo) faz um comando que \
             falhou virar identidade de placa, e nenhum teste de unidade pega isso \
             porque a lógica interna de `identidade_da_saida` continua certa"
        );
    }
}
