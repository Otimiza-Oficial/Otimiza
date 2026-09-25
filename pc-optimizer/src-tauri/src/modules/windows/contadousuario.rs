// A conta que vai jogar e a conta que roda o Otimiza. Os ajustes de `HKEY_CURRENT_USER` vão para a conta do
// PROCESSO: se o cliente eleva com a senha de OUTRA conta (a "Admin" de fábrica, a do técnico), vão para um
// perfil que ninguém usa, e a releitura confere e diz "aplicado". A conta que joga é a dona do `explorer.exe`.
// O produto AVISA e não tenta escrever no perfil de outra conta.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum Conta {
    Mesma { usuario: String },
    Diferente { processo: String, shell: String },
    /// NÃO é "está tudo bem": é uma verificação que não aconteceu.
    NaoDeuParaLer { motivo: String },
}

impl Conta {
    /// `None` e não `false`: "está errado" e "não sei" mandam a pessoa fazer coisas diferentes.
    pub fn ajustes_de_usuario_valem(&self) -> Option<bool> {
        match self {
            Conta::Mesma { .. } => Some(true),
            Conta::Diferente { .. } => Some(false),
            Conta::NaoDeuParaLer { .. } => None,
        }
    }
}

/// Sem diferença de caixa: o Windows não distingue, e acusar contas diferentes aí é o alarme falso mais fácil.
pub fn comparar(processo: &str, shell: &str) -> Conta {
    let p = processo.trim();
    let s = shell.trim();

    if p.is_empty() || s.is_empty() {
        return Conta::NaoDeuParaLer {
            motivo: "o Windows não respondeu o nome de uma das contas.".to_string(),
        };
    }

    if p.eq_ignore_ascii_case(s) {
        Conta::Mesma { usuario: p.to_string() }
    } else {
        Conta::Diferente {
            processo: p.to_string(),
            shell: s.to_string(),
        }
    }
}

#[cfg(target_os = "windows")]
pub fn verificar() -> Conta {
    // Duas saídas separadas: em aspas simples do PowerShell o `n é literal, e a saída chegava numa linha só.
    let script = "[Security.Principal.WindowsIdentity]::GetCurrent().Name; \
                  $e = Get-CimInstance Win32_Process -Filter \"name='explorer.exe'\" \
                       -ErrorAction SilentlyContinue | Select-Object -First 1; \
                  if ($e) { \
                    $o = Invoke-CimMethod -InputObject $e -MethodName GetOwner; \
                    \"$($o.Domain)\\$($o.User)\" \
                  }";

    let saida = match super::shell::powershell(script) {
        Ok(s) if s.success => s.stdout,
        _ => {
            return Conta::NaoDeuParaLer {
                motivo: "não deu para perguntar ao Windows quem está com a sessão aberta."
                    .to_string(),
            }
        }
    };

    ler_duas_linhas(&saida)
}

#[cfg(not(target_os = "windows"))]
pub fn verificar() -> Conta {
    Conta::NaoDeuParaLer { motivo: "só no Windows.".to_string() }
}

/// Uma linha só = `explorer.exe` não encontrado (sessão remota, modo de segurança): `NaoDeuParaLer`, nunca
/// "as contas são iguais".
pub fn ler_duas_linhas(saida: &str) -> Conta {
    let linhas: Vec<&str> = saida.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    match linhas.as_slice() {
        [processo, shell] => comparar(processo, shell),
        [_um_so] => Conta::NaoDeuParaLer {
            motivo: "não há um Explorer rodando nesta sessão, então não dá para saber que \
                     conta está na frente da máquina."
                .to_string(),
        },
        _ => Conta::NaoDeuParaLer {
            motivo: "o Windows respondeu de um jeito que não deu para ler.".to_string(),
        },
    }
}

pub fn explicar(conta: &Conta, quantos_ajustes: usize) -> String {
    match conta {
        Conta::Mesma { usuario } => format!(
            "O Otimiza está rodando pela conta {usuario}, que é a mesma que está usando o \
             computador. Os ajustes que dependem da conta — mouse, transparência, \
             notificações — vão para o lugar certo."
        ),
        Conta::Diferente { processo, shell } => format!(
            "ATENÇÃO: o Otimiza está rodando pela conta {processo}, e quem está usando o \
             computador é {shell}. Isso acontece quando a senha digitada na tela de \
             administrador é de outra conta.\n\n\
             {quantos_ajustes} ajustes deste catálogo são por CONTA, e não pela máquina — \
             mouse, transparência, notificações, aplicativos em segundo plano. Aplicados \
             assim, eles vão para o perfil de {processo} e não vão surtir efeito nenhum \
             para {shell}. O Otimiza vai dizer que conferiu e vai estar certo: ele confere \
             a conta que está gravando, que é a errada.\n\n\
             O que resolve: feche o Otimiza, torne {shell} administrador do computador, e \
             abra de novo por essa conta. Os ajustes que valem para a máquina inteira — \
             plano de energia, serviços, placa de vídeo — funcionam do mesmo jeito e não \
             são afetados por isto."
        ),
        Conta::NaoDeuParaLer { motivo } => format!(
            "Não deu para saber se o Otimiza está rodando pela mesma conta que está usando \
             o computador: {motivo} Isso importa porque {quantos_ajustes} ajustes são por \
             conta, e elevados por outra eles iriam para o perfil errado. Esta verificação \
             não aconteceu — o que não quer dizer que esteja tudo bem."
        ),
    }
}

/// Contado do catálogo, e não escrito à mão.
pub fn quantos_sao_por_conta() -> usize {
    use super::catalog::{Action, CATALOG};

    CATALOG
        .iter()
        .filter(|spec| {
            spec.actions.iter().any(|a| {
                matches!(a, Action::Registry { hive, .. } if hive.eq_ignore_ascii_case("HKCU"))
            })
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mesma_conta_e_reconhecida() {
        assert_eq!(
            comparar("SNYX-PC\\User", "SNYX-PC\\User"),
            Conta::Mesma { usuario: "SNYX-PC\\User".to_string() }
        );
    }

    #[test]
    fn a_caixa_das_letras_nao_cria_conta_diferente() {
        assert!(matches!(
            comparar("SNYX-PC\\User", "snyx-pc\\user"),
            Conta::Mesma { .. }
        ));
    }

    #[test]
    fn contas_diferentes_sao_apontadas() {
        let c = comparar("PC\\Admin", "PC\\Eduardo");

        assert_eq!(
            c,
            Conta::Diferente {
                processo: "PC\\Admin".to_string(),
                shell: "PC\\Eduardo".to_string(),
            }
        );
        assert_eq!(c.ajustes_de_usuario_valem(), Some(false));
    }

    #[test]
    fn sem_explorer_o_produto_nao_presume_que_esta_tudo_bem() {
        let c = ler_duas_linhas("PC\\Admin\n");

        assert!(matches!(c, Conta::NaoDeuParaLer { .. }));
        assert_eq!(c.ajustes_de_usuario_valem(), None, "não sei não é não");
    }

    #[test]
    fn saida_vazia_nao_vira_conta_igual() {
        assert!(matches!(ler_duas_linhas(""), Conta::NaoDeuParaLer { .. }));
        assert!(matches!(comparar("", "PC\\Eduardo"), Conta::NaoDeuParaLer { .. }));
    }

    #[test]
    fn le_as_duas_linhas_da_saida() {
        let c = ler_duas_linhas("PC\\Admin\r\nPC\\Eduardo\r\n");

        assert_eq!(
            c,
            Conta::Diferente {
                processo: "PC\\Admin".to_string(),
                shell: "PC\\Eduardo".to_string(),
            }
        );
    }

    /// Diz O QUE FAZER, e que o resto continua funcionando.
    #[test]
    fn o_aviso_diz_o_que_fazer_e_o_que_continua_valendo() {
        let frase = explicar(
            &Conta::Diferente {
                processo: "PC\\Admin".to_string(),
                shell: "PC\\Eduardo".to_string(),
            },
            21,
        );

        assert!(frase.contains("21"), "precisa dizer quantos ajustes são afetados");
        assert!(frase.contains("PC\\Eduardo administrador"), "falta o conserto");
        assert!(
            frase.contains("não são afetados"),
            "precisa dizer que plano de energia e serviços continuam valendo"
        );
        assert!(
            frase.contains("vai dizer que conferiu e vai estar certo"),
            "a parte mais importante: a verificação está certa e a resposta é errada"
        );
    }

    #[test]
    fn a_lacuna_nao_soa_como_aprovacao() {
        let frase = explicar(
            &Conta::NaoDeuParaLer { motivo: "sem shell.".to_string() },
            21,
        );

        assert!(frase.contains("não quer dizer que esteja tudo bem"));
    }

    #[test]
    fn a_contagem_de_ajustes_por_conta_vem_do_catalogo() {
        let n = quantos_sao_por_conta();

        assert!(n >= 5, "só {n} ajustes por conta? a contagem provavelmente quebrou");
        assert!(n < super::super::catalog::CATALOG.len());
    }

    #[test]
    #[ignore = "toca o Windows desta máquina"]
    fn a_conta_desta_maquina() {
        let c = verificar();
        println!("{c:?}");
        println!("{}", explicar(&c, quantos_sao_por_conta()));
    }
}
