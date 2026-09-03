// Perda de pacote até o servidor do jogo
//
// POR QUE ISTO É O RECURSO MAIS IMPORTANTE DESTE LANÇAMENTO
//
// Uma rede que engasga sente EXATAMENTE igual a FPS baixo. O carro que
// teleporta, o tiro que não registra, a tela que congela por meio segundo — o
// jogador chama tudo isso de "meu PC é ruim". Então algum cliente que compra
// um otimizador de FPS tem, na verdade, um problema que nunca foi de FPS: ele
// otimiza, mede, não vê diferença nenhuma, e pede reembolso — com razão,
// porque o produto nunca poderia ter resolvido o problema dele.
//
// Medir perda de pacote não é mais uma funcionalidade. É a diferença entre
// "não funcionou" e "não era isso, e aqui está a prova".
//
// `network.rs` já abre dizendo que reduzir ping é mentira: ping é distância
// física mais roteamento, e nada no PC do cliente muda isso. Essa recusa
// continua valendo aqui. O que faltava era medir a PERDA — a causa real da
// travada — e este módulo faz isso, contra o servidor em que o cliente está
// jogando de verdade.
//
// NÃO É O `jitter.rs` DE `modules/jitter.rs`
//
// Aquele mede o atraso do AGENDADOR DO WINDOWS — quanto tempo o sistema
// operacional demora para devolver a CPU a uma thread que pediu para dormir 1
// ms. É engasgo de QUADRO, local, sem rede nenhuma envolvida.
//
// Este módulo mede a variação do TEMPO DE IDA E VOLTA até o servidor —
// jitter de REDE. São dois defeitos diferentes que produzem o mesmo sintoma
// na tela (travada), e por isso os dois precisam existir e não podem ser
// confundidos um com o outro.
//
// QUATRO REGRAS, E POR QUE CADA UMA
//
// 1. NÃO DESCOBRIR O SERVIDOR É UM RESULTADO, NÃO UMA FALHA A ESCONDER.
//    Medir contra um host qualquer e apresentar como "o servidor do jogo"
//    fabricaria um número — exatamente o que este produto existe para não
//    fazer. Ver `AlvoDaMedida` e `decidir_alvo`.
//
// 2. 100% DE PERDA É UMA MEDIÇÃO. ZERO RESPOSTA POR A MEDIÇÃO TER FALHADO NÃO
//    É. Colapsar as duas faria o produto anunciar "sua rede está destruída"
//    quando o defeito era do próprio Otimiza (PowerShell não rodou, JSON não
//    veio, etc.). Ver `Perda` e `resumir`.
//
// 3. NÃO PROMETE MELHORAR. Perda quase sempre é do provedor, do cabo, do
//    Wi-Fi ou do servidor do jogo — nunca do PC. O produto mede e diz ONDE
//    está o problema, porque saber que é o Wi-Fi poupa do cliente uma tarde
//    reinstalando driver e uma otimização paga que nunca ia ajudar.
//
// 4. A TELA PRECISA DIZER, EM PORTUGUÊS CLARO, QUE TRAVADA DE REDE SENTE
//    IGUAL A FPS BAIXO. É essa frase que impede o cliente de achar que foi
//    enganado quando otimiza o PC e o problema continua — porque o problema
//    nunca esteve no PC. Ver `montar_nota`.
//
// COMO SE MEDE, E O PONTO CEGO ESCOLHIDO DE OLHOS ABERTOS
//
// A medição é ICMP (ping), via `Test-Connection` do PowerShell — não o
// `ping.exe` de linha de comando. O motivo é o mesmo já aprendido duas vezes
// neste projeto: `ping.exe` imprime em português no Windows em português
// ("Esgotado o tempo limite do pedido", "Tempo limite do pedido excedido") e
// analisar aquele texto é reconstruir, na marra, uma tradução que muda de
// idioma para idioma. `Test-Connection` devolve OBJETO — `ResponseTime` e
// `StatusCode` — igual a `Get-NetAdapter` já devolve em `network.rs`.
//
// O PONTO CEGO: um servidor FiveM pode ter o ICMP bloqueado no firewall e
// ainda assim responder perfeitamente na porta do jogo. Nesse caso este
// módulo mediria "100% de perda" numa conexão que na verdade está saudável.
// É um ponto cego real, e por isso a nota que acompanha perda total sempre
// avisa dessa possibilidade — ver `montar_nota`. A alternativa (medir a
// própria porta do jogo com um pacote UDP do protocolo do FiveM) exigiria
// falar o protocolo interno do jogo, que muda a cada versão do FXServer; ICMP
// é o que dá para medir com uma ferramenta do sistema, de forma estável, e
// dizendo com todas as letras onde a medição pode enganar.
//
// COMO SE DESCOBRE O SERVIDOR
//
// `gamemode.rs` já sabe achar o jogo aberto e o PID dele — `servidor_do_jogo`
// reaproveita `gamemode::jogo_aberto_com_pid`, sem escrever um segundo
// detector. A partir do PID, `Get-NetTCPConnection -OwningProcess` lista as
// conexões TCP estabelecidas daquele processo; o endereço remoto público
// (não privado, não loopback) é o candidato a servidor.
//
// O QUE FOI VERIFICADO E O QUE NÃO FOI: `Get-NetTCPConnection` filtrando por
// `OwningProcess` e `-State Established` é um cmdlet padrão do Windows desde
// o 8/2012, documentado pela Microsoft, e o padrão `@(...) | Select-Object |
// ConvertTo-Json` é o mesmo já usado (e testado) em `adaptadores()`, em
// `network.rs` — isso foi conferido lendo o código deste projeto. O que NÃO
// foi verificado numa máquina real com FiveM aberto é SE a conexão TCP
// estabelecida do processo do jogo aponta, de fato, para o servidor de jogo
// (FiveM fala HTTP/TCP com o servidor para baixar recursos, no mesmo host e
// porta do jogo — é o que a documentação do Cfx.re descreve) e não para outro
// destino qualquer. Por isso a decisão é conservadora: se sobrar mais de um
// endereço público distinto entre as conexões, ou nenhum, o resultado é "não
// descobri" — ambiguidade nunca vira palpite. Passo 4 do plano pede
// justamente essa verificação na máquina do dono, com o FiveM aberto.

use super::{gamemode, shell};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;

/// Quantas vezes o servidor é sondado numa medição.
///
/// Pouco o bastante para não fazer o cliente esperar (cada tentativa custa
/// no máximo o tempo de um timeout de ping), e o suficiente para a variação
/// entre tentativas (o jitter) significar alguma coisa — com 1 ou 2 amostras
/// não há o que variar.
const AMOSTRAS_PADRAO: u32 = 20;

/// O que a medição decidiu sobre CONTRA QUEM medir.
///
/// Separado da medição em si de propósito: é a Regra 1 do módulo virando
/// tipo. `NaoDescobri` não é um valor de erro escondido dentro de `Option` —
/// é um resultado nomeado, para não ter como o resto do código confundir
/// "não descobri o servidor" com "descobri e a resposta veio vazia".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlvoDaMedida {
    /// O processo do jogo não tem um único destino público confiável entre
    /// as conexões ativas — seja porque não há jogo aberto, seja porque há
    /// mais de um candidato e escolher um seria chutar.
    NaoDescobri,
    /// `"ip:porta"` do servidor, vindo das conexões ativas do jogo.
    Servidor(String),
}

/// Decide o alvo a partir do que `servidor_do_jogo` (ou um teste) forneceu.
///
/// Função pura, sem rede nem processo: é o que permite testar a Regra 1 sem
/// depender de nada estar aberto na máquina.
pub fn decidir_alvo(candidato: Option<String>) -> AlvoDaMedida {
    match candidato {
        Some(endereco) if !endereco.trim().is_empty() => AlvoDaMedida::Servidor(endereco),
        _ => AlvoDaMedida::NaoDescobri,
    }
}

/// A perda de pacote medida — ou a informação de que a medição não rodou.
///
/// Esta é a Regra 2 virando tipo: `NaoMedi` e "100% de perdidos" são
/// variantes diferentes de propósito, para o resto do código nunca poder
/// escrever `perdidos == enviados` como se isso cobrisse os dois casos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tipo")]
pub enum Perda {
    /// A medição rodou. `perdidos` pode ir de 0 até `enviados` — inclusive
    /// igual, e isso É uma medição válida (rede fora do ar, ou ICMP
    /// bloqueado — ver a nota).
    Medida { enviados: u32, perdidos: u32 },
    /// A checagem em si não rodou — nenhum pacote saiu. Não é 0% de perda
    /// nem 100%: é ausência de dado.
    NaoMedi,
    /// Nenhum ping voltou, MAS a porta do jogo aceitou conexão.
    ///
    /// ESTA VARIANTE EXISTE PARA O PRODUTO NÃO MENTIR, E O MOTIVO É CONCRETO.
    ///
    /// Hospedagem de FiveM é alvo conhecido de ataque, e boa parte dos
    /// servidores fica atrás de filtragem anti-DDoS que descarta ICMP e deixa
    /// a porta do jogo passar intacta. Vários firewalls de nuvem também
    /// bloqueiam ICMP por padrão.
    ///
    /// Sem esta resposta, esse servidor — perfeitamente saudável — apareceria
    /// como "20 de 20 perdidos, rede fora do ar". O produto estaria dizendo a
    /// um cliente pagante que a conexão dele está destruída quando ela está
    /// perfeita, e numa versão cujo argumento é medir sem mentir isso seria
    /// pior do que não ter o recurso.
    ///
    /// A prova de que não é perda vem do aperto de mão TCP na porta que a
    /// descoberta já achou: se o serviço com que o jogo fala aceita conexão, o
    /// que não voltou foi só o ping.
    NaoRespondePing { enviados: u32 },
}

/// Resume enviados/recebidos em `Perda`. Pura, testável sem PowerShell.
pub fn resumir(enviados: u32, recebidos: u32) -> Perda {
    if enviados == 0 {
        return Perda::NaoMedi;
    }

    Perda::Medida {
        enviados,
        perdidos: enviados.saturating_sub(recebidos),
    }
}

/// Decide o desfecho quando NENHUM ping voltou, usando a porta como testemunha.
///
/// Pura de propósito: a decisão que separa "sua rede está ruim" de "este
/// servidor não responde a ping" é a mais cara de errar do módulo, e precisa
/// poder ser testada sem rede nenhuma.
///
/// `porta_respondeu`:
///   - `Some(true)`  — o serviço do jogo aceitou conexão. Não é perda.
///   - `Some(false)` — nem ping nem porta. Aí a perda total é real.
///   - `None`        — não deu para tentar a porta (a descoberta não trouxe
///                     uma). Sem testemunha, fica a medição crua do ping.
pub fn avaliar_perda_total(enviados: u32, porta_respondeu: Option<bool>) -> Perda {
    match porta_respondeu {
        Some(true) => Perda::NaoRespondePing { enviados },
        _ => Perda::Medida {
            enviados,
            perdidos: enviados,
        },
    }
}

/// Resultado completo de uma medição, pronto para a tela.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedidaDeRede {
    /// `"ip:porta"` do servidor medido, ou `None` quando a Regra 1 decidiu
    /// não medir contra ninguém.
    pub alvo: Option<String>,
    pub perda: Perda,
    /// Variação do tempo de resposta — jitter de REDE, distinto do jitter de
    /// quadro medido em `modules/jitter.rs`. `None` sem amostras suficientes.
    pub jitter_ms: Option<f64>,
    /// Mediana do tempo de resposta às tentativas que voltaram.
    pub tempo_ms: Option<f64>,
    /// Texto para a tela, em português. Carrega as Regras 3 e 4 sempre, e o
    /// resultado da Regra 1/2 conforme o caso.
    pub nota: String,
}

// ------------------------------------------------------ descoberta do alvo

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct ConexaoBruta {
    remote_address: Option<String>,
    remote_port: Option<u16>,
}

/// Conexões TCP estabelecidas de um processo, pelo PID.
fn conexoes_estabelecidas(pid: u32) -> Vec<ConexaoBruta> {
    let script = format!(
        "ConvertTo-Json -Compress -Depth 3 -InputObject @(Get-NetTCPConnection \
         -OwningProcess {} -State Established -ErrorAction SilentlyContinue | \
         Select-Object RemoteAddress,RemotePort)",
        pid
    );

    shell::powershell(&script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default()
}

/// Verdadeiro para um endereço que pode ser, de fato, um servidor na
/// internet — e falso para o que claramente não é: rede local, loopback,
/// link-local, multicast.
fn e_endereco_publico(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.is_unspecified())
        }
        IpAddr::V6(v6) => {
            !(v6.is_loopback()
                || v6.is_multicast()
                || v6.is_unspecified()
                // fe80::/10, link-local unicast.
                || (v6.segments()[0] & 0xffc0) == 0xfe80)
        }
    }
}

/// O servidor em que o cliente está jogando agora, ou `None`.
///
/// Reaproveita `gamemode::jogo_aberto_com_pid` — não escreve um segundo
/// detector de jogo. Ambiguidade (nenhum endereço público, ou mais de um
/// distinto) vira `None`: é a Regra 1 na prática, escolher não chutar.
pub fn servidor_do_jogo() -> Option<String> {
    let (_, pid) = gamemode::jogo_aberto_com_pid()?;

    let mut candidatos: Vec<(String, u16)> = conexoes_estabelecidas(pid)
        .into_iter()
        .filter_map(|c| {
            let endereco = c.remote_address?;
            let porta = c.remote_port?;
            let ip: IpAddr = endereco.parse().ok()?;

            e_endereco_publico(&ip).then_some((endereco, porta))
        })
        .collect();

    candidatos.sort();
    candidatos.dedup_by(|a, b| a.0 == b.0);

    match candidatos.as_slice() {
        [(ip, porta)] => Some(format!("{}:{}", ip, porta)),
        _ => None,
    }
}

// ---------------------------------------------------------------- medição

#[derive(Debug, Deserialize, Default, Clone, Copy)]
struct RespostaPing {
    ms: Option<f64>,
    ok: Option<bool>,
}

/// Sonda o host com ICMP, uma vez por amostra, via `Test-Connection`.
///
/// Uma tentativa por chamada ao cmdlet — e não `-Count N` de uma vez — para
/// que UMA tentativa falhando nunca derrube as outras: cada `try/catch` do
/// laço é independente, e a lista sempre sai com `amostras` itens (Regra 2:
/// `enviados` vem do tamanho da lista, não de um contador otimista).
fn sondar(host: &str, amostras: u32) -> Vec<RespostaPing> {
    let script = format!(
        "$r = @(); foreach ($i in 1..{}) {{ \
           try {{ $resp = Test-Connection -ComputerName '{}' -Count 1 \
                  -ErrorAction Stop; \
                  $r += [ordered]@{{ ms = [double]$resp.ResponseTime; ok = $true }} }} \
           catch {{ $r += [ordered]@{{ ms = $null; ok = $false }} }} }}; \
         ConvertTo-Json -Compress -InputObject @($r)",
        amostras, host
    );

    shell::powershell(&script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default()
}

fn calcular_perda(respostas: &[RespostaPing]) -> Perda {
    let enviados = respostas.len() as u32;
    let recebidos = respostas.iter().filter(|r| r.ok == Some(true)).count() as u32;
    resumir(enviados, recebidos)
}

/// Mediana dos tempos de resposta que voltaram. Mediana, e não média, pelo
/// mesmo motivo de `network.rs`: uma tentativa isolada e lenta não pode
/// decidir sozinha o número que a tela mostra.
fn calcular_tempo_mediano(respostas: &[RespostaPing]) -> Option<f64> {
    let mut tempos: Vec<f64> = respostas
        .iter()
        .filter(|r| r.ok == Some(true))
        .filter_map(|r| r.ms)
        .collect();

    if tempos.is_empty() {
        return None;
    }

    tempos.sort_by(f64::total_cmp);
    Some(tempos[tempos.len() / 2])
}

/// Jitter de rede: a variação, e não o tamanho, do tempo de resposta.
///
/// Calculado como a média das diferenças absolutas entre tentativas
/// SUCESSIVAS que voltaram — dois pings de 20 ms cada não têm jitter, um de
/// 20 ms seguido de um de 80 ms tem 60 ms de jitter. Precisa de ao menos duas
/// respostas: uma sozinha não tem com o que variar.
fn calcular_jitter(respostas: &[RespostaPing]) -> Option<f64> {
    let tempos: Vec<f64> = respostas
        .iter()
        .filter(|r| r.ok == Some(true))
        .filter_map(|r| r.ms)
        .collect();

    if tempos.len() < 2 {
        return None;
    }

    let diferencas: Vec<f64> = tempos.windows(2).map(|par| (par[1] - par[0]).abs()).collect();
    Some(diferencas.iter().sum::<f64>() / diferencas.len() as f64)
}

/// Monta o texto que acompanha a medição.
///
/// As Regras 3 e 4 vão SEMPRE no texto — não promete melhorar, e diz que
/// travada de rede sente igual a FPS baixo — porque é isso que impede o
/// cliente de achar que o produto o enganou quando ele otimiza o PC e a
/// travada continua.
fn montar_nota(perda: &Perda, sem_alvo: bool) -> String {
    let base = "Travamento na hora de jogar sente exatamente igual a FPS baixo: o carro que \
                teleporta, o tiro que não registra, a tela que congela por um instante. Se você \
                otimizou o PC e não sentiu diferença, pode ser isto aqui — e não o computador. \
                Perda de pacote é quase sempre do provedor, do cabo, do Wi-Fi ou do servidor do \
                jogo, nunca do PC: o Otimiza mede e mostra onde está o problema, sem prometer \
                consertar o que não está aqui.";

    let extra = if sem_alvo {
        "Não descobri, com confiança, o servidor em que você está jogando agora. Medir contra \
         um endereço qualquer e apresentar como \"o servidor do jogo\" seria inventar um número \
         — por isso a medição não rodou. Abra o jogo e tente de novo."
            .to_string()
    } else {
        match perda {
            Perda::NaoMedi => "A checagem não rodou desta vez — não é 0% de perda nem 100%, é \
                que nenhum pacote chegou a sair. Tente de novo."
                .to_string(),
            // A PORTA JÁ RESPONDEU. Isto NÃO é perda, e a frase não pode
            // soar como se fosse: quem lê aqui está com a conexão boa.
            Perda::NaoRespondePing { enviados } => format!(
                "Este servidor não responde a ping — as {} tentativas ficaram sem resposta —, \
                 mas a porta do jogo aceitou conexão normalmente. Ou seja: **não é perda de \
                 pacote**. Bloquear ping é comum em servidor de jogo, por segurança. Não dá \
                 para medir perda contra este servidor, e isso não é problema na sua rede.",
                enviados
            ),
            Perda::Medida { enviados, perdidos } if *perdidos == *enviados => format!(
                "Nenhuma das {} tentativas voltou, e a porta do jogo também não respondeu. \
                 As duas coisas juntas apontam o servidor fora do ar, ou algo entre você e \
                 ele bloqueando a conexão inteira.",
                enviados
            ),
            Perda::Medida { enviados, perdidos } if *perdidos == 0 => format!(
                "Nenhuma perda nas {} tentativas contra o servidor agora — é a foto deste \
                 instante, não garantia de que nunca há perda.",
                enviados
            ),
            Perda::Medida { enviados, perdidos } => format!(
                "{} de {} pacotes não voltaram ({:.0}%) na checagem contra o servidor.",
                perdidos,
                enviados,
                (*perdidos as f64 / *enviados as f64) * 100.0
            ),
        }
    };

    format!("{} {}", base, extra)
}

/// Levantamento completo: decide o alvo (Regra 1) e, se houver um, mede.
pub fn medir(alvo: Option<String>, amostras: u32) -> MedidaDeRede {
    match decidir_alvo(alvo) {
        AlvoDaMedida::NaoDescobri => MedidaDeRede {
            alvo: None,
            perda: Perda::NaoMedi,
            jitter_ms: None,
            tempo_ms: None,
            nota: montar_nota(&Perda::NaoMedi, true),
        },
        AlvoDaMedida::Servidor(destino) => {
            let host = destino.rsplit_once(':').map(|(h, _)| h).unwrap_or(&destino);

            // Defesa em profundidade: só o que já foi validado como IP entra
            // no script do PowerShell. `servidor_do_jogo` só produz IP, mas
            // `medir` aceita `Option<String>` de qualquer chamador.
            if host.parse::<IpAddr>().is_err() {
                return MedidaDeRede {
                    alvo: Some(destino),
                    perda: Perda::NaoMedi,
                    jitter_ms: None,
                    tempo_ms: None,
                    nota: "O endereço do servidor não é um IP válido — a medição não rodou."
                        .to_string(),
                };
            }

            let respostas = sondar(host, amostras.max(1));
            let mut perda = calcular_perda(&respostas);

            // NENHUM PING VOLTOU? PERGUNTE À PORTA ANTES DE ACUSAR A REDE.
            //
            // A descoberta já achou a porta, e o `medir` a jogava fora. Um
            // aperto de mão TCP nela é respondido pelo serviço com que o jogo
            // realmente fala, e NÃO é engolido por regra de firewall que só
            // vale para ICMP — que é o caso comum em servidor de FiveM atrás
            // de filtragem anti-DDoS.
            //
            // Só roda no caso 100%: com qualquer resposta de ping, a medição
            // já se sustenta e uma conexão a mais seria ruído.
            if matches!(&perda, Perda::Medida { enviados, perdidos } if perdidos == enviados) {
                let enviados = respostas.len() as u32;
                perda = avaliar_perda_total(enviados, porta_responde(&destino));
            }

            MedidaDeRede {
                alvo: Some(destino),
                jitter_ms: calcular_jitter(&respostas),
                tempo_ms: calcular_tempo_mediano(&respostas),
                nota: montar_nota(&perda, false),
                perda,
            }
        }
    }
}

/// Quanto se espera pelo aperto de mão TCP antes de desistir.
///
/// Dois segundos: é confirmação, não medição. Se a porta não respondeu nesse
/// tempo, o produto simplesmente não tem a testemunha e volta para a medição
/// crua do ping — esperar mais só atrasaria a tela para o cliente.
const PRAZO_DA_PORTA: Duration = Duration::from_secs(2);

/// Se a porta do jogo aceita conexão.
///
/// `None` quando não há porta para tentar: a descoberta pode devolver só o
/// endereço, e sem porta não existe testemunha — que é diferente de a
/// testemunha ter dito não.
fn porta_responde(destino: &str) -> Option<bool> {
    let (host, porta) = destino.rsplit_once(':')?;
    let porta: u16 = porta.parse().ok()?;
    let ip: IpAddr = host.parse().ok()?;

    Some(TcpStream::connect_timeout(&SocketAddr::new(ip, porta), PRAZO_DA_PORTA).is_ok())
}

/// Medição com a quantidade de amostras padrão do produto.
pub fn medir_agora() -> MedidaDeRede {
    medir(servidor_do_jogo(), AMOSTRAS_PADRAO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sem_alvo_conhecido_nao_mede_contra_qualquer_um() {
        // Medir contra um alvo qualquer e apresentar como "o servidor do jogo" é
        // fabricar um número. Não descobrir é um resultado; inventar não é.
        assert!(matches!(decidir_alvo(None), AlvoDaMedida::NaoDescobri));
        assert!(matches!(
            decidir_alvo(Some("203.0.113.10:30120".into())),
            AlvoDaMedida::Servidor(_)
        ));
    }

    #[test]
    fn ping_bloqueado_nao_e_apresentado_como_perda_de_pacote() {
        // A DECISÃO MAIS CARA DE ERRAR DO MÓDULO.
        //
        // Hospedagem de FiveM é alvo de ataque, e boa parte fica atrás de
        // filtragem anti-DDoS que descarta ICMP e passa a porta do jogo
        // intacta. Sem esta separação, esse servidor — saudável — apareceria
        // como "20 de 20 perdidos, rede fora do ar", e o produto estaria
        // dizendo a um cliente pagante que a conexão dele está destruída
        // quando ela está perfeita.
        assert_eq!(
            avaliar_perda_total(20, Some(true)),
            Perda::NaoRespondePing { enviados: 20 },
            "porta respondendo continuou virando perda total"
        );

        // Nem ping nem porta: aí a perda total é real, e dizer isso é o certo.
        assert_eq!(
            avaliar_perda_total(20, Some(false)),
            Perda::Medida { enviados: 20, perdidos: 20 }
        );

        // Sem porta para tentar não há testemunha — e ausência de testemunha
        // não é testemunho a favor. Fica a medição crua do ping.
        assert_eq!(
            avaliar_perda_total(20, None),
            Perda::Medida { enviados: 20, perdidos: 20 }
        );
    }

    #[test]
    fn a_frase_do_ping_bloqueado_nega_a_perda_em_vez_de_ressalvar() {
        // A ressalva antiga vivia entre parênteses no fim de uma frase que
        // começava com "rede fora do ar" — e quem lê etiqueta vermelha e
        // "20/20 perdidos" não chega ao parêntese. A negação tem que estar na
        // frase, não depois dela.
        let nota = montar_nota(&Perda::NaoRespondePing { enviados: 20 }, false);
        let minuscula = nota.to_lowercase();

        assert!(
            minuscula.contains("não é perda de pacote"),
            "a frase não nega a perda: {}",
            nota
        );
        assert!(
            !minuscula.contains("fora do ar"),
            "a frase ainda fala em rede fora do ar: {}",
            nota
        );
    }

    #[test]
    fn perda_total_nao_se_confunde_com_nao_medido() {
        // 100% de perda é uma medição — e das mais importantes. Zero resposta por
        // falha da própria medição é outra coisa. Colapsar as duas faria o
        // produto anunciar rede destruída quando o defeito era dele.
        assert_eq!(resumir(10, 0), Perda::Medida { enviados: 10, perdidos: 10 });
        assert_eq!(resumir(0, 0), Perda::NaoMedi);
    }

    #[test]
    fn string_vazia_tambem_e_nao_descobri() {
        // Uma string vazia não é um endereço — é o mesmo "nada" que `None`.
        assert!(matches!(decidir_alvo(Some(String::new())), AlvoDaMedida::NaoDescobri));
        assert!(matches!(decidir_alvo(Some("   ".into())), AlvoDaMedida::NaoDescobri));
    }

    #[test]
    fn perda_parcial_e_perda_zero_sao_distintas() {
        assert_eq!(resumir(20, 20), Perda::Medida { enviados: 20, perdidos: 0 });
        assert_eq!(resumir(20, 15), Perda::Medida { enviados: 20, perdidos: 5 });
    }

    #[test]
    fn a_nota_sempre_diz_que_travada_de_rede_sente_igual_a_fps_baixo() {
        // Regra 4: é a frase que impede o cliente de achar que foi enganado.
        // Canário: apagar esta frase da constante `base` derruba este teste.
        for (perda, sem_alvo) in [
            (Perda::NaoMedi, true),
            (Perda::NaoMedi, false),
            (Perda::Medida { enviados: 10, perdidos: 10 }, false),
            (Perda::Medida { enviados: 10, perdidos: 0 }, false),
            (Perda::Medida { enviados: 10, perdidos: 3 }, false),
        ] {
            let nota = montar_nota(&perda, sem_alvo);
            assert!(
                nota.contains("sente exatamente igual a FPS baixo"),
                "faltou a frase que justifica o recurso: {:?} / sem_alvo={}",
                perda,
                sem_alvo
            );
        }
    }

    #[test]
    fn a_nota_nunca_promete_melhorar() {
        // Regra 3: o produto mede e diz onde está — nunca promete consertar.
        for (perda, sem_alvo) in [
            (Perda::NaoMedi, true),
            (Perda::Medida { enviados: 10, perdidos: 10 }, false),
            (Perda::Medida { enviados: 10, perdidos: 0 }, false),
        ] {
            let nota = montar_nota(&perda, sem_alvo);
            assert!(nota.contains("nunca do PC"));
            assert!(nota.contains("sem prometer consertar"));
        }
    }

    #[test]
    fn perda_total_confirmada_pela_porta_nao_ressalva_o_que_nao_ha() {
        // ESTE TESTE MUDOU DE SENTIDO, E A MUDANÇA É A CORREÇÃO.
        //
        // Ele exigia que a nota de 100% carregasse a ressalva "alguns
        // servidores bloqueiam ping" — ou seja, exigia o DEFEITO: a dúvida
        // entre parênteses, no fim de uma frase que começava com "rede fora do
        // ar", ao lado de uma etiqueta vermelha e "20/20 perdidos". Quem lê
        // isso não chega ao parêntese.
        //
        // A dúvida deixou de morar na prosa e passou a morar na RESPOSTA: se o
        // ping não volta, o produto pergunta à porta do jogo antes de concluir
        // qualquer coisa. Quando ela responde, o desfecho é outro
        // (`NaoRespondePing`) e a frase NEGA a perda em vez de ressalvá-la.
        //
        // Então esta nota — perda total COM a porta também muda —, não precisa
        // mais de ressalva: as duas testemunhas disseram a mesma coisa.
        let nota = montar_nota(&Perda::Medida { enviados: 20, perdidos: 20 }, false);

        assert!(
            nota.contains("a porta do jogo também não respondeu"),
            "a nota não diz que a porta foi consultada: {}",
            nota
        );
        assert!(
            !nota.contains("bloqueiam ping"),
            "a ressalva voltou para uma nota que não precisa mais dela: {}",
            nota
        );
    }

    #[test]
    fn nota_sem_alvo_explica_por_que_nao_mediu() {
        let nota = montar_nota(&Perda::NaoMedi, true);
        assert!(nota.contains("Não descobri"));
        assert!(nota.contains("inventar um número"));
    }

    #[test]
    fn medir_sem_alvo_nao_chama_powershell() {
        // Sem alvo, `medir` precisa devolver na hora — sem tentar sondar nada.
        let medida = medir(None, 20);
        assert_eq!(medida.alvo, None);
        assert_eq!(medida.perda, Perda::NaoMedi);
        assert!(medida.jitter_ms.is_none());
        assert!(medida.tempo_ms.is_none());
    }

    #[test]
    fn medir_recusa_alvo_que_nao_e_ip() {
        // Defesa em profundidade: `medir` é a fronteira pública, e só aceita
        // sondar um endereço que já passou por validação de IP.
        let medida = medir(Some("nao-e-um-ip:1234".into()), 5);
        assert_eq!(medida.perda, Perda::NaoMedi);
        assert!(medida.tempo_ms.is_none());
    }

    #[test]
    fn calculo_de_perda_conta_pelo_tamanho_da_lista_e_nao_por_um_contador_separado() {
        let respostas = vec![
            RespostaPing { ms: Some(10.0), ok: Some(true) },
            RespostaPing { ms: None, ok: Some(false) },
            RespostaPing { ms: Some(12.0), ok: Some(true) },
        ];
        assert_eq!(calcular_perda(&respostas), Perda::Medida { enviados: 3, perdidos: 1 });
        assert_eq!(calcular_perda(&[]), Perda::NaoMedi);
    }

    #[test]
    fn mediana_ignora_falhas() {
        let respostas = vec![
            RespostaPing { ms: Some(30.0), ok: Some(true) },
            RespostaPing { ms: None, ok: Some(false) },
            RespostaPing { ms: Some(10.0), ok: Some(true) },
            RespostaPing { ms: Some(20.0), ok: Some(true) },
        ];
        assert_eq!(calcular_tempo_mediano(&respostas), Some(20.0));
        assert_eq!(calcular_tempo_mediano(&[]), None);
    }

    #[test]
    fn jitter_zero_quando_tempo_constante() {
        let respostas = vec![
            RespostaPing { ms: Some(20.0), ok: Some(true) },
            RespostaPing { ms: Some(20.0), ok: Some(true) },
            RespostaPing { ms: Some(20.0), ok: Some(true) },
        ];
        assert_eq!(calcular_jitter(&respostas), Some(0.0));
    }

    #[test]
    fn jitter_mede_a_variacao_entre_tentativas_sucessivas() {
        // 20 -> 80 -> 20: duas variações de 60 ms cada, média 60.
        let respostas = vec![
            RespostaPing { ms: Some(20.0), ok: Some(true) },
            RespostaPing { ms: Some(80.0), ok: Some(true) },
            RespostaPing { ms: Some(20.0), ok: Some(true) },
        ];
        assert_eq!(calcular_jitter(&respostas), Some(60.0));
    }

    #[test]
    fn jitter_precisa_de_ao_menos_duas_respostas() {
        let uma = vec![RespostaPing { ms: Some(20.0), ok: Some(true) }];
        assert_eq!(calcular_jitter(&uma), None);
        assert_eq!(calcular_jitter(&[]), None);
    }

    #[test]
    fn endereco_publico_exclui_privado_loopback_e_link_local() {
        for privado in ["10.0.0.5", "192.168.1.1", "172.16.0.1", "127.0.0.1", "169.254.1.1"] {
            let ip: IpAddr = privado.parse().unwrap();
            assert!(!e_endereco_publico(&ip), "{} deveria ser privado", privado);
        }

        let publico: IpAddr = "203.0.113.10".parse().unwrap();
        assert!(e_endereco_publico(&publico));
    }

    #[test]
    fn medicao_real_desta_maquina() {
        // Sem jogo aberto durante o teste, `servidor_do_jogo` deve devolver
        // `None` e `medir_agora` cair no caminho "não descobri" — sem tentar
        // sondar nada. Com o FiveM aberto (verificação manual do passo 4),
        // deve haver um alvo e uma medição de verdade.
        let medida = medir_agora();
        println!("alvo: {:?}", medida.alvo);
        println!("perda: {:?}", medida.perda);
        println!("jitter: {:?} ms", medida.jitter_ms);
        println!("tempo: {:?} ms", medida.tempo_ms);
        println!("nota: {}", medida.nota);

        assert!(!medida.nota.is_empty());

        if let Perda::Medida { enviados, perdidos } = medida.perda {
            assert!(perdidos <= enviados, "perdidos não pode passar de enviados");
        }
    }
}
