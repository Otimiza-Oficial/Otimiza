// O lab de compatibilidade, preenchido pela máquina do cliente
//
// POR QUE ISTO EXISTE
//
// A Fase 4 do roadmap — Intel desktop, Intel notebook, Intel híbrido, AMD
// desktop, AMD notebook, NVIDIA, AMD GPU, Intel GPU — está parada por um motivo
// que nenhum commit resolve: não existem essas máquinas aqui. Há um i3-10100F
// com uma GTX 1650, desktop, Windows 10.
//
// E os labs de compatibilidade do second brain (`229 - Windows Customer
// Compatibility Labs`) são MODELOS COM AS CAIXAS VAZIAS. Eles descrevem um
// protocolo muito bom e não têm uma medição sequer.
//
// A saída não é comprar hardware: é que O CLIENTE JÁ TEM A MÁQUINA. Cada pessoa
// que instala o Otimiza está sentada em cima de exatamente o dado que falta, e o
// produto já lê quase tudo que o protocolo pede.
//
// Este módulo fecha essa distância: gera a nota do lab já preenchida, no formato
// que o vault usa, para o cliente mandar junto da queixa. Uma reclamação vira
// uma evidência.
//
// O QUE ELE NÃO FAZ, E É DE PROPÓSITO:
//
// - não escreve nada na máquina. É relatório, e relatório que altera o objeto
//   medido não é relatório;
// - não inventa classificação. Os sete termos são os do protocolo do vault, e
//   cada um sai de uma leitura, não de um palpite;
// - não leva nada que identifique a pessoa. Mesma regra do `suporte.rs` e do
//   `cabecalho.rs`: descreve uma MÁQUINA, não um dono. O arquivo vai inteiro
//   para o Discord do atendimento.

use super::{cabecalho, essenciais, firmware, hardware, planoenergia};
use planoenergia::{Alimentacao, DesfechoDoPlano, RelatorioDoPlano, StatusDoAjuste};

/// A capacidade sob teste. Hoje só há uma; o campo existe porque o protocolo do
/// vault é por capacidade, e a próxima nota vai ser de outra.
pub const CAPACIDADE: &str = "power plan creation";

/// Em que condição a máquina foi encontrada.
///
/// O vocabulário sai da lista de testes do protocolo — `clean machine`,
/// `already-applied`, `non-admin`, `policy/OEM conflict` — para que as notas
/// geradas por clientes se agrupem com as escritas à mão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condicao {
    MaquinaLimpa,
    JaAplicado,
    /// O plano existe e alguém mexeu nele depois. É a condição das duas notas
    /// que já estão no vault (WIN-COMPAT-0366 e 0677).
    ClienteMudouManualmente,
    SemAdministrador,
}

impl Condicao {
    pub fn termo(&self) -> &'static str {
        match self {
            Condicao::MaquinaLimpa => "clean machine",
            Condicao::JaAplicado => "already-applied",
            Condicao::ClienteMudouManualmente => "customer manually changed setting",
            Condicao::SemAdministrador => "non-admin",
        }
    }
}

/// Os SETE termos que o protocolo permite, e nenhum outro.
///
/// Dois deles — `blocked by policy` e `user/OEM-managed` — o produto descobriu
/// na prática antes de saber que o vault já tinha nome para eles, e estava
/// jogando os dois no mesmo balde de "falhou". Nomes diferentes porque são
/// conversas diferentes com o cliente: um é a empresa dele mandando, o outro é a
/// imagem de Windows que ele instalou.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classificacao {
    Verified,
    Partial,
    Unsupported,
    BlockedByPolicy,
    /// NENHUM AJUSTE DO PLANO DE ENERGIA EXIGE REINÍCIO, então esta variante
    /// hoje não é construída — e isso é informação, não sobra.
    ///
    /// Ela fica porque o vocabulário é do PROTOCOLO, não desta capacidade: a
    /// próxima nota de lab vai ser de agendamento de GPU por hardware ou de
    /// VBS, e as duas só valem depois de reiniciar. Remover agora obrigaria a
    /// próxima capacidade a reinventar o termo, provavelmente com outro nome —
    /// e notas que não usam as mesmas palavras não se agrupam.
    #[allow(dead_code)]
    RequiresRestartOrLogoff,
    Failed,
    UserOrOemManaged,
}

impl Classificacao {
    pub fn termo(&self) -> &'static str {
        match self {
            Classificacao::Verified => "verified",
            Classificacao::Partial => "partial",
            Classificacao::Unsupported => "unsupported",
            Classificacao::BlockedByPolicy => "blocked by policy",
            Classificacao::RequiresRestartOrLogoff => "requires restart/logoff",
            Classificacao::Failed => "failed",
            Classificacao::UserOrOemManaged => "user/OEM-managed",
        }
    }
}

/// O que o protocolo chama de "machine class": o rótulo que agrupa notas de
/// máquinas parecidas.
///
/// Segue a forma das notas que já existem no vault — "64GB workstation",
/// "AMD Ryzen laptop" —, e é só isso: um rótulo. Nenhuma decisão do produto sai
/// daqui.
pub fn classe_da_maquina(
    ram_gb: f64,
    notebook: bool,
    fabricante: planoenergia::FabricanteDaCpu,
) -> String {
    use planoenergia::FabricanteDaCpu as F;

    let forma = if notebook { "laptop" } else { "desktop" };

    // 64 GB e acima já não é "desktop", é estação de trabalho — e o vault usa
    // esse nome. Abaixo disso, o fabricante diz mais do que a memória.
    if !notebook && ram_gb >= 60.0 {
        return format!("{}GB workstation", arredondar_ram(ram_gb));
    }

    let marca = match fabricante {
        F::Intel => "Intel",
        F::Amd => "AMD Ryzen",
        F::Outro => "other-CPU",
    };

    format!("{} {} ({}GB)", marca, forma, arredondar_ram(ram_gb))
}

/// A memória como o dono da máquina a conhece.
///
/// O Windows relata menos do que está instalado — parte fica com o vídeo
/// integrado e com o firmware. 7,9 GB são 8 GB para quem comprou, e um rótulo
/// que diz "7GB" faz o cliente achar que falta um pente.
pub fn arredondar_ram(gb: f64) -> u32 {
    const COMUNS: &[u32] = &[2, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256];

    COMUNS
        .iter()
        .copied()
        .find(|&c| gb <= c as f64 + 0.5)
        .unwrap_or_else(|| gb.round() as u32)
}

/// Em que condição esta máquina foi encontrada. Função pura.
pub fn condicao(
    elevado: bool,
    plano_existia: bool,
    ha_ajuste_fora_do_alvo: bool,
) -> Condicao {
    if !elevado {
        return Condicao::SemAdministrador;
    }

    match (plano_existia, ha_ajuste_fora_do_alvo) {
        // O plano é nosso e alguém mexeu: é exatamente a condição das duas notas
        // que já estão no vault.
        (true, true) => Condicao::ClienteMudouManualmente,
        (true, false) => Condicao::JaAplicado,
        (false, _) => Condicao::MaquinaLimpa,
    }
}

/// A classificação, pelos sete termos. Função pura.
///
/// A ORDEM É A REGRA, e ela vai do mais específico para o mais geral: uma
/// máquina gerenciada por OEM que também tem um ajuste sem suporte é uma nota
/// sobre a gerência, não sobre o ajuste — o segundo é consequência do primeiro.
pub fn classificar(
    desfecho: DesfechoDoPlano,
    gerenciada_por_oem: bool,
    houve_falha_de_verificacao: bool,
    houve_falha_de_aplicacao: bool,
    ha_sem_suporte: bool,
) -> Classificacao {
    if gerenciada_por_oem {
        return Classificacao::UserOrOemManaged;
    }

    // O COMANDO FOI ACEITO E O VALOR NÃO FICOU. É a assinatura de alguém
    // reescrevendo a chave por baixo — política de domínio, na esmagadora
    // maioria. Distinguir de `failed` importa: aqui não há nada a consertar no
    // produto, e há uma conversa a ter com quem administra a máquina.
    if houve_falha_de_verificacao {
        return Classificacao::BlockedByPolicy;
    }

    if houve_falha_de_aplicacao || desfecho == DesfechoDoPlano::Falhou {
        return Classificacao::Failed;
    }

    if ha_sem_suporte {
        return Classificacao::Unsupported;
    }

    match desfecho {
        DesfechoDoPlano::Sucesso => Classificacao::Verified,
        _ => Classificacao::Partial,
    }
}

/// Tudo que a nota imprime. Nada aqui lê o sistema.
pub struct Lab {
    /// A data, em `AAAA-MM-DD`. Entra no `Lab` em vez de ser lida dentro do
    /// `montar` para a montagem continuar pura e testável.
    pub data: String,
    pub classe: String,
    pub condicao: Condicao,
    pub classificacao: Classificacao,
    pub cabecalho: cabecalho::Dados,
    pub alimentacao: Alimentacao,
    pub armazenamento: hardware::StorageKind,
    pub vbs_rodando: Option<bool>,
    pub fabricante_da_imagem: Option<String>,
    pub modelo_da_imagem: Option<String>,
    pub essenciais_desativados: usize,
    pub relatorio: RelatorioDoPlano,
    /// Quais testes do protocolo esta execução de fato exerceu.
    pub testes: Vec<(&'static str, bool)>,
}

fn sim_nao_nao_sei(v: Option<bool>) -> &'static str {
    match v {
        Some(true) => "yes",
        Some(false) => "no",
        None => "could not read",
    }
}

fn valor(v: Option<u32>) -> String {
    match v {
        Some(v) => v.to_string(),
        None => "-".to_string(),
    }
}

/// Um identificador estável para esta máquina, derivado do hardware.
///
/// NÃO IDENTIFICA A PESSOA: sai do modelo do processador, da memória e do build
/// do Windows — três coisas que milhares de máquinas compartilham. Serve para
/// que dois relatórios do MESMO cliente caiam no mesmo id e dê para ver que é a
/// mesma máquina antes e depois, sem saber quem ela é.
pub fn identificador(d: &cabecalho::Dados) -> String {
    // FNV-1a, 32 bits. Não é criptografia e não precisa ser: é um rótulo.
    let semente = format!(
        "{}|{}|{:.0}|{}",
        d.maquina.cpu, d.gpu, d.ram_gb, d.build
    );

    let mut hash: u32 = 0x811c_9dc5;

    for byte in semente.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }

    format!("{:08x}", hash)
}

/// Monta a nota, no formato das que já estão em
/// `229 - Windows Customer Compatibility Labs`. Função pura.
pub fn montar(lab: &Lab) -> String {
    let d = &lab.cabecalho;
    let mut s = String::new();

    // O NÚMERO NÃO É INVENTADO. A série do vault (0366, 0677) é numerada à mão,
    // e escolher um número aqui colidiria com o próximo que alguém criar. Data
    // mais identificador da máquina não colide, diz quando foi medido, e a
    // forma `AAAA-MM-DD-xxxxxxxx` não se confunde com a série de quatro dígitos.
    s.push_str("---\ntype: windows-compatibility-lab\nproject: OTIMIZA\n");
    s.push_str("source: generated-by-otimiza\n");
    s.push_str(&format!("otimiza-version: {}\n", d.versao));
    s.push_str(&format!("measured: {}\n---\n\n", lab.data));

    s.push_str(&format!(
        "# WIN-COMPAT-{}-{} - {} - {} - {}\n\n",
        lab.data,
        identificador(d),
        lab.classe,
        CAPACIDADE,
        lab.condicao.termo()
    ));

    s.push_str("> Gerada pelo Otimiza na máquina de um cliente. Nada foi alterado\n");
    s.push_str("> para produzir este relatório. Renumere para a série WIN-COMPAT ao\n");
    s.push_str("> promover.\n\n");

    s.push_str("## Machine class\n\n");
    s.push_str(&format!("**{}**\n\n", lab.classe));
    s.push_str(&format!("Capability under test: **{}**\n\n", CAPACIDADE));
    s.push_str(&format!("Condition: **{}**\n\n", lab.condicao.termo()));

    // ── Detect ────────────────────────────────────────────────────────────
    s.push_str("## Detect\n\n");

    let campo = |s: &mut String, k: &str, v: String| {
        s.push_str(&format!("- **{}**: {}\n", k, v));
    };

    campo(&mut s, "Windows", format!("{} {}", d.windows, d.edicao));
    campo(&mut s, "Build", d.build.clone());
    campo(&mut s, "CPU", format!("{} ({:?})", d.maquina.cpu, d.maquina.fabricante_da_cpu));
    campo(
        &mut s,
        "Cores",
        format!(
            "{} physical / {} logical",
            d.nucleos_fisicos
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".into()),
            d.maquina.nucleos_logicos
        ),
    );
    campo(&mut s, "GPU", d.gpu.clone());
    campo(&mut s, "Storage (system)", format!("{:?}", lab.armazenamento));
    campo(&mut s, "RAM", format!("{:.1} GB", d.ram_gb));
    campo(&mut s, "OS architecture", d.arquitetura_do_windows.clone());
    campo(&mut s, "Process bitness", d.arquitetura_do_processo.to_string());
    campo(&mut s, "Elevated", sim_nao_nao_sei(Some(d.elevado)).to_string());
    campo(
        &mut s,
        "Form factor",
        if d.maquina.notebook { "laptop" } else { "desktop" }.to_string(),
    );
    campo(&mut s, "Battery present", sim_nao_nao_sei(Some(d.maquina.tem_bateria)).to_string());
    campo(
        &mut s,
        "Power source now",
        match lab.alimentacao {
            Alimentacao::Tomada => "AC",
            Alimentacao::Bateria => "battery",
            Alimentacao::NaoSei => "could not read",
        }
        .to_string(),
    );
    campo(&mut s, "Modern Standby", sim_nao_nao_sei(Some(d.maquina.modern_standby)).to_string());
    campo(&mut s, "VBS running", sim_nao_nao_sei(lab.vbs_rodando).to_string());

    // "relevant OEM or policy ownership" — o item do protocolo que só esta
    // máquina sabe responder, e o que separa "o produto falhou" de "esta imagem
    // de Windows não é a da Microsoft".
    campo(
        &mut s,
        "OEM/image owner",
        match (&lab.fabricante_da_imagem, &lab.modelo_da_imagem) {
            (Some(f), Some(m)) => format!("{} / {}", f, m),
            (Some(f), None) => f.clone(),
            (None, Some(m)) => m.clone(),
            (None, None) => "not declared".to_string(),
        },
    );
    campo(
        &mut s,
        "Essential services disabled",
        lab.essenciais_desativados.to_string(),
    );
    campo(
        &mut s,
        "Active power plan",
        format!(
            "{} ({})",
            d.nome_do_plano_ativo.clone().unwrap_or_else(|| "?".into()),
            d.plano_ativo.clone().unwrap_or_else(|| "?".into())
        ),
    );

    // ── Read before / Read after ──────────────────────────────────────────
    s.push_str("\n## Read before / Read after\n\n");
    s.push_str("| Setting | Supported | AC before | AC target | AC after | DC before | DC target | DC after | Status |\n");
    s.push_str("|---|---|---|---|---|---|---|---|---|\n");

    for a in &lab.relatorio.ajustes {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {:?} |\n",
            a.nome,
            if a.suportado { "yes" } else { "no" },
            valor(a.ac_antes),
            valor(a.ac_alvo),
            valor(a.ac_depois),
            valor(a.dc_antes),
            valor(a.dc_alvo),
            valor(a.dc_depois),
            a.status
        ));
    }

    // ── Classify ──────────────────────────────────────────────────────────
    s.push_str("\n## Classify\n\n");
    s.push_str(&format!("**{}**\n\n", lab.classificacao.termo()));
    s.push_str(&format!(
        "- applied and verified: {}\n- already correct: {}\n- unsupported on this Windows: {}\n- did not take: {}\n",
        lab.relatorio.aplicados,
        lab.relatorio.ja_estavam_bons,
        lab.relatorio.nao_suportados,
        lab.relatorio.falhas
    ));

    // ── Tests ─────────────────────────────────────────────────────────────
    s.push_str("\n## Tests\n\n");

    for (nome, feito) in &lab.testes {
        s.push_str(&format!("- [{}] {}\n", if *feito { "x" } else { " " }, nome));
    }

    s.push_str("\n## OTIMIZA principle\n\n");
    s.push_str(
        "A tweak is not \"successful\" because the command ran. It is successful only when \
         the intended state is verified and the machine remains inside safety/performance \
         guardrails.\n",
    );

    s.push_str("\n## Related\n\n");
    s.push_str("- [[10 - Projects/OTIMIZA/Power Plan OTIMIZA]]\n");
    s.push_str("- [[40 - Decisions/ADR/ADR-0001 - Verificar estado após otimização]]\n");
    s.push_str("- [[229 - Windows Customer Compatibility Labs/WIN-COMPAT-0366 - 64GB workstation - power plan creation - customer manually changed setting]]\n");

    s
}

/// Lê a máquina e monta o lab. NÃO ESCREVE NADA: a montagem do plano roda em
/// modo de simulação.
pub fn gerar() -> Result<String, String> {
    let dados = cabecalho::coletar();

    // Simulação: lê o estado de cada ajuste sem tocar em nada. Numa máquina onde
    // o cliente JÁ aplicou, ela lê os valores reais de depois — que é o que
    // torna a coluna "AC after" verdadeira sem precisar reaplicar.
    let relatorio = planoenergia::montar(true, false)?;

    let checagem = essenciais::checar();

    let fora_do_alvo = relatorio
        .ajustes
        .iter()
        .any(|a| a.status == StatusDoAjuste::Mudaria);

    let falha_de_verificacao = relatorio
        .ajustes
        .iter()
        .any(|a| a.status == StatusDoAjuste::FalhouNaVerificacao);

    let falha_de_aplicacao = relatorio
        .ajustes
        .iter()
        .any(|a| a.status == StatusDoAjuste::FalhouAoAplicar);

    let sem_suporte = relatorio.nao_suportados > 0;

    // GERENCIADA POR OEM não é "tem fabricante declarado": todo PC de marca
    // declara um. O sinal é o fabricante declarado JUNTO de serviços essenciais
    // desligados — que é uma imagem modificada, não um Windows de fábrica.
    let gerenciada_por_oem = checagem.fabricante.is_some() && checagem.desativados > 0;

    let condicao = condicao(dados.elevado, relatorio.plano_existia, fora_do_alvo);

    let classificacao = classificar(
        relatorio.desfecho,
        gerenciada_por_oem,
        falha_de_verificacao,
        falha_de_aplicacao,
        sem_suporte,
    );

    let lab = Lab {
        data: chrono::Local::now().format("%Y-%m-%d").to_string(),
        classe: classe_da_maquina(
            dados.ram_gb,
            dados.maquina.notebook,
            dados.maquina.fabricante_da_cpu,
        ),
        condicao,
        classificacao,
        alimentacao: planoenergia::alimentacao(),
        armazenamento: hardware::profile().system_storage,
        vbs_rodando: firmware::vbs_running(),
        fabricante_da_imagem: checagem.fabricante.clone(),
        modelo_da_imagem: checagem.modelo.clone(),
        essenciais_desativados: checagem.desativados,
        // As caixas marcadas dizem o que ESTA execução exerceu, e só isso. Um
        // relatório que marca tudo é um relatório que não vale nada.
        testes: vec![
            ("clean machine", condicao == Condicao::MaquinaLimpa),
            ("already-applied", condicao == Condicao::JaAplicado),
            ("non-admin if relevant", !dados.elevado),
            ("unsupported hardware/OS", sem_suporte),
            ("policy/OEM conflict", gerenciada_por_oem || falha_de_verificacao),
            // Estes três exigem aplicar, reiniciar e desfazer de verdade. Uma
            // leitura nunca os exerce, e marcá-los seria mentir no campo que o
            // protocolo criou justamente para não deixar mentir.
            ("restart persistence", false),
            ("rollback", false),
            ("app crash mid-operation", false),
        ],
        relatorio,
        cabecalho: dados,
    };

    Ok(montar(&lab))
}

#[cfg(test)]
mod tests {
    use super::*;
    use planoenergia::FabricanteDaCpu as F;

    /// Imprime a nota desta máquina. Só lê.
    ///
    ///   cargo test --lib labcompat -- --ignored --nocapture
    #[test]
    #[ignore]
    fn nota_desta_maquina() {
        match gerar() {
            Ok(nota) => println!("\n{}", nota),
            Err(e) => println!("FALHOU: {}", e),
        }
    }

    #[test]
    fn a_classe_segue_os_nomes_que_o_vault_ja_usa() {
        // As duas notas existentes chamam "64GB workstation" e "AMD Ryzen
        // laptop". Gerar "Intel desktop (64GB)" para a primeira faria as notas
        // do cliente não se agruparem com as escritas à mão — que é o motivo
        // inteiro de existir um rótulo de classe.
        assert_eq!(classe_da_maquina(63.8, false, F::Intel), "64GB workstation");
        assert_eq!(
            classe_da_maquina(15.8, true, F::Amd),
            "AMD Ryzen laptop (16GB)"
        );
        assert_eq!(
            classe_da_maquina(7.9, false, F::Intel),
            "Intel desktop (8GB)"
        );
    }

    #[test]
    fn a_memoria_e_arredondada_para_o_que_o_dono_comprou() {
        // O Windows relata menos do que está instalado: parte fica com o vídeo
        // integrado e com o firmware. "7GB" faz o cliente achar que falta pente.
        assert_eq!(arredondar_ram(7.9), 8);
        assert_eq!(arredondar_ram(15.8), 16);
        assert_eq!(arredondar_ram(31.7), 32);
        assert_eq!(arredondar_ram(63.8), 64);
        assert_eq!(arredondar_ram(3.9), 4);
    }

    #[test]
    fn plano_existente_com_ajuste_fora_do_alvo_e_mudanca_manual() {
        // É a condição das duas notas que já estão no vault, e o produto agora
        // sabe reconhecê-la sozinho.
        assert_eq!(
            condicao(true, true, true),
            Condicao::ClienteMudouManualmente
        );
        assert_eq!(condicao(true, true, false), Condicao::JaAplicado);
        assert_eq!(condicao(true, false, false), Condicao::MaquinaLimpa);
    }

    #[test]
    fn sem_administrador_vence_tudo() {
        // Sem elevação nada foi de fato exercido, e qualquer outra condição
        // seria uma conclusão tirada de uma leitura que não aconteceu.
        assert_eq!(condicao(false, true, true), Condicao::SemAdministrador);
        assert_eq!(condicao(false, false, false), Condicao::SemAdministrador);
    }

    #[test]
    fn comando_aceito_com_valor_que_nao_ficou_e_politica_e_nao_falha() {
        // A distinção que o produto não tinha e o vault já nomeava. Importa
        // porque não há nada a consertar no Otimiza: há uma conversa a ter com
        // quem administra a máquina.
        assert_eq!(
            classificar(DesfechoDoPlano::EmParte, false, true, false, false),
            Classificacao::BlockedByPolicy
        );
    }

    #[test]
    fn imagem_modificada_vence_o_resto() {
        // Uma máquina com imagem de terceiros e um ajuste sem suporte é uma nota
        // sobre a imagem: o segundo é consequência do primeiro, e classificar
        // como "unsupported" mandaria o atendimento procurar no lugar errado.
        assert_eq!(
            classificar(DesfechoDoPlano::EmParte, true, false, false, true),
            Classificacao::UserOrOemManaged
        );
    }

    #[test]
    fn os_sete_termos_sao_os_do_protocolo() {
        // Se alguém renomear um destes, as notas geradas param de se agrupar
        // com as do vault e o lab perde a razão de existir.
        assert_eq!(Classificacao::Verified.termo(), "verified");
        assert_eq!(Classificacao::Partial.termo(), "partial");
        assert_eq!(Classificacao::Unsupported.termo(), "unsupported");
        assert_eq!(Classificacao::BlockedByPolicy.termo(), "blocked by policy");
        assert_eq!(
            Classificacao::RequiresRestartOrLogoff.termo(),
            "requires restart/logoff"
        );
        assert_eq!(Classificacao::Failed.termo(), "failed");
        assert_eq!(Classificacao::UserOrOemManaged.termo(), "user/OEM-managed");
    }

    #[test]
    fn sucesso_limpo_e_verified() {
        assert_eq!(
            classificar(DesfechoDoPlano::Sucesso, false, false, false, false),
            Classificacao::Verified
        );
        assert_eq!(
            classificar(DesfechoDoPlano::Falhou, false, false, false, false),
            Classificacao::Failed
        );
        assert_eq!(
            classificar(DesfechoDoPlano::EmParte, false, false, false, true),
            Classificacao::Unsupported
        );
    }

    #[test]
    fn o_titulo_nao_se_confunde_com_a_serie_numerada_do_vault() {
        // A série do vault é `WIN-COMPAT-0366`. Um título gerado que caísse na
        // mesma forma de quatro dígitos viraria colisão com a próxima nota que
        // alguém criasse à mão.
        let lab = lab_de_teste();
        let titulo = montar(&lab);

        assert!(titulo.contains("WIN-COMPAT-2026-09-14-"), "{}", &titulo[..200]);
        assert!(titulo.contains("measured: 2026-09-14"));
        assert!(titulo.contains("otimiza-version: 2.1.0"));
    }

    fn lab_de_teste() -> Lab {
        Lab {
            data: "2026-09-14".to_string(),
            classe: "Intel desktop (8GB)".to_string(),
            condicao: Condicao::MaquinaLimpa,
            classificacao: Classificacao::Verified,
            cabecalho: cabecalho_de_teste(),
            alimentacao: Alimentacao::Tomada,
            armazenamento: hardware::StorageKind::Ssd,
            vbs_rodando: Some(false),
            fabricante_da_imagem: None,
            modelo_da_imagem: None,
            essenciais_desativados: 0,
            relatorio: planoenergia::RelatorioDoPlano {
                maquina: cabecalho_de_teste().maquina,
                simulacao: true,
                plano_existia: false,
                guid_do_plano: None,
                guid_anterior: None,
                plano_ativo: false,
                ajustes: Vec::new(),
                aplicados: 0,
                ja_estavam_bons: 0,
                nao_suportados: 0,
                falhas: 0,
                desfecho: DesfechoDoPlano::EmParte,
            },
            testes: vec![("clean machine", true)],
        }
    }

    #[test]
    fn o_identificador_nao_carrega_nada_da_pessoa() {
        // Ele sai de processador, placa de vídeo, memória e build — coisas que
        // milhares de máquinas compartilham. E precisa ser ESTÁVEL: dois
        // relatórios do mesmo cliente têm que cair no mesmo id para dar para
        // comparar antes e depois.
        let a = cabecalho_de_teste();
        let b = cabecalho_de_teste();

        assert_eq!(identificador(&a), identificador(&b));
        assert_eq!(identificador(&a).len(), 8);

        let mut outra = cabecalho_de_teste();
        outra.maquina.cpu = "AMD Ryzen 5 5600X".to_string();

        assert_ne!(identificador(&a), identificador(&outra));
    }

    fn cabecalho_de_teste() -> cabecalho::Dados {
        cabecalho::Dados {
            versao: "2.1.0".to_string(),
            windows: "Windows 10 Pro".to_string(),
            build: "19045.4170".to_string(),
            edicao: "Professional".to_string(),
            arquitetura_do_windows: "AMD64".to_string(),
            arquitetura_do_processo: "64 bits",
            elevado: true,
            maquina: planoenergia::Maquina {
                notebook: false,
                tem_bateria: false,
                fabricante_da_cpu: F::Intel,
                cpu: "Intel(R) Core(TM) i3-10100F".to_string(),
                nucleos_logicos: 8,
                modern_standby: false,
                build_do_windows: 19045,
                windows11: false,
            },
            nucleos_fisicos: Some(4),
            gpu: "NVIDIA GeForce GTX 1650".to_string(),
            ram_gb: 7.9,
            plano_ativo: Some("381b4222-f694-41f0-9685-ff5bb260df2e".to_string()),
            nome_do_plano_ativo: Some("Equilibrado".to_string()),
        }
    }

    #[test]
    fn o_relatorio_nao_marca_teste_que_nao_aconteceu() {
        // Persistência no reinício, rollback e queda no meio da operação exigem
        // aplicar de verdade. Uma leitura nunca os exerce, e marcá-los seria
        // mentir exatamente no campo que o protocolo criou para não deixar
        // mentir.
        for nome in ["restart persistence", "rollback", "app crash mid-operation"] {
            assert!(
                !PODEM_SER_MARCADOS_POR_LEITURA.contains(&nome),
                "`{}` não pode ser marcado por um relatório que só lê",
                nome
            );
        }
    }

    /// Os testes do protocolo que uma execução de LEITURA consegue exercer.
    const PODEM_SER_MARCADOS_POR_LEITURA: &[&str] = &[
        "clean machine",
        "already-applied",
        "non-admin if relevant",
        "unsupported hardware/OS",
        "policy/OEM conflict",
    ];
}
