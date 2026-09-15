// O que olhar na BIOS, em fases
//
// ESTE MÓDULO NÃO ESCREVE NADA, e isso não é limitação — é a decisão central
// dele. O Windows não tem como alterar a configuração de firmware de um PC de
// mesa, e uma ferramenta que encontrasse um jeito de fazer isso seria uma
// ferramenta capaz de deixar a máquina do cliente sem ligar.
//
// O que ele faz é montar a lista do que vale olhar, EM ORDEM DE RISCO, com o
// modelo da placa-mãe e a versão da BIOS ao lado — porque com esses dois dados
// a pessoa acha o manual certo em vez de seguir um vídeo de outra placa.
//
// ─────────────────────────────────────────────────────────────────────────
// AS CINCO FASES, e por que a ordem é essa
//
//   FASE 1 — ligar o que a pessoa JÁ COMPROU. XMP/EXPO, Resizable BAR, sair do
//            modo Legacy. Nada aqui é overclock: é usar a peça na velocidade
//            que está escrita na caixa dela.
//
//   FASE 2 — o que o fabricante documenta e reverte num clique. Costuma ser
//            uma opção só, e o pior caso é voltar como estava.
//
//   FASE 3 — o que exige TESTE DE ESTABILIDADE. Curve Optimizer, undervolt. A
//            falha aqui não aparece na hora: aparece como travada aleatória
//            três semanas depois, quando ninguém mais lembra do que mexeu.
//
//   FASE 4 — overclock manual. O Otimiza NÃO orienta, e diz por quê.
//
//   FASE 5 — atualizar a BIOS. NUNCA automaticamente, nunca sem plano de
//            recuperação, e só quando há um motivo nomeado.
//
// A ordem é por risco crescente e por reversibilidade decrescente. Quem parar
// na fase 1 pegou a maior parte do ganho disponível — e é o que a maioria
// deveria fazer.
//
// ─────────────────────────────────────────────────────────────────────────
// O QUE ESTE MÓDULO SE RECUSA A DAR
//
// **Número de Curve Optimizer.** Não existe valor seguro genérico: cada chip
// aceita um limite diferente, e um `-30` copiado de um vídeo produz uma máquina
// que passa em benchmark e trava no jogo. Dizer "use -20" seria o mesmo tipo de
// palpite que gravou o valor errado de placa de vídeo na 2.1.0, com um custo
// bem maior — ali o cliente perdeu FPS; aqui ele perderia estabilidade sem
// saber por quê.

use serde::{Deserialize, Serialize};

/// O que o Windows conseguiu ler do firmware.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Leitura {
    /// Fabricante e modelo da placa-mãe — o que permite achar o manual certo.
    pub placa_mae: Option<String>,
    pub versao_da_bios: Option<String>,
    /// Data de lançamento da BIOS, como o Windows a declara.
    pub data_da_bios: Option<String>,
    /// `Some(true)` para UEFI, `Some(false)` para Legacy/CSM. `None` é não sei.
    pub uefi: Option<bool>,
    pub secure_boot: Option<bool>,
    pub lacunas: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Fase {
    /// Ligar o que já foi comprado.
    UsarOQueTem = 1,
    /// Documentado pelo fabricante, reverte num clique.
    Documentado = 2,
    /// Exige teste de estabilidade.
    ExigeTeste = 3,
    /// Overclock manual. O produto não orienta.
    NaoOrientamos = 4,
    /// Atualizar a BIOS. Nunca automático.
    UltimoRecurso = 5,
}

impl Fase {
    pub fn titulo(self) -> &'static str {
        match self {
            Fase::UsarOQueTem => "Fase 1 — ligar o que você já comprou",
            Fase::Documentado => "Fase 2 — o que o fabricante documenta",
            Fase::ExigeTeste => "Fase 3 — exige teste de estabilidade",
            Fase::NaoOrientamos => "Fase 4 — overclock manual",
            Fase::UltimoRecurso => "Fase 5 — atualizar a BIOS",
        }
    }

    pub fn explicacao(self) -> &'static str {
        match self {
            Fase::UsarOQueTem => {
                "Nada aqui é overclock: é usar a peça na velocidade que está escrita na \
                 caixa dela. É onde está a maior parte do ganho disponível, e quem parar \
                 nesta fase fez o que a maioria deveria fazer."
            }
            Fase::Documentado => {
                "Opções que o fabricante da placa documenta no manual e que voltam ao que \
                 eram num clique. O pior caso é não mudar nada."
            }
            Fase::ExigeTeste => {
                "A falha destes ajustes NÃO aparece na hora. Ela aparece como travada \
                 aleatória três semanas depois, quando ninguém mais lembra do que mexeu. \
                 Só entre aqui se você estiver disposto a testar estabilidade por horas e \
                 a desfazer ao primeiro sinal."
            }
            Fase::NaoOrientamos => {
                "O Otimiza não dá números de overclock manual. Não é cautela decorativa: \
                 cada processador aceita um limite diferente, e um número copiado de um \
                 vídeo produz uma máquina que passa em teste e trava no jogo."
            }
            Fase::UltimoRecurso => {
                "Atualizar a BIOS resolve problema NOMEADO — compatibilidade com um \
                 processador novo, um defeito que o fabricante corrigiu. Não é manutenção \
                 de rotina, e uma queda de energia no meio do processo deixa a máquina sem \
                 ligar. O Otimiza nunca faz isso, e nunca vai fazer."
            }
        }
    }
}

/// Um item para olhar na BIOS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Passo {
    pub id: String,
    pub fase: Fase,
    pub titulo: String,
    /// Onde a opção costuma estar. "Costuma" é honesto: o nome muda de placa
    /// para placa, e é por isso que o modelo da placa aparece junto.
    pub onde: String,
    pub o_que_faz: String,
    /// O que pode dar errado, e como voltar. Nunca vazio.
    pub risco_e_volta: String,
    /// Verdadeiro quando o Otimiza MEDIU alguma coisa que torna este passo
    /// relevante nesta máquina — e não quando ele é só uma boa ideia genérica.
    pub medido_aqui: bool,
}

/// Monta a lista. **Função pura.**
///
/// `memoria_abaixo_do_nominal` e `rebar_desligado` vêm de quem já mede essas
/// coisas (`firmware`, `rbar`). Este módulo não remede nada: duas leituras da
/// mesma coisa é como um produto passa a mostrar dois números diferentes para o
/// mesmo fato em duas telas.
pub fn montar(
    leitura: &Leitura,
    memoria_abaixo_do_nominal: bool,
    rebar_desligado: bool,
    amd: bool,
) -> Vec<Passo> {
    let mut passos = Vec::new();

    // ── Fase 1 ────────────────────────────────────────────────────────────
    passos.push(Passo {
        id: "xmp".to_string(),
        fase: Fase::UsarOQueTem,
        titulo: "Perfil de velocidade da memória (XMP, EXPO ou DOCP)".to_string(),
        onde: "Costuma estar na primeira tela do modo avançado, ou em \"OC\" / \"Ai Tweaker\" \
               / \"Extreme Tweaker\", como uma lista com \"Disabled\" e \"Profile 1\"."
            .to_string(),
        o_que_faz: "Os pentes de memória são vendidos com uma velocidade na embalagem e \
                    ligam numa velocidade menor, padrão do processador. O perfil é a \
                    instrução que o próprio pente carrega para rodar no que foi anunciado. \
                    Ligar isso não é overclock: é parar de usar abaixo do que foi pago."
            .to_string(),
        risco_e_volta: "Em pente vendido em par, quase nunca dá problema. Se a máquina não \
                        ligar, ela volta sozinha para o padrão depois de algumas tentativas \
                        — e dá para forçar tirando a bateria da placa-mãe por um minuto, com \
                        o computador desligado da tomada."
            .to_string(),
        medido_aqui: memoria_abaixo_do_nominal,
    });

    passos.push(Passo {
        id: "rebar".to_string(),
        fase: Fase::UsarOQueTem,
        titulo: "Resizable BAR / Smart Access Memory".to_string(),
        onde: "Em \"PCI Subsystem Settings\" ou \"Advanced\". Exige \"Above 4G Decoding\" \
               ligado junto, e a placa precisa estar em modo UEFI."
            .to_string(),
        o_que_faz: "Deixa o processador enxergar a memória da placa de vídeo inteira de uma \
                    vez, em vez de em pedaços de 256 MB. Em parte dos jogos rende alguns por \
                    cento; em outros não muda nada, e em alguns poucos custa."
            .to_string(),
        risco_e_volta: "É uma opção de ligar e desligar, e volta do mesmo jeito. Como o ganho \
                        varia por jogo, meça antes e depois — o Otimiza guarda as duas \
                        medições."
            .to_string(),
        medido_aqui: rebar_desligado,
    });

    if leitura.uefi == Some(false) {
        passos.push(Passo {
            id: "csm".to_string(),
            fase: Fase::UsarOQueTem,
            titulo: "Sair do modo Legacy (CSM)".to_string(),
            onde: "Em \"Boot\", como \"CSM\" ou \"Compatibility Support Module\".".to_string(),
            o_que_faz: "Esta máquina está iniciando em modo de compatibilidade com BIOS \
                        antiga. Nesse modo o Resizable BAR não funciona, o Secure Boot não \
                        liga, e recursos de segurança do Windows 11 ficam indisponíveis."
                .to_string(),
            risco_e_volta: "ESTE É O PASSO MAIS DELICADO DA FASE 1: se o Windows foi \
                            instalado em modo Legacy, mudar para UEFI sem converter o disco \
                            faz a máquina não iniciar. A conversão existe e é feita pela \
                            ferramenta `mbr2gpt` do próprio Windows, mas ela pede backup \
                            antes. Não faça isso com pressa, e não faça sem cópia dos seus \
                            arquivos."
                .to_string(),
            medido_aqui: true,
        });
    }

    // ── Fase 2 ────────────────────────────────────────────────────────────
    if amd {
        passos.push(Passo {
            id: "pbo".to_string(),
            fase: Fase::Documentado,
            titulo: "Precision Boost Overdrive (PBO) em Auto".to_string(),
            onde: "Em \"AMD Overclocking\" → \"Precision Boost Overdrive\".".to_string(),
            o_que_faz: "Deixa o processador usar a folga térmica e elétrica que a placa \
                        oferece, dentro dos limites que a própria AMD define. Em Auto, quem \
                        decide continua sendo o processador — o que muda é o teto que ele \
                        pode usar."
                .to_string(),
            risco_e_volta: "Em Auto é reversível numa opção e não força tensão nenhuma. Ele \
                            esquenta mais e consome mais, então num gabinete mal ventilado o \
                            ganho pode virar zero. Meça a temperatura depois."
                .to_string(),
            medido_aqui: false,
        });
    }

    // ── Fase 3 ────────────────────────────────────────────────────────────
    if amd {
        passos.push(Passo {
            id: "curve_optimizer".to_string(),
            fase: Fase::ExigeTeste,
            titulo: "Curve Optimizer".to_string(),
            onde: "Dentro do PBO, como \"Curve Optimizer\", por núcleo ou para todos."
                .to_string(),
            o_que_faz: "Reduz a tensão que o processador pede em cada ponto da curva de \
                        frequência. Menos tensão é menos calor, e menos calor costuma virar \
                        frequência mais alta sustentada. É o ajuste com melhor relação entre \
                        esforço e ganho nos Ryzen — e o mais fácil de fazer errado."
                .to_string(),
            risco_e_volta: "O OTIMIZA NÃO DÁ NÚMERO AQUI, e isso é deliberado: cada chip \
                            aceita um limite diferente, e um valor copiado de um vídeo \
                            produz uma máquina que passa em teste de estresse e trava no \
                            jogo, semanas depois, sem ninguém ligar as duas coisas. Se for \
                            fazer: comece conservador, teste horas, e desfaça ao primeiro \
                            travamento — inclusive os que parecerem não ter relação."
                .to_string(),
            medido_aqui: false,
        });
    }

    // ── Fase 4 e 5: sempre presentes, porque a recusa é parte da orientação ─
    passos.push(Passo {
        id: "overclock_manual".to_string(),
        fase: Fase::NaoOrientamos,
        titulo: "Overclock manual de processador ou memória".to_string(),
        onde: "Não se aplica: o Otimiza não orienta este caminho.".to_string(),
        o_que_faz: "Fixar frequência e tensão à mão, acima do que o fabricante garante."
            .to_string(),
        risco_e_volta: "Não damos números porque não existe número genérico que sirva. O que \
                        dá para dizer com honestidade: num PC de jogo moderno, overclock \
                        manual de processador rende pouco perto do que o próprio turbo já \
                        faz, e cobra em calor, consumo e estabilidade. Se o seu objetivo é \
                        FPS, as fases 1 e 2 têm mais a entregar."
            .to_string(),
        medido_aqui: false,
    });

    passos.push(Passo {
        id: "atualizar_bios".to_string(),
        fase: Fase::UltimoRecurso,
        titulo: "Atualizar a BIOS".to_string(),
        onde: format!(
            "No site do fabricante da placa, procurando pelo modelo exato{}.",
            leitura
                .placa_mae
                .as_deref()
                .map(|p| format!(" — o desta máquina é {p}"))
                .unwrap_or_default()
        ),
        o_que_faz: "Corrige defeitos e acrescenta compatibilidade. Não é manutenção de \
                    rotina e raramente muda desempenho."
            .to_string(),
        risco_e_volta: "O OTIMIZA NUNCA FAZ ISSO E NUNCA VAI FAZER. Uma queda de energia no \
                        meio do processo deixa a máquina sem ligar, e a recuperação depende \
                        de a placa ter um recurso que nem toda placa tem. Só atualize com um \
                        motivo nomeado, com o arquivo baixado do site do fabricante da SUA \
                        placa, e nunca com o computador ligado só na tomada da parede sem \
                        nobreak."
            .to_string(),
        medido_aqui: false,
    });

    passos.sort_by_key(|p| p.fase);
    passos
}

/// Lê o firmware desta máquina.
#[cfg(target_os = "windows")]
pub fn ler() -> Leitura {
    let mut leitura = Leitura::default();

    let script = "$b = Get-CimInstance Win32_BIOS -ErrorAction SilentlyContinue; \
                  $m = Get-CimInstance Win32_BaseBoard -ErrorAction SilentlyContinue; \
                  ConvertTo-Json -Compress -InputObject ([ordered]@{ \
                    Placa = \"$($m.Manufacturer) $($m.Product)\"; \
                    Versao = [string]$b.SMBIOSBIOSVersion; \
                    Data = [string]$b.ReleaseDate; \
                    Uefi = ($env:firmware_type -eq 'UEFI'); \
                    Secure = $(try { [bool](Confirm-SecureBootUEFI) } catch { $null }) })";

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Bruto {
        placa: Option<String>,
        versao: Option<String>,
        data: Option<String>,
        uefi: Option<bool>,
        secure: Option<bool>,
    }

    let bruto: Option<Bruto> = super::shell::powershell(script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok());

    let Some(b) = bruto else {
        leitura.lacunas.push(
            "Não deu para ler a placa-mãe nem a versão da BIOS desta máquina. Sem isso, o \
             passo a passo abaixo continua valendo, mas você vai precisar descobrir o modelo \
             da placa de outro jeito para achar o manual certo."
                .to_string(),
        );
        return leitura;
    };

    leitura.placa_mae = b.placa.map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
    leitura.versao_da_bios = b.versao.filter(|v| !v.trim().is_empty());
    leitura.data_da_bios = b.data.filter(|d| !d.trim().is_empty());
    leitura.uefi = b.uefi;
    leitura.secure_boot = b.secure;

    if leitura.uefi.is_none() {
        leitura
            .lacunas
            .push("Não deu para saber se esta máquina inicia em modo UEFI ou Legacy.".to_string());
    }

    leitura
}

#[cfg(not(target_os = "windows"))]
pub fn ler() -> Leitura {
    Leitura::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leitura_padrao() -> Leitura {
        Leitura {
            placa_mae: Some("Micro-Star International Co., Ltd. PRO H410M-B(MS-7D82)".to_string()),
            versao_da_bios: Some("1.20".to_string()),
            data_da_bios: Some("20220522210000.000000+000".to_string()),
            uefi: Some(true),
            secure_boot: Some(true),
            lacunas: Vec::new(),
        }
    }

    /// As fases saem em ordem de risco crescente. Uma lista fora de ordem faria
    /// o cliente encontrar o Curve Optimizer antes do XMP — e ele mexeria no
    /// caro antes do barato.
    #[test]
    fn os_passos_saem_em_ordem_de_risco() {
        let passos = montar(&leitura_padrao(), true, true, true);

        let fases: Vec<Fase> = passos.iter().map(|p| p.fase).collect();
        let mut ordenadas = fases.clone();
        ordenadas.sort();

        assert_eq!(fases, ordenadas, "as fases saíram fora de ordem");
        assert_eq!(passos[0].fase, Fase::UsarOQueTem);
    }

    /// A REGRA MAIS IMPORTANTE: o produto não dá número de Curve Optimizer.
    ///
    /// Um `-30` copiado de vídeo produz uma máquina que passa em teste e trava
    /// no jogo semanas depois. Seria o mesmo palpite que gravou o valor errado
    /// de placa de vídeo na 2.1.0, com um custo bem maior.
    #[test]
    fn o_curve_optimizer_nunca_vem_com_numero() {
        let passos = montar(&leitura_padrao(), false, false, true);
        let curve = passos.iter().find(|p| p.id == "curve_optimizer").unwrap();

        assert!(curve.risco_e_volta.contains("NÃO DÁ NÚMERO"));

        // Nenhum número negativo de offset no texto inteiro do passo.
        let texto = format!("{} {} {}", curve.titulo, curve.o_que_faz, curve.risco_e_volta);
        for proibido in ["-5", "-10", "-15", "-20", "-25", "-30"] {
            assert!(!texto.contains(proibido), "apareceu o valor {proibido} no texto");
        }
    }

    /// E a atualização de BIOS diz, com todas as letras, que o produto não faz.
    #[test]
    fn a_atualizacao_de_bios_e_recusada_por_escrito() {
        let passos = montar(&leitura_padrao(), false, false, false);
        let bios = passos.iter().find(|p| p.id == "atualizar_bios").unwrap();

        assert_eq!(bios.fase, Fase::UltimoRecurso);
        assert!(bios.risco_e_volta.contains("NUNCA FAZ ISSO E NUNCA VAI FAZER"));
        assert!(bios.risco_e_volta.contains("sem ligar"), "precisa dizer o pior caso");
    }

    /// Os passos de AMD só aparecem em máquina AMD. Mandar um dono de Intel
    /// procurar "Precision Boost Overdrive" no menu faria ele desistir do
    /// passo a passo inteiro na primeira tentativa.
    #[test]
    fn pbo_e_curve_optimizer_so_aparecem_em_amd() {
        let intel = montar(&leitura_padrao(), false, false, false);

        assert!(!intel.iter().any(|p| p.id == "pbo"));
        assert!(!intel.iter().any(|p| p.id == "curve_optimizer"));

        let amd = montar(&leitura_padrao(), false, false, true);
        assert!(amd.iter().any(|p| p.id == "pbo"));
    }

    /// O passo do modo Legacy só aparece quando a máquina ESTÁ em Legacy — e
    /// quando aparece, carrega o aviso de que mudar sem converter o disco faz o
    /// Windows não iniciar.
    #[test]
    fn o_passo_do_legacy_so_aparece_em_legacy_e_avisa_do_disco() {
        let uefi = montar(&leitura_padrao(), false, false, false);
        assert!(!uefi.iter().any(|p| p.id == "csm"));

        let legacy = Leitura { uefi: Some(false), ..leitura_padrao() };
        let passos = montar(&legacy, false, false, false);
        let csm = passos.iter().find(|p| p.id == "csm").expect("precisa aparecer");

        assert!(csm.risco_e_volta.contains("não iniciar"));
        assert!(csm.risco_e_volta.contains("mbr2gpt"), "precisa nomear a ferramenta");
        assert!(csm.risco_e_volta.contains("backup") || csm.risco_e_volta.contains("cópia"));
    }

    /// Não saber se é UEFI não pode virar "está em Legacy": seria mandar o
    /// cliente mexer no boot da máquina dele por causa de uma leitura falha.
    #[test]
    fn uefi_desconhecido_nao_vira_passo_de_legacy() {
        let sem_saber = Leitura { uefi: None, ..leitura_padrao() };
        assert!(!montar(&sem_saber, false, false, false).iter().any(|p| p.id == "csm"));
    }

    /// `medido_aqui` separa "isto vale para você" de "isto é uma boa ideia em
    /// geral". Sem essa marca, a lista seria a mesma em toda máquina — que é
    /// exatamente o que os vídeos de tweak fazem.
    #[test]
    fn o_que_foi_medido_aqui_e_marcado() {
        let com_achado = montar(&leitura_padrao(), true, true, false);

        assert!(com_achado.iter().find(|p| p.id == "xmp").unwrap().medido_aqui);
        assert!(com_achado.iter().find(|p| p.id == "rebar").unwrap().medido_aqui);

        let sem_achado = montar(&leitura_padrao(), false, false, false);
        assert!(!sem_achado.iter().find(|p| p.id == "xmp").unwrap().medido_aqui);
    }

    /// Todo passo diz o risco E como voltar. Um passo de BIOS sem volta escrita
    /// é um convite para o cliente ficar sem máquina.
    #[test]
    fn todo_passo_diz_o_risco_e_como_voltar() {
        for passo in montar(&leitura_padrao(), true, true, true) {
            assert!(
                passo.risco_e_volta.len() >= 120,
                "`{}` não explica o risco nem a volta",
                passo.id
            );
            assert!(!passo.onde.trim().is_empty(), "`{}` não diz onde procurar", passo.id);
        }
    }

    /// O modelo da placa vai junto do passo de atualização — é ele que faz a
    /// pessoa achar o arquivo certo em vez de um de outra placa parecida.
    #[test]
    fn o_modelo_da_placa_aparece_onde_importa() {
        let passos = montar(&leitura_padrao(), false, false, false);
        let bios = passos.iter().find(|p| p.id == "atualizar_bios").unwrap();

        assert!(bios.onde.contains("PRO H410M-B"));
    }

    /// E sem o modelo, a frase não fica quebrada nem inventa um nome.
    #[test]
    fn sem_o_modelo_a_frase_continua_inteira() {
        let sem = Leitura { placa_mae: None, ..leitura_padrao() };
        let passos = montar(&sem, false, false, false);
        let bios = passos.iter().find(|p| p.id == "atualizar_bios").unwrap();

        assert!(bios.onde.ends_with("modelo exato."));
    }

    #[test]
    fn toda_fase_se_explica() {
        for fase in [
            Fase::UsarOQueTem,
            Fase::Documentado,
            Fase::ExigeTeste,
            Fase::NaoOrientamos,
            Fase::UltimoRecurso,
        ] {
            assert!(fase.explicacao().len() >= 100, "{:?} sem explicação", fase);
            assert!(fase.titulo().starts_with("Fase "));
        }
    }

    #[test]
    #[ignore = "toca o Windows desta máquina"]
    fn o_firmware_desta_maquina() {
        let l = ler();
        println!("{l:#?}");
    }
}
