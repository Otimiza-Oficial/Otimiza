// Nota de laboratório de compatibilidade preenchida pela máquina do cliente, no formato do vault
// (`229 - Windows Customer Compatibility Labs`): as máquinas que faltam para a Fase 4 são as dos clientes. Não
// escreve nada; os sete termos são os do protocolo, cada um de uma leitura; não leva nada que identifique a pessoa
// (o arquivo vai inteiro para o Discord do atendimento).

use super::{cabecalho, essenciais, firmware, hardware, planoenergia};
use planoenergia::{Alimentacao, DesfechoDoPlano, RelatorioDoPlano, StatusDoAjuste};

pub const CAPACIDADE: &str = "power plan creation";

/// Vocabulário da lista de testes do protocolo, para as notas se agruparem com as escritas à mão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condicao {
    MaquinaLimpa,
    JaAplicado,
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

/// Os SETE termos do protocolo, e nenhum outro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classificacao {
    Verified,
    Partial,
    Unsupported,
    BlockedByPolicy,
    /// Nenhum ajuste de energia exige reinício, mas o termo é do PROTOCOLO: a próxima capacidade (HAGS, VBS)
    /// precisará dele, e com outro nome as notas não se agrupam.
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

/// Só um rótulo, na forma das notas do vault ("64GB workstation"); nenhuma decisão sai daqui.
pub fn classe_da_maquina(
    ram_gb: f64,
    notebook: bool,
    fabricante: planoenergia::FabricanteDaCpu,
) -> String {
    use planoenergia::FabricanteDaCpu as F;

    let forma = if notebook { "laptop" } else { "desktop" };

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

/// O Windows relata menos que o instalado: 7,9 GB são 8 GB para quem comprou.
pub fn arredondar_ram(gb: f64) -> u32 {
    const COMUNS: &[u32] = &[2, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256];

    COMUNS
        .iter()
        .copied()
        .find(|&c| gb <= c as f64 + 0.5)
        .unwrap_or_else(|| gb.round() as u32)
}

pub fn condicao(
    elevado: bool,
    plano_existia: bool,
    ha_ajuste_fora_do_alvo: bool,
) -> Condicao {
    if !elevado {
        return Condicao::SemAdministrador;
    }

    match (plano_existia, ha_ajuste_fora_do_alvo) {
        (true, true) => Condicao::ClienteMudouManualmente,
        (true, false) => Condicao::JaAplicado,
        (false, _) => Condicao::MaquinaLimpa,
    }
}

/// A ORDEM É A REGRA, do mais específico ao mais geral: gerência de OEM com ajuste sem suporte é nota sobre a
/// gerência.
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

    // Aceito e o valor não ficou: alguém reescreve por baixo (política de domínio). Não há o que consertar no
    // produto.
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

pub struct Lab {
    /// Entra no `Lab` para a montagem continuar pura.
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

/// Não identifica a pessoa (processador, memória e build, que milhares compartilham): serve para dois relatórios
/// da mesma máquina caírem no mesmo id.
pub fn identificador(d: &cabecalho::Dados) -> String {
    // FNV-1a de 32 bits: é um rótulo, não criptografia.
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

pub fn montar(lab: &Lab) -> String {
    let d = &lab.cabecalho;
    let mut s = String::new();

    // Número não inventado: a série do vault é à mão, e `AAAA-MM-DD-xxxxxxxx` não colide com os quatro dígitos.
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

    s.push_str("\n## Classify\n\n");
    s.push_str(&format!("**{}**\n\n", lab.classificacao.termo()));
    s.push_str(&format!(
        "- applied and verified: {}\n- already correct: {}\n- unsupported on this Windows: {}\n- did not take: {}\n",
        lab.relatorio.aplicados,
        lab.relatorio.ja_estavam_bons,
        lab.relatorio.nao_suportados,
        lab.relatorio.falhas
    ));

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

/// NÃO ESCREVE NADA: o plano roda em simulação.
pub fn gerar() -> Result<String, String> {
    let dados = cabecalho::coletar();

    // Numa máquina já aplicada, a simulação lê os valores reais de depois, sem reaplicar.
    let relatorio = planoenergia::montar(true)?;

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

    // A mesma definição do motor (`windows::governanca`), não uma cópia: duas divergiriam no primeiro conserto.
    let gerenciada_por_oem = super::governanca().imagem_de_terceiros;

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
        // Marca só o que ESTA execução exerceu.
        testes: vec![
            ("clean machine", condicao == Condicao::MaquinaLimpa),
            ("already-applied", condicao == Condicao::JaAplicado),
            ("non-admin if relevant", !dados.elevado),
            ("unsupported hardware/OS", sem_suporte),
            ("policy/OEM conflict", gerenciada_por_oem || falha_de_verificacao),
            // Exigem aplicar, reiniciar e desfazer: uma leitura nunca os exerce.
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

    /// `cargo test --lib labcompat -- --ignored --nocapture`
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
        assert_eq!(arredondar_ram(7.9), 8);
        assert_eq!(arredondar_ram(15.8), 16);
        assert_eq!(arredondar_ram(31.7), 32);
        assert_eq!(arredondar_ram(63.8), 64);
        assert_eq!(arredondar_ram(3.9), 4);
    }

    #[test]
    fn plano_existente_com_ajuste_fora_do_alvo_e_mudanca_manual() {
        assert_eq!(
            condicao(true, true, true),
            Condicao::ClienteMudouManualmente
        );
        assert_eq!(condicao(true, true, false), Condicao::JaAplicado);
        assert_eq!(condicao(true, false, false), Condicao::MaquinaLimpa);
    }

    #[test]
    fn sem_administrador_vence_tudo() {
        assert_eq!(condicao(false, true, true), Condicao::SemAdministrador);
        assert_eq!(condicao(false, false, false), Condicao::SemAdministrador);
    }

    #[test]
    fn comando_aceito_com_valor_que_nao_ficou_e_politica_e_nao_falha() {
        assert_eq!(
            classificar(DesfechoDoPlano::EmParte, false, true, false, false),
            Classificacao::BlockedByPolicy
        );
    }

    #[test]
    fn imagem_modificada_vence_o_resto() {
        assert_eq!(
            classificar(DesfechoDoPlano::EmParte, true, false, false, true),
            Classificacao::UserOrOemManaged
        );
    }

    #[test]
    fn os_sete_termos_sao_os_do_protocolo() {
        // Renomear um destes desagrupa as notas geradas das do vault.
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
        for nome in ["restart persistence", "rollback", "app crash mid-operation"] {
            assert!(
                !PODEM_SER_MARCADOS_POR_LEITURA.contains(&nome),
                "`{}` não pode ser marcado por um relatório que só lê",
                nome
            );
        }
    }

    const PODEM_SER_MARCADOS_POR_LEITURA: &[&str] = &[
        "clean machine",
        "already-applied",
        "non-admin if relevant",
        "unsupported hardware/OS",
        "policy/OEM conflict",
    ];
}
