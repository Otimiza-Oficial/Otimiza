// Rede
//
// Esta é a área do mercado com mais promessa falsa por metro quadrado. Quase
// todo otimizador vende "reduzir ping", e é preciso dizer com todas as letras:
// ping é distância física mais roteamento. Nenhum ajuste no PC do cliente
// encurta o cabo até o servidor. Quem promete isso está mentindo, e o produto
// não vai entrar nessa fila.
//
// O QUE REALMENTE EXISTE, E O QUE CADA COISA FAZ
//
// Trocar o servidor de DNS acelera a RESOLUÇÃO DE NOMES: o tempo entre pedir
// "servidor-de-rp.com" e receber o endereço numérico. Isso aparece em carregar
// página, em abrir lista de servidores e em baixar arquivo. NÃO aparece no ping
// dentro do jogo, porque depois de conectado a conversa é direta com o IP e o
// DNS não participa mais.
//
// Limpar o cache de DNS resolve um caso específico e real — endereço que mudou
// e o PC continua indo no antigo — e fora dele não faz nada. É inofensivo e
// quase sempre inútil, e o produto diz isso.
//
// A DECISÃO QUE DEFINE O MÓDULO
//
// Em vez de afirmar que um DNS é mais rápido, ele MEDE. Faz consultas reais aos
// servidores candidatos, cronometra cada uma e mostra os números lado a lado.
// Se o DNS que o cliente já usa for o mais rápido, é isso que aparece na tela —
// inclusive quando isso significa não ter nada a vender.

use super::{registry, shell};
use crate::modules::changelog::{ChangeRecord, PreviousValue};
use serde::{Deserialize, Serialize};

const INTERFACES: &str = r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces";

/// Servidores públicos medidos, além do que a máquina já usa.
///
/// Só resolvedores grandes e conhecidos, com política de privacidade pública.
/// A lista é curta de propósito: cada entrada é uma consulta a mais na medição,
/// e mais opção não deixa a escolha melhor.
pub const CANDIDATOS: &[(&str, &str, &str)] = &[
    (
        "cloudflare",
        "Cloudflare",
        "1.1.1.1,1.0.0.1",
    ),
    ("google", "Google", "8.8.8.8,8.8.4.4"),
    ("quad9", "Quad9", "9.9.9.9,149.112.112.112"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsMeasurement {
    pub id: String,
    pub name: String,
    pub servers: String,
    /// Mediana do tempo de resposta, em milissegundos.
    pub median_ms: Option<f64>,
    /// Quantas consultas falharam. Servidor bloqueado pela operadora aparece
    /// aqui, e não como "lento".
    pub failures: usize,
    /// Se este é o que a máquina usa agora.
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adapter {
    pub guid: String,
    pub name: String,
    /// Vazio quando o endereço vem do roteador automaticamente.
    pub dns: String,
    pub automatic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkReport {
    pub adapters: Vec<Adapter>,
    pub measurements: Vec<DnsMeasurement>,
    /// Ganho da melhor opção contra o que está em uso, em milissegundos.
    /// `None` quando não dá para comparar.
    pub gain_ms: Option<f64>,
    pub note: String,
}

// ------------------------------------------------------------- adaptadores

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawAdapter {
    name: Option<String>,
    interface_guid: Option<String>,
}

/// Adaptadores de rede fisicamente presentes e ligados.
///
/// O filtro por `ComponentId` começando em `pci\` ou `usb\` já foi necessário
/// antes neste projeto: sem ele entram WAN Miniport, adaptadores do Hyper-V e
/// o depurador de kernel, que não são placas de rede de ninguém.
fn adaptadores() -> Vec<RawAdapter> {
    let script = "ConvertTo-Json -Compress -Depth 3 -InputObject @(Get-NetAdapter \
                  -Physical -ErrorAction SilentlyContinue | \
                  Where-Object { $_.Status -eq 'Up' } | \
                  Select-Object Name,InterfaceGuid)";

    shell::powershell(script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default()
}

/// DNS configurado à mão para um adaptador, lido do registro.
///
/// Vazio significa que o endereço vem do roteador por DHCP — que é o padrão e
/// não é defeito.
fn dns_do_adaptador(guid: &str) -> String {
    registry::read_text("HKLM", &format!("{}\\{}", INTERFACES, guid), "NameServer")
        .unwrap_or_default()
}

// ---------------------------------------------------------------- medição

/// Domínios usados na medição.
///
/// Três domínios diferentes, e nenhum deles é da nossa infraestrutura: medir
/// contra um domínio só daria um número que depende de um cache específico.
const DOMINIOS: [&str; 3] = ["cloudflare.com", "wikipedia.org", "github.com"];

#[derive(Debug, Deserialize, Default)]
struct RawTempo {
    ms: Option<f64>,
    ok: Option<bool>,
}

/// Cronometra consultas a um servidor de DNS.
///
/// `None` no servidor significa "use o que a máquina já usa", que é o
/// comparativo mais importante da tela.
fn medir(servidor: Option<&str>) -> (Option<f64>, usize) {
    let alvo = match servidor {
        Some(ip) => format!("-Server {} ", ip),
        None => String::new(),
    };

    // `-DnsOnly` e `-NoHostsFile` evitam que o arquivo hosts ou o cache do
    // NetBIOS respondam no lugar do servidor, o que daria um tempo falso.
    let script = format!(
        "$r = @(); foreach ($d in @('{}')) {{ \
           $t = Get-Date; $ok = $true; \
           try {{ Resolve-DnsName -Name $d {} -DnsOnly -NoHostsFile -Type A \
                  -QuickTimeout -ErrorAction Stop | Out-Null }} \
           catch {{ $ok = $false }}; \
           $r += [ordered]@{{ ms = ((Get-Date) - $t).TotalMilliseconds; ok = $ok }} }}; \
         ConvertTo-Json -Compress -InputObject @($r)",
        DOMINIOS.join("','"),
        alvo
    );

    let tempos: Vec<RawTempo> = shell::powershell(&script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default();

    let falhas = tempos.iter().filter(|t| t.ok != Some(true)).count();

    let mut bons: Vec<f64> = tempos
        .iter()
        .filter(|t| t.ok == Some(true))
        .filter_map(|t| t.ms)
        .collect();

    if bons.is_empty() {
        return (None, falhas.max(DOMINIOS.len()));
    }

    // Mediana, e não média: uma consulta que caiu num tempo esquisito não pode
    // decidir sozinha qual servidor o cliente vai usar.
    bons.sort_by(f64::total_cmp);
    (Some(bons[bons.len() / 2]), falhas)
}

/// Monta a frase de resumo a partir dos números medidos.
///
/// É aqui que o módulo se recusa a vender: quando o ganho é pequeno, ele diz
/// que é pequeno.
pub fn montar_nota(ganho: Option<f64>, melhor: Option<&str>) -> String {
    let base = "Trocar o DNS acelera a busca do endereço de um site — carregar página, abrir \
                lista de servidores, começar um download. Não muda o seu ping dentro do jogo: \
                depois de conectado, a conversa é direta com o servidor e o DNS não participa \
                mais. Quem promete reduzir ping com ajuste no PC está vendendo o que não existe.";

    match (ganho, melhor) {
        (Some(g), Some(nome)) if g >= 20.0 => format!(
            "{} Aqui a medição mostrou {:.0} ms de diferença a favor do {}, o que é \
             perceptível ao abrir sites.",
            base, g, nome
        ),
        (Some(g), Some(nome)) if g >= 5.0 => format!(
            "{} A diferença medida foi de {:.0} ms a favor do {} — real, mas pequena.",
            base, g, nome
        ),
        (Some(_), _) | (None, _) => format!(
            "{} Nesta máquina a medição não mostrou ganho que valha a troca: o DNS que você \
             já usa está tão rápido quanto as alternativas.",
            base
        ),
    }
}

/// Levantamento completo.
pub fn analyze() -> NetworkReport {
    let adapters: Vec<Adapter> = adaptadores()
        .into_iter()
        .filter_map(|a| {
            let guid = a.interface_guid?;
            let dns = dns_do_adaptador(&guid);

            Some(Adapter {
                automatic: dns.trim().is_empty(),
                name: a.name.unwrap_or_default(),
                guid,
                dns,
            })
        })
        .collect();

    // O que a máquina usa hoje entra na comparação como qualquer outro. Sem
    // isso não há como dizer se a troca vale.
    let (atual_ms, atual_falhas) = medir(None);

    let mut measurements = vec![DnsMeasurement {
        id: "atual".to_string(),
        name: "O que você usa hoje".to_string(),
        servers: adapters
            .iter()
            .find(|a| !a.automatic)
            .map(|a| a.dns.clone())
            .unwrap_or_else(|| "automático, vindo do roteador".to_string()),
        median_ms: atual_ms,
        failures: atual_falhas,
        current: true,
    }];

    for (id, nome, ips) in CANDIDATOS {
        let primeiro = ips.split(',').next().unwrap_or(ips);
        let (ms, falhas) = medir(Some(primeiro));

        measurements.push(DnsMeasurement {
            id: id.to_string(),
            name: nome.to_string(),
            servers: ips.to_string(),
            median_ms: ms,
            failures: falhas,
            current: false,
        });
    }

    // Melhor alternativa que respondeu a tudo. Servidor com falha não é
    // recomendado por mais rápido que pareça nas consultas que sobraram.
    let melhor = measurements
        .iter()
        .filter(|m| !m.current && m.failures == 0)
        .filter_map(|m| m.median_ms.map(|ms| (m, ms)))
        .min_by(|a, b| a.1.total_cmp(&b.1));

    let gain_ms = match (atual_ms, melhor) {
        (Some(atual), Some((_, melhor_ms))) => Some(atual - melhor_ms),
        _ => None,
    };

    let note = montar_nota(gain_ms, melhor.map(|(m, _)| m.name.as_str()));

    NetworkReport {
        adapters,
        measurements,
        gain_ms,
        note,
    }
}

// ------------------------------------------------------------------- ações

/// Define o DNS de um adaptador, com registro para reversão.
pub fn definir_dns(guid: &str, servidores: &str) -> Result<ChangeRecord, String> {
    if !registry::is_elevated() {
        return Err("Trocar o DNS exige executar como administrador.".to_string());
    }

    // Só endereços da lista conhecida, ou a volta para automático. O comando é
    // exposto por IPC, e apontar o DNS de alguém para um servidor arbitrário é
    // exatamente como se sequestra a navegação de uma máquina.
    let permitido = servidores.trim().is_empty()
        || CANDIDATOS.iter().any(|(_, _, ips)| *ips == servidores);

    if !permitido {
        return Err(
            "Endereço de DNS fora da lista conhecida. O Otimiza só configura resolvedores \
             públicos conhecidos — apontar o DNS para um servidor arbitrário é o mecanismo \
             clássico de sequestro de navegação."
                .to_string(),
        );
    }

    let caminho = format!("{}\\{}", INTERFACES, guid);
    let anterior = registry::read("HKLM", &caminho, "NameServer")
        .unwrap_or(PreviousValue::Absent);

    registry::set_string("HKLM", &caminho, "NameServer", servidores)?;

    // Sem isto a mudança só passa a valer no próximo boot.
    let _ = shell::run("ipconfig", &["/flushdns"]);

    Ok(ChangeRecord::RegistryValue {
        hive: "HKLM".to_string(),
        path: caminho,
        name: "NameServer".to_string(),
        previous: anterior,
    })
}

/// Limpa o cache de resolução de nomes.
///
/// Não é reversível e nem precisa ser: o cache se refaz sozinho na próxima
/// consulta. O texto diz que quase sempre não muda nada, porque é verdade.
pub fn limpar_cache_dns() -> Result<String, String> {
    shell::run_checked("ipconfig", &["/flushdns"])
        .map_err(|e| format!("Não foi possível limpar o cache de DNS: {}", e))?;

    Ok("Cache de nomes limpo. Isso resolve um caso específico — site que mudou de endereço e \
        o PC continuava indo no antigo. Fora desse caso, não muda nada, e é honesto dizer \
        que você provavelmente não vai notar diferença."
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nota_desmente_a_promessa_de_reduzir_ping() {
        // A frase que separa este módulo do resto do mercado. Tem que estar
        // presente em qualquer resultado de medição.
        for ganho in [Some(50.0), Some(8.0), Some(0.5), None] {
            let nota = montar_nota(ganho, Some("Cloudflare"));

            assert!(
                nota.contains("Não muda o seu ping dentro do jogo"),
                "faltou desmentir a promessa de ping com ganho {:?}",
                ganho
            );
            assert!(nota.contains("vendendo o que não existe"));
        }
    }

    #[test]
    fn ganho_pequeno_e_chamado_de_pequeno() {
        let grande = montar_nota(Some(60.0), Some("Cloudflare"));
        assert!(grande.contains("perceptível"));

        let pequeno = montar_nota(Some(8.0), Some("Cloudflare"));
        assert!(pequeno.contains("pequena"));

        // E ganho irrelevante vira "não vale a troca", em vez de virar venda.
        let nenhum = montar_nota(Some(1.0), Some("Cloudflare"));
        assert!(nenhum.contains("não mostrou ganho que valha a troca"));
    }

    #[test]
    fn dns_arbitrario_e_recusado() {
        // Apontar o DNS de uma máquina para um servidor qualquer é o mecanismo
        // clássico de sequestro de navegação. O comando é exposto por IPC e não
        // pode aceitar endereço vindo de fora.
        let erro = definir_dns("{qualquer}", "203.0.113.66").unwrap_err();

        assert!(
            erro.contains("fora da lista conhecida") || erro.contains("administrador"),
            "recusa inesperada: {}",
            erro
        );
    }

    #[test]
    fn candidatos_sao_resolvedores_publicos_conhecidos() {
        assert!(CANDIDATOS.iter().any(|(_, _, ips)| ips.starts_with("1.1.1.1")));
        assert!(CANDIDATOS.iter().any(|(_, _, ips)| ips.starts_with("8.8.8.8")));

        // Todo candidato tem endereço secundário: um resolvedor sozinho deixa a
        // máquina sem internet se ele cair.
        for (_, nome, ips) in CANDIDATOS {
            assert!(ips.contains(','), "{} não tem servidor secundário", nome);
        }
    }

    #[test]
    fn mede_esta_maquina() {
        let r = analyze();

        println!("nota: {}", r.note);
        for m in &r.measurements {
            println!(
                "  {:<22} {:>8}  {}{}",
                m.name,
                m.median_ms
                    .map(|ms| format!("{:.0} ms", ms))
                    .unwrap_or_else(|| "sem resposta".into()),
                m.servers,
                if m.failures > 0 {
                    format!("  ({} falha[s])", m.failures)
                } else {
                    String::new()
                }
            );
        }
        for a in &r.adapters {
            println!(
                "  adaptador: {} — DNS {}",
                a.name,
                if a.automatic { "automático" } else { &a.dns }
            );
        }

        assert!(!r.note.is_empty());
        // O que a máquina usa hoje precisa estar na comparação, senão não há
        // como saber se a troca vale.
        assert!(r.measurements.iter().any(|m| m.current));
        assert_eq!(r.measurements.len(), CANDIDATOS.len() + 1);

        // Tempo negativo seria erro de medição virando recomendação.
        for m in &r.measurements {
            if let Some(ms) = m.median_ms {
                assert!(ms >= 0.0, "{} mediu {} ms", m.name, ms);
            }
        }
    }
}
