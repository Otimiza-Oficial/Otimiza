// O cabeçalho do registro: em QUE máquina isto aconteceu
//
// A 2.0 fez o registro ir para um arquivo (`utils::logger`). Faltava a metade
// que torna o arquivo útil no atendimento: o arquivo diz o que o produto fez,
// e não dizia nada sobre o computador onde fez.
//
// A DIFERENÇA, NA PRÁTICA. Uma linha como "aplicar `disable_vbs`: ação 2/3
// falhou" não responde nada sozinha. Com o cabeçalho, ela responde quase tudo:
// o processo estava elevado? é notebook ou desktop? qual build do Windows? qual
// plano de energia estava ativo? Sem isso o atendimento volta a pedir print,
// pedir AnyDesk, ou pedir que o cliente rode script de PowerShell à mão — que
// foi exatamente o que motivou o `suporte.rs`.
//
// E NÃO É O MESMO QUE O `suporte.rs`. Aquele monta um bloco curto para o
// cliente COLAR numa mensagem, e por isso cabe em 1900 caracteres e corta o que
// não couber. Este é escrito no arquivo, onde não há limite de tamanho, e
// existe para o caso em que o cliente MANDA O ARQUIVO porque algo deu errado —
// o cenário em que o bloco curto já não basta.
//
// AS REGRAS DO `suporte.rs` VALEM AQUI IGUAL, e a segunda é a que exige
// cuidado: NADA QUE IDENTIFIQUE A PESSOA. Sem nome de usuário do Windows, sem
// caminho de perfil, sem número de série. O cabeçalho descreve uma MÁQUINA, não
// um dono — e o arquivo vai parar no Discord de um atendimento.

use super::{hardware, planoenergia, power, registry};

/// Tudo que o cabeçalho imprime. Nenhum campo aqui lê o sistema: a coleta é
/// separada da montagem para que a montagem possa ser testada sem máquina, do
/// mesmo jeito que o `suporte::Entrada`.
pub struct Dados {
    pub versao: String,
    pub windows: String,
    pub build: String,
    pub edicao: String,
    pub arquitetura_do_windows: String,
    pub arquitetura_do_processo: &'static str,
    pub elevado: bool,
    pub maquina: planoenergia::Maquina,
    pub nucleos_fisicos: Option<usize>,
    pub gpu: String,
    pub ram_gb: f64,
    pub plano_ativo: Option<String>,
    pub nome_do_plano_ativo: Option<String>,
}

/// `None` vira "não deu para ler", e nunca um valor inventado.
///
/// Um campo em branco num relatório de suporte faz o atendimento supor — e a
/// suposição mais comum é a benigna ("deve estar normal"), que é a errada.
fn ou_nao_lido(valor: &Option<String>) -> &str {
    match valor {
        Some(v) if !v.trim().is_empty() => v,
        _ => "(não deu para ler)",
    }
}

fn sim_ou_nao(valor: bool) -> &'static str {
    if valor {
        "sim"
    } else {
        "não"
    }
}

/// Monta o bloco. Função pura.
pub fn montar(d: &Dados) -> String {
    let mut linhas = Vec::new();

    linhas.push("==============================".to_string());
    linhas.push("OTIMIZA — RELATÓRIO DO SISTEMA".to_string());
    linhas.push("==============================".to_string());

    let mut campo = |rotulo: &str, valor: String| {
        linhas.push(format!("{:<24}{}", rotulo, valor));
    };

    campo("Versão do Otimiza", d.versao.clone());
    campo("Windows", d.windows.clone());
    campo("Build", d.build.clone());
    campo("Edição", d.edicao.clone());
    campo("Arquitetura do Windows", d.arquitetura_do_windows.clone());

    // AS DUAS ARQUITETURAS, E NÃO UMA SÓ. Um processo de 32 bits num Windows de
    // 64 lê o registro pelo espelho `WOW6432Node` e enxerga outra máquina — as
    // otimizações "aplicam" e não valem. Ver as duas lado a lado é o que deixa
    // isso óbvio em vez de virar uma caça de uma tarde.
    campo(
        "Arquitetura do processo",
        d.arquitetura_do_processo.to_string(),
    );

    campo("Administrador", sim_ou_nao(d.elevado).to_string());
    campo("Processador", d.maquina.cpu.clone());
    campo(
        "Fabricante da CPU",
        format!("{:?}", d.maquina.fabricante_da_cpu),
    );
    campo(
        "Núcleos físicos",
        match d.nucleos_fisicos {
            Some(n) => n.to_string(),
            None => "(não deu para ler)".to_string(),
        },
    );
    campo(
        "Processadores lógicos",
        d.maquina.nucleos_logicos.to_string(),
    );
    campo("Placa de vídeo", d.gpu.clone());
    campo("Memória", format!("{:.1} GB", d.ram_gb));
    campo(
        "Tipo de máquina",
        if d.maquina.notebook {
            "notebook".to_string()
        } else {
            "desktop".to_string()
        },
    );
    campo("Bateria", sim_ou_nao(d.maquina.tem_bateria).to_string());
    campo(
        "Modern Standby",
        sim_ou_nao(d.maquina.modern_standby).to_string(),
    );
    campo(
        "Plano de energia",
        ou_nao_lido(&d.nome_do_plano_ativo).to_string(),
    );
    campo("GUID do plano", ou_nao_lido(&d.plano_ativo).to_string());

    linhas.push("==============================".to_string());

    linhas.join("\n")
}

/// Lê a máquina. Custa PowerShell e `powercfg` — por isso roda fora da abertura,
/// numa thread própria. Ver `anotar_em_segundo_plano`.
pub fn coletar() -> Dados {
    let perfil = hardware::profile();
    let maquina = planoenergia::detectar();

    let texto = |nome: &str| -> Option<String> {
        registry::read_text("HKLM", r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", nome)
            .ok()
            .flatten()
    };

    // O UBR É DWORD, E O `CurrentBuildNumber` É TEXTO — na mesma chave.
    //
    // Lido como texto, o UBR sumia e o build saía "19045" em vez de
    // "19045.4046". A revisão de build é o que separa um Windows atualizado de
    // um parado há um ano, e é justamente o tipo de diferença que faz uma
    // otimização funcionar aqui e não lá.
    let numero = |nome: &str| -> Option<u32> {
        use crate::modules::changelog::PreviousValue;

        match registry::read("HKLM", r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", nome) {
            Ok(PreviousValue::Dword(v)) => Some(v),
            _ => None,
        }
    };

    let plano_ativo = power::active_scheme().ok();

    // O NOME DO PLANO SAI DA LISTA, e não de uma tradução nossa. O `powercfg`
    // devolve o nome no idioma do Windows do cliente, e é esse nome que ele vê
    // no painel — escrever outro no relatório faria o atendimento e o cliente
    // falarem de coisas diferentes.
    let nome_do_plano_ativo = plano_ativo.as_ref().and_then(|guid| {
        let lista = super::shell::run_checked("powercfg", &["/list"]).ok()?;

        planoenergia::planos_da_saida(&lista)
            .into_iter()
            .find(|(g, _)| g.eq_ignore_ascii_case(guid))
            .map(|(_, nome)| nome)
    });

    Dados {
        versao: env!("CARGO_PKG_VERSION").to_string(),
        windows: texto("ProductName").unwrap_or_else(|| "(não deu para ler)".to_string()),
        build: match (texto("CurrentBuildNumber"), numero("UBR")) {
            (Some(b), Some(ubr)) => format!("{}.{}", b, ubr),
            (Some(b), None) => b,
            _ => maquina.build_do_windows.to_string(),
        },
        edicao: texto("EditionID").unwrap_or_else(|| "(não deu para ler)".to_string()),
        arquitetura_do_windows: std::env::var("PROCESSOR_ARCHITECTURE")
            .unwrap_or_else(|_| "(não deu para ler)".to_string()),
        arquitetura_do_processo: if cfg!(target_pointer_width = "64") {
            "64 bits"
        } else {
            "32 bits"
        },
        elevado: registry::is_elevated(),
        nucleos_fisicos: Some(num_cpus::get_physical()),
        gpu: perfil.gpu_name.clone(),
        ram_gb: perfil.total_ram_gb,
        maquina,
        plano_ativo,
        nome_do_plano_ativo,
    }
}

/// Escreve o cabeçalho no registro, numa thread própria.
///
/// FORA DA ABERTURA DE PROPÓSITO. A coleta chama PowerShell e `powercfg`, e a
/// 1.7 gastou uma versão inteira derrubando o tempo de abertura de 3,7 s para
/// 1,2 s. Devolver parte disso para escrever um cabeçalho que ninguém lê na
/// hora seria desfazer aquele trabalho pelo motivo errado — o cabeçalho existe
/// para ser lido DEPOIS, quando algo deu errado.
///
/// Chega no arquivo alguns segundos depois da primeira linha, e isso não
/// atrapalha: cada linha do registro tem horário próprio.
pub fn anotar_em_segundo_plano() {
    std::thread::spawn(|| {
        let bloco = montar(&coletar());

        // Linha a linha, e não um bloco só, para cada uma sair com o horário e
        // o nível que o resto do arquivo usa — um bloco cru no meio de linhas
        // datadas é o que faz alguém achar que o arquivo está corrompido.
        for linha in bloco.lines() {
            crate::utils::Logger::info(linha);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::windows::planoenergia::{FabricanteDaCpu, Maquina};

    fn dados_de_teste() -> Dados {
        Dados {
            versao: "2.0.0".to_string(),
            windows: "Windows 10 Pro".to_string(),
            build: "19045.4046".to_string(),
            edicao: "Professional".to_string(),
            arquitetura_do_windows: "AMD64".to_string(),
            arquitetura_do_processo: "64 bits",
            elevado: true,
            maquina: Maquina {
                notebook: false,
                tem_bateria: false,
                fabricante_da_cpu: FabricanteDaCpu::Intel,
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
    fn o_cabecalho_traz_o_que_o_atendimento_precisa() {
        let texto = montar(&dados_de_teste());

        for esperado in [
            "2.0.0",
            "Windows 10 Pro",
            "19045.4046",
            "Professional",
            "AMD64",
            "64 bits",
            "Intel(R) Core(TM) i3-10100F",
            "NVIDIA GeForce GTX 1650",
            "7.9 GB",
            "desktop",
            "Equilibrado",
            "381b4222-f694-41f0-9685-ff5bb260df2e",
        ] {
            assert!(
                texto.contains(esperado),
                "o cabeçalho não traz `{}`:\n{}",
                esperado,
                texto
            );
        }
    }

    #[test]
    fn as_duas_arquiteturas_aparecem_separadas() {
        // Um processo de 32 bits num Windows de 64 lê o registro pelo espelho
        // WOW6432Node e enxerga outra máquina. Mostrar só uma das duas esconde
        // exatamente a causa que este par existe para revelar.
        let texto = montar(&dados_de_teste());

        assert!(texto.contains("Arquitetura do Windows"));
        assert!(texto.contains("Arquitetura do processo"));
    }

    #[test]
    fn leitura_que_falhou_aparece_escrita() {
        // Campo em branco faz o atendimento supor, e a suposição mais comum é a
        // benigna — que é a errada. A lacuna precisa estar escrita.
        let mut d = dados_de_teste();
        d.nome_do_plano_ativo = None;
        d.plano_ativo = None;
        d.nucleos_fisicos = None;

        let texto = montar(&d);

        assert_eq!(texto.matches("(não deu para ler)").count(), 3);
    }

    #[test]
    fn campo_vazio_conta_como_nao_lido() {
        // O registro devolve string vazia com mais frequência do que se imagina,
        // e uma linha "Edição:" sozinha é pior que dizer que não foi lida.
        assert_eq!(ou_nao_lido(&Some("  ".to_string())), "(não deu para ler)");
        assert_eq!(ou_nao_lido(&Some("Pro".to_string())), "Pro");
        assert_eq!(ou_nao_lido(&None), "(não deu para ler)");
    }

    #[test]
    fn o_cabecalho_nao_leva_nada_que_identifique_a_pessoa() {
        // MESMA REGRA DO `suporte.rs`, e aqui ela é mais importante: este bloco
        // vai dentro de um ARQUIVO que o cliente manda inteiro, sem ler. O
        // cabeçalho descreve uma máquina, não um dono.
        //
        // A trava é sobre a ESTRUTURA: nenhum campo do `Dados` pode carregar
        // nome de usuário ou caminho de perfil. Se alguém acrescentar um, este
        // teste reprova.
        let texto = montar(&dados_de_teste());

        for proibido in ["\\Users\\", "C:\\Users", "USERNAME", "USERPROFILE"] {
            assert!(
                !texto.contains(proibido),
                "o cabeçalho carrega `{}`",
                proibido
            );
        }
    }

    /// Imprime o cabeçalho desta máquina. Só lê, não escreve nada.
    ///
    ///   cargo test --lib cabecalho -- --ignored --nocapture
    #[test]
    #[ignore]
    fn cabecalho_desta_maquina() {
        println!("\n{}", montar(&coletar()));
    }

    #[test]
    fn notebook_aparece_com_bateria_e_modern_standby() {
        let mut d = dados_de_teste();
        d.maquina.notebook = true;
        d.maquina.tem_bateria = true;
        d.maquina.modern_standby = true;

        let texto = montar(&d);

        assert!(texto.contains("notebook"));
        assert!(texto.contains("Bateria                 sim"));
        assert!(texto.contains("Modern Standby          sim"));
    }
}
