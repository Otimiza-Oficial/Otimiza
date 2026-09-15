// A conta que vai jogar, e a conta que está rodando o Otimiza
//
// ESTE É O DEFEITO MAIS SILENCIOSO QUE ESTE PROJETO TEM, e ele explica uma
// classe inteira de "funcionou aqui e não lá".
//
// Vinte e uma das otimizações do catálogo escrevem em `HKEY_CURRENT_USER` —
// aceleração do mouse, transparência, notificações, efeitos visuais, busca na
// internet do menu Iniciar, aplicativos em segundo plano. `HKEY_CURRENT_USER`
// não é "o usuário da máquina": é a conta do PROCESSO que está escrevendo.
//
// Agora o cenário real, e ele é comum no público deste produto:
//
//   O PC do cliente tem uma conta comum, sem privilégio. O Otimiza precisa de
//   administrador. O Windows pergunta a senha, e o cliente digita a senha da
//   conta de administrador — que é OUTRA CONTA, muitas vezes a do técnico que
//   montou a máquina, ou a conta "Admin" que veio de fábrica.
//
//   A partir daí, `HKEY_CURRENT_USER` aponta para o perfil dessa outra conta.
//   As vinte e uma otimizações são gravadas num perfil que ninguém usa.
//
// E o pior: **o produto confere e diz que deu certo.** Ele relê a chave que
// acabou de escrever, encontra o valor certo, e reporta "aplicado e conferido".
// A verificação está correta e a resposta está errada — porque a pergunta era
// sobre a conta errada.
//
// O cliente reinicia, joga, não vê diferença nenhuma no mouse, e conclui que o
// produto não faz nada. Ele tem razão: naquela máquina, não fez.
//
// ─────────────────────────────────────────────────────────────────────────
// COMO SE DESCOBRE
//
// A conta que vai jogar é a dona do `explorer.exe` — o shell do Windows roda
// sempre como o usuário que está na frente da máquina, e continua rodando como
// ele mesmo quando outra conta eleva alguma coisa por cima.
//
// Comparar o dono do shell com a conta do processo responde a pergunta. Os dois
// nomes vêm do próprio Windows, no formato `DOMÍNIO\usuário`, e não são
// traduzidos.
//
// Conferido nesta máquina: processo `SNYX-PC\User`, shell `SNYX-PC\User`.
//
// ─────────────────────────────────────────────────────────────────────────
// O QUE O PRODUTO FAZ COM ISSO
//
// AVISA, e não tenta consertar. Escrever no perfil de outro usuário exige
// carregar a colmeia dele (`reg load`) e adivinhar qual é — e uma ferramenta
// que escreve no registro de outra conta é uma ferramenta que ninguém deveria
// instalar. O conserto é a pessoa abrir o Otimiza pela conta dela, com ela
// sendo administradora, e isso o aviso explica.

use serde::{Deserialize, Serialize};

/// Quem está rodando, comparado com quem vai jogar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum Conta {
    /// A mesma. É o caso normal, e o único em que os ajustes de usuário valem.
    Mesma { usuario: String },
    /// Contas diferentes: os ajustes de usuário vão para o perfil errado.
    Diferente { processo: String, shell: String },
    /// Não deu para descobrir. NÃO é "está tudo bem" — é uma verificação que
    /// não aconteceu, e ela fica dita como tal.
    NaoDeuParaLer { motivo: String },
}

impl Conta {
    /// Os ajustes de usuário vão para o perfil certo?
    ///
    /// `NaoDeuParaLer` responde `None`, e não `false`: quem chama precisa poder
    /// distinguir "está errado" de "não sei", porque as duas frases mandam a
    /// pessoa fazer coisas diferentes.
    pub fn ajustes_de_usuario_valem(&self) -> Option<bool> {
        match self {
            Conta::Mesma { .. } => Some(true),
            Conta::Diferente { .. } => Some(false),
            Conta::NaoDeuParaLer { .. } => None,
        }
    }
}

/// Compara os dois nomes. **Função pura.**
///
/// Sem diferença entre maiúscula e minúscula: o Windows não distingue nome de
/// usuário por caixa, e `SNYX-PC\User` e `snyx-pc\user` são a mesma conta. Uma
/// comparação sensível a caixa acusaria contas diferentes onde não há — que é o
/// alarme falso mais fácil de cometer aqui.
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

/// Lê o dono do `explorer.exe` e a conta do processo.
#[cfg(target_os = "windows")]
pub fn verificar() -> Conta {
    // DUAS SAÍDAS SEPARADAS, e não uma linha com `\n` no meio.
    //
    // A primeira versão montava as duas com `'{0}`n{1}' -f ...`. Em PowerShell,
    // aspas SIMPLES são literais: o `n não vira quebra de linha, vira os dois
    // caracteres. A saída chegava numa linha só, o módulo respondia "não deu
    // para ler", e a verificação que existe para pegar defeito silencioso
    // falhava em silêncio. Pego rodando contra esta máquina.
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

/// **Função pura**: a saída são duas linhas, processo e shell.
///
/// Com UMA linha só, o `explorer.exe` não foi encontrado — acontece em sessão
/// sem shell, como conexão remota recém-aberta ou máquina em modo de segurança.
/// Isso é `NaoDeuParaLer` e não "as contas são iguais": presumir igualdade ali
/// devolveria o produto ao defeito que este módulo existe para pegar.
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

/// A frase que o cliente lê. Regra de produto, e por isso tem teste.
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

/// Quantos ajustes do catálogo dependem da conta.
///
/// Contado do catálogo e não escrito à mão: um número na frase que diverge da
/// lista é a mesma classe de mentira que o resto do produto passou versões
/// consertando.
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

    /// O alarme falso mais fácil de cometer aqui: o Windows não distingue conta
    /// por caixa, e `snyx-pc\user` é a mesma conta que `SNYX-PC\User`. Acusar
    /// diferença ali mandaria o cliente reinstalar a máquina por nada.
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

    /// Sem Explorer, o produto NÃO presume que as contas são iguais. Presumir
    /// ali devolveria exatamente o defeito que este módulo existe para pegar.
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

    /// O aviso precisa dizer O QUE FAZER, e precisa dizer que o resto continua
    /// funcionando — senão o cliente acha que o produto inteiro não serve.
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

    /// O número na frase sai do catálogo e não da minha memória. Vinte e um é o
    /// que havia quando este módulo nasceu; o teste existe para que a mudança
    /// desse número seja percebida, não para travá-lo.
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
