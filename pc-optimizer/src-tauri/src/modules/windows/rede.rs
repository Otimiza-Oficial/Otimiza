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

/// Quanto se espera por UM ping antes de contá-lo como perdido.
///
/// POR QUE ISTO EXISTE, E POR QUE NÃO É MAIS `Test-Connection`. O
/// `Test-Connection` do Windows PowerShell 5.1 — que é o que roda na máquina
/// do cliente — NÃO aceita prazo: o padrão do `Win32_PingStatus` é de cerca de
/// quatro segundos por tentativa. Medido nesta máquina: três tentativas falhas
/// levaram 11,7 s; cinco bem-sucedidas, 245 ms. Vinte amostras contra um
/// servidor com ICMP bloqueado levavam cerca de 78 segundos — e a tela promete
/// "~10 s".
///
/// Pior do que a espera: o script é ASCII e passa pela sessão viva do
/// PowerShell, que é COMPARTILHADA e serializada num mutex sem prazo de
/// leitura. Durante esses 78 s, toda outra análise do produto — saúde,
/// térmico, serviços, boot — ficava parada na fila, e da tela o Otimiza
/// inteiro parecia travado. O caso que mais penalizava é exatamente o que
/// esta medição existe para tratar: servidor de FiveM atrás de filtragem
/// anti-DDoS, que descarta ICMP.
///
/// `System.Net.NetworkInformation.Ping` aceita prazo, acompanha o .NET
/// Framework que já vem no Windows, e devolve `Status` e `RoundtripTime` como
/// CAMPOS — sem depender do texto traduzido do `ping.exe`, que foi a razão de
/// este módulo não usar o `ping.exe` desde o começo.
const PRAZO_DO_PING_MS: u32 = 1000;

/// Depois de quantas tentativas falhas SEGUIDAS o laço para.
///
/// Cinco silêncios seguidos já respondem a pergunta que a sonda faz: o ICMP
/// não está voltando. As quinze tentativas restantes não acrescentariam
/// informação nenhuma, só espera — e é justamente esse caso que a testemunha
/// da porta TCP esclarece logo em seguida. O contador ZERA a cada resposta,
/// então perda parcial (o caso em que cada amostra conta de verdade) roda as
/// vinte normalmente.
const DESISTE_APOS: u32 = 5;

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
    /// PARTE dos pings não voltou, mas a porta do jogo aceitou conexão.
    ///
    /// O irmão parcial de `NaoRespondePing`, e ele existe porque o caso comum
    /// não é o firewall descartar TODO o ICMP — é LIMITAR A TAXA. O servidor
    /// responde a um ping e ignora os dezenove seguintes, por segundo.
    ///
    /// Sem esta variante, a correção do alarme falso só valia no 100% exato:
    /// 19 de 20 perdidos passava direto e a tela dizia "95% de perda" para um
    /// cliente com a conexão perfeita — o mesmo alarme falso, um pacote abaixo
    /// do limiar.
    ///
    /// Note o que esta variante NÃO afirma: ela não diz que a rede está boa.
    /// Diz que, com o ping sendo descartado por regra do servidor, NÃO DÁ PARA
    /// SABER se houve perda real — e nesse caso o produto informa que não
    /// sabe, em vez de escolher a resposta mais assustadora.
    PingLimitado { enviados: u32, perdidos: u32 },
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
pub fn avaliar_perda_total(enviados: u32, perdidos: u32, porta_respondeu: Option<bool>) -> Perda {
    match porta_respondeu {
        // A porta respondeu: o que não voltou foi só o ping. Total vira
        // "não responde a ping"; parcial vira "ping limitado" — em nenhum
        // dos dois o produto pode apresentar o número como perda medida.
        Some(true) if perdidos >= enviados => Perda::NaoRespondePing { enviados },
        Some(true) => Perda::PingLimitado { enviados, perdidos },
        // Sem testemunha, ou testemunha dizendo não: a medição crua vale, e
        // é ela que vai para a tela.
        _ => Perda::Medida { enviados, perdidos },
    }
}

/// A partir de que fração de perda vale a pena consultar a porta.
///
/// Metade. Abaixo disso a medição se sustenta sozinha e um aperto de mão a
/// mais só atrasaria a tela; acima, a hipótese de o servidor estar
/// descartando ICMP por regra passa a ser tão provável quanto a de haver
/// perda real, e o produto não pode escolher a pior das duas sem perguntar.
const PERDA_QUE_PEDE_TESTEMUNHA: f64 = 0.5;

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
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                // fc00::/7, unique-local: o equivalente IPv6 de 192.168.x.x.
                // Faltava, e um `fd00:…` de rede local passava como "público"
                // e virava candidato a servidor do jogo.
                || (v6.segments()[0] & 0xfe00) == 0xfc00)
        }
    }
}

/// Portas que NUNCA são o servidor do jogo, mesmo sendo a única sobrando.
///
/// O cliente do FiveM mantém conexões TCP com a própria Cfx.re além do
/// servidor: o nucleus, a listagem de servidores, o relatório de falha. Todas
/// em HTTP/HTTPS. E o tráfego de jogo é UDP (ENet) — a TCP com o servidor
/// costuma FECHAR depois que os recursos terminam de baixar.
///
/// A consequência, sem este filtro: passados alguns minutos de partida sobra
/// exatamente UMA conexão pública estabelecida, a da Cfx.re em `:443`. A
/// Regra 1 só barrava ambiguidade — dois candidatos ou nenhum —, e o caso de
/// UM candidato ERRADO passava direto. A tela desenhava "Servidor: <ip da
/// Cfx.re>:443" e media vinte pings contra a CDN, apresentando o número como
/// a checagem contra o servidor em que o cliente está jogando. Isso é
/// exatamente o número inventado que o cabeçalho deste módulo diz existir
/// para evitar.
const PORTAS_QUE_NAO_SAO_JOGO: [u16; 5] = [80, 443, 8080, 8443, 3478];

// A CONEXÃO TCP É A ÚNICA FONTE, E ISSO FOI VERIFICADO — NÃO SUPOSTO.
//
// A saída óbvia para o problema acima seria ler o endereço do log do FiveM em
// vez de olhar as conexões. Procurei, no log de uma sessão real de jogo da
// máquina do dono (`%LOCALAPPDATA%\FiveM\FiveM.app\logs\`, 2,5 MB, partida
// inteira), e nas outras fontes que o cliente deixa em disco:
//
//   CitizenFX_log_*.log ....... "Connecting to server...", sem endereço
//                               nenhum. Os únicos números com cara de IP no
//                               arquivo são versões (2.0.9.0, 1.0.53.576).
//   cef_console.txt ........... nada
//   nui-storage/Local Storage . nada
//
// Ou seja: NÃO EXISTE fallback de log a construir. O FiveM simplesmente não
// grava contra quem conectou. Isto está escrito aqui para a próxima pessoa
// que tiver a mesma ideia não gastar o mesmo tempo — e para ninguém escrever
// um leitor de log baseado em blog, que já foi o erro de uma pesquisa
// anterior neste mesmo produto.
//
// Consequência aceita: em partida longa, a checagem diz "não identifiquei o
// servidor" e explica por quê. É menos útil do que medir, e é honesto — o
// contrário seria medir contra a CDN e chamar de servidor do jogo.

/// O servidor em que o cliente está jogando agora, ou `None`.
///
/// Reaproveita `gamemode::jogo_aberto_com_pid` — não escreve um segundo
/// detector de jogo. Ambiguidade (nenhum endereço público, mais de um
/// distinto, ou só serviço de web sobrando) vira `None`: é a Regra 1 na
/// prática, escolher não chutar.
pub fn servidor_do_jogo() -> Option<String> {
    let (_, pid) = gamemode::jogo_aberto_com_pid()?;

    let candidatos: Vec<(String, u16)> = conexoes_estabelecidas(pid)
        .into_iter()
        .filter_map(|c| {
            let endereco = c.remote_address?;
            let porta = c.remote_port?;
            let ip: IpAddr = endereco.parse().ok()?;

            e_endereco_publico(&ip).then_some((endereco, porta))
        })
        .collect();

    escolher_alvo(candidatos)
}

/// Escolhe o único candidato, ou desiste. Pura, testável sem rede.
///
/// O descarte das portas de web mora AQUI, e não na coleta, de propósito: é
/// a regra que decide se existe alvo, e regra que decide precisa ser
/// testável sem rede, sem jogo aberto e sem PowerShell. Deixada na coleta,
/// ela só rodaria na máquina de um cliente com o FiveM ligado — que é onde
/// ninguém está olhando.
///
/// A deduplicação é pelo PAR, e não só pelo endereço. Antes era pelo
/// endereço, sobre a lista já ordenada, o que mantinha silenciosamente a
/// MENOR porta: um servidor com duas conexões no mesmo host (30110 e 30120)
/// fazia o aperto de mão TCP bater na porta errada. Duas portas no mesmo
/// host são ambiguidade honesta — viram `None`.
fn escolher_alvo(candidatos: Vec<(String, u16)>) -> Option<String> {
    let mut candidatos: Vec<(String, u16)> = candidatos
        .into_iter()
        .filter(|(_, porta)| !PORTAS_QUE_NAO_SAO_JOGO.contains(porta))
        .collect();

    candidatos.sort();
    candidatos.dedup();

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
        "$p = New-Object System.Net.NetworkInformation.Ping; \
         $r = @(); $seguidas = 0; \
         foreach ($i in 1..{}) {{ \
           try {{ $resp = $p.Send('{}', {}); \
                  if ($resp.Status -eq 'Success') {{ \
                    $seguidas = 0; \
                    $r += [ordered]@{{ ms = [double]$resp.RoundtripTime; ok = $true }} }} \
                  else {{ $seguidas++; \
                    $r += [ordered]@{{ ms = $null; ok = $false }} }} }} \
           catch {{ $seguidas++; $r += [ordered]@{{ ms = $null; ok = $false }} }}; \
           if ($seguidas -ge {}) {{ break }} }}; \
         $p.Dispose(); \
         ConvertTo-Json -Compress -InputObject @($r)",
        amostras, host, PRAZO_DO_PING_MS, DESISTE_APOS
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
/// As Regras 3 e 4 em texto, e o motivo de serem uma CONSTANTE.
///
/// Havia uma frase no módulo — a de endereço inválido — escrita direto no
/// `MedidaDeRede`, sem passar por aqui. Era a única saída que NÃO carregava
/// estas duas regras, e o teste de canário, que percorre as combinações de
/// `montar_nota`, não a alcançava. O caminho é inalcançável em produção hoje,
/// mas um furo na guarda que existe para impedir a próxima frase solta vale
/// tanto quanto a frase.
const BASE_DA_NOTA: &str =
    "Travamento na hora de jogar sente exatamente igual a FPS baixo: o carro que teleporta, o \
     tiro que não registra, a tela que congela por um instante. Se você otimizou o PC e não \
     sentiu diferença, pode ser isto aqui — e não o computador. Perda de pacote é quase sempre \
     do provedor, do cabo, do Wi-Fi ou do servidor do jogo, nunca do PC: o Otimiza mede e \
     mostra onde está o problema, sem prometer consertar o que não está aqui.";

fn montar_nota(perda: &Perda, sem_alvo: bool) -> String {
    let base = BASE_DA_NOTA;

    let extra = if sem_alvo {
        "Não descobri, com confiança, o servidor em que você está jogando agora. Medir contra \
         um endereço qualquer e apresentar como \"o servidor do jogo\" seria inventar um número \
         — por isso a medição não rodou. Esta checagem funciona melhor logo depois de entrar \
         no servidor: no FiveM, o jogo em si conversa por um caminho que não dá para identificar \
         de fora, e depois de um tempo de partida o endereço deixa de ficar visível aqui. Se o \
         jogo já está aberto há bastante tempo, reconectar ao servidor faz o endereço aparecer \
         de novo."
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
            // MESMO CUIDADO DA VARIANTE ACIMA: a porta respondeu, então o
            // número de perdidos NÃO pode ser apresentado como perda.
            Perda::PingLimitado { enviados, perdidos } => format!(
                "Este servidor respondeu a {} de {} pings, mas aceitou conexão na porta do jogo \
                 normalmente. Limitar a taxa de ping é comum em servidor de jogo, por segurança, \
                 e é a explicação mais provável. **Não dá para dizer, por aqui, se houve perda \
                 de pacote de verdade** — e afirmar que houve seria inventar um número.",
                enviados - perdidos,
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
                    nota: format!(
                        "{} O endereço do servidor não é um IP válido — a medição não rodou.",
                        BASE_DA_NOTA
                    )
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
            // NÃO é só no 100%. O comportamento comum de firewall de
            // hospedagem não é descartar todo o ICMP — é limitar a taxa, e
            // 19 de 20 perdidos passava direto acusando "95% de perda" numa
            // rede boa. Metade já pede a testemunha.
            if let Perda::Medida { enviados, perdidos } = perda {
                if enviados > 0
                    && f64::from(perdidos) / f64::from(enviados) >= PERDA_QUE_PEDE_TESTEMUNHA
                {
                    perda = avaliar_perda_total(enviados, perdidos, porta_responde(&destino));
                }
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
            avaliar_perda_total(20, 20, Some(true)),
            Perda::NaoRespondePing { enviados: 20 },
            "porta respondendo continuou virando perda total"
        );

        // Nem ping nem porta: aí a perda total é real, e dizer isso é o certo.
        assert_eq!(
            avaliar_perda_total(20, 20, Some(false)),
            Perda::Medida { enviados: 20, perdidos: 20 }
        );

        // Sem porta para tentar não há testemunha — e ausência de testemunha
        // não é testemunho a favor. Fica a medição crua do ping.
        assert_eq!(
            avaliar_perda_total(20, 20, None),
            Perda::Medida { enviados: 20, perdidos: 20 }
        );
    }

    #[test]
    fn ping_limitado_por_taxa_tambem_nao_vira_perda_de_pacote() {
        // O alarme falso um pacote abaixo do limiar. Firewall de hospedagem
        // raramente descarta TODO o ICMP — o comum é limitar a taxa: responde
        // a um ping e ignora os dezenove seguintes. A correção anterior só
        // valia no 100% exato, então 19 de 20 saía como "95% de perda" para
        // um cliente com a conexão perfeita.
        assert_eq!(
            avaliar_perda_total(20, 19, Some(true)),
            Perda::PingLimitado {
                enviados: 20,
                perdidos: 19
            },
            "perda parcial com a porta respondendo continuou virando perda medida"
        );

        // E a frase precisa DIZER que não sabe, em vez de escolher a pior
        // das duas leituras.
        let nota = montar_nota(
            &Perda::PingLimitado {
                enviados: 20,
                perdidos: 19,
            },
            false,
        );
        assert!(
            nota.contains("não dá para dizer") || nota.contains("Não dá para dizer"),
            "a nota precisa admitir que não sabe: {}",
            nota
        );

        // Sem a testemunha, a mesma perda parcial continua sendo medição
        // crua — ausência de testemunha não é testemunho a favor.
        assert_eq!(
            avaliar_perda_total(20, 19, None),
            Perda::Medida {
                enviados: 20,
                perdidos: 19
            }
        );
    }

    #[test]
    fn servico_de_web_nao_e_confundido_com_o_servidor_do_jogo() {
        // O CASO REAL, E ELE É O MAIS PROVÁVEL EM PARTIDA EM ANDAMENTO.
        //
        // O tráfego de jogo do FiveM é UDP; a conexão TCP com o servidor
        // costuma fechar depois do download de recursos. Passados alguns
        // minutos, a ÚNICA pública estabelecida que sobra é a da Cfx.re em
        // :443. Um candidato só — a Regra 1 antiga aprovava — e a tela media
        // vinte pings contra a CDN apresentando o número como "o servidor do
        // jogo".
        assert_eq!(
            escolher_alvo(vec![("203.0.113.10".to_string(), 443)]).as_deref(),
            None,
            "a CDN da Cfx.re em :443 continuou virando o servidor do jogo"
        );

        // Com o servidor de verdade junto, sobra ele — e só ele.
        assert_eq!(
            escolher_alvo(vec![
                ("203.0.113.10".to_string(), 30120),
                ("203.0.113.10".to_string(), 30120),
            ])
            .as_deref(),
            Some("203.0.113.10:30120")
        );

        // Duas portas no MESMO host é ambiguidade honesta: a dedup antiga era
        // pelo endereço, sobre a lista ordenada, e ficava silenciosamente com
        // a MENOR porta — o aperto de mão TCP batia na porta errada.
        assert_eq!(
            escolher_alvo(vec![
                ("203.0.113.10".to_string(), 30120),
                ("203.0.113.10".to_string(), 30110),
            ]),
            None,
            "duas portas no mesmo host continuaram sendo resolvidas por chute"
        );

        // E o filtro de porta é da descoberta, não do escolher: a lista que
        // chega aqui já veio filtrada, então uma porta de jogo passa.
        assert!(PORTAS_QUE_NAO_SAO_JOGO.contains(&443));
        assert!(!PORTAS_QUE_NAO_SAO_JOGO.contains(&30120));
    }

    #[test]
    fn endereco_ipv6_de_rede_local_nao_e_publico() {
        // O teste vizinho lista cinco endereços — todos IPv4. Ele afirmava
        // sobre a função inteira e nunca tocava o ramo IPv6, onde faltava
        // `fc00::/7`, o equivalente IPv6 de 192.168.x.x.
        let local: IpAddr = "fd00::1".parse().unwrap();
        let outro_local: IpAddr = "fc00::abcd".parse().unwrap();
        let link_local: IpAddr = "fe80::1".parse().unwrap();
        let publico: IpAddr = "2001:db8::1".parse().unwrap();

        assert!(!e_endereco_publico(&local), "fd00::/8 passou como público");
        assert!(
            !e_endereco_publico(&outro_local),
            "fc00::/8 passou como público"
        );
        assert!(!e_endereco_publico(&link_local));
        // Este último não é para virar `false`: documentação ou não, é um
        // unicast global e a função não tem por que recusá-lo.
        assert!(e_endereco_publico(&publico));
    }

    #[test]
    fn a_sonda_desiste_depois_de_silencio_seguido() {
        // Não dá para medir o tempo do PowerShell num teste determinístico,
        // então o que se trava aqui é a INTENÇÃO: as duas constantes que
        // fazem a medição caber no "~10 s" que a tela promete. Vinte
        // tentativas a quatro segundos cada — o `Test-Connection` do
        // PowerShell 5.1, que não aceita prazo — davam cerca de 78 s, e
        // prendiam a sessão compartilhada do PowerShell nesse tempo.
        assert!(
            PRAZO_DO_PING_MS <= 1000,
            "prazo por tentativa acima de 1 s estoura o custo anunciado na tela"
        );
        assert!(
            DESISTE_APOS <= 5 && DESISTE_APOS >= 2,
            "desistir cedo demais confunde perda com bloqueio; tarde demais volta a travar"
        );
        assert!(
            u32::from(DESISTE_APOS) * PRAZO_DO_PING_MS <= 6000,
            "o pior caso — ICMP bloqueado — precisa caber junto com o prazo da porta"
        );

        // E o script precisa usar o Ping do .NET, não o `Test-Connection`:
        // é a diferença entre 5 s e 78 s.
        let script = format!(
            "$p = New-Object System.Net.NetworkInformation.Ping; $p.Send('{}', {})",
            "1.1.1.1", PRAZO_DO_PING_MS
        );
        assert!(script.contains("NetworkInformation.Ping"));
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
            (Perda::NaoRespondePing { enviados: 10 }, false),
            (
                Perda::PingLimitado {
                    enviados: 10,
                    perdidos: 9,
                },
                false,
            ),
        ] {
            let nota = montar_nota(&perda, sem_alvo);
            assert!(
                nota.contains("sente exatamente igual a FPS baixo"),
                "faltou a frase que justifica o recurso: {:?} / sem_alvo={}",
                perda,
                sem_alvo
            );
        }

        // E O CAMINHO QUE NÃO PASSA POR `montar_nota`. Era o furo da guarda:
        // a frase de endereço inválido era escrita direto no `MedidaDeRede`, e
        // este laço nunca a alcançava. Ela é a única saída do módulo que o
        // canário não cobria.
        let fora_do_montar = medir(Some("nao-e-um-ip:30120".to_string()), 1);
        assert!(
            fora_do_montar
                .nota
                .contains("sente exatamente igual a FPS baixo"),
            "a frase de endereço inválido escapou da regra: {}",
            fora_do_montar.nota
        );
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
