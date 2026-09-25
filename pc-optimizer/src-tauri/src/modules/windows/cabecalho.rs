// O cabeçalho do registro: em QUE máquina aconteceu (elevado? notebook? build? plano ativo?). Diferente do
// `suporte.rs`, vai no arquivo, sem limite de tamanho, para quando o cliente manda o arquivo inteiro. NADA QUE
// IDENTIFIQUE A PESSOA: sem nome de usuário, caminho de perfil ou número de série.

use super::{hardware, planoenergia, power, registry};

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

/// `None` vira "não deu para ler": campo em branco faz o atendimento supor o benigno, que é o errado.
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

    // As DUAS arquiteturas: processo de 32 bits num Windows de 64 lê o `WOW6432Node` e enxerga outra máquina.
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

pub fn coletar() -> Dados {
    let perfil = hardware::profile();
    let maquina = planoenergia::detectar();

    let texto = |nome: &str| -> Option<String> {
        registry::read_text("HKLM", r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", nome)
            .ok()
            .flatten()
    };

    // O UBR é DWORD e o `CurrentBuildNumber` é texto, na mesma chave: lido como texto, o build saía "19045" e
    // não "19045.4046".
    let numero = |nome: &str| -> Option<u32> {
        use crate::modules::changelog::PreviousValue;

        match registry::read("HKLM", r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", nome) {
            Ok(PreviousValue::Dword(v)) => Some(v),
            _ => None,
        }
    };

    let plano_ativo = power::active_scheme().ok();

    // O nome sai do `powercfg`, no idioma do cliente: é o nome que ele vê no painel.
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

/// FORA DA ABERTURA: custa PowerShell e `powercfg`, e o cabeçalho existe para ser lido depois. Cada linha tem
/// horário próprio.
pub fn anotar_em_segundo_plano() {
    std::thread::spawn(|| {
        let bloco = montar(&coletar());

        // Linha a linha, com o horário e o nível do resto do arquivo.
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
        let texto = montar(&dados_de_teste());

        assert!(texto.contains("Arquitetura do Windows"));
        assert!(texto.contains("Arquitetura do processo"));
    }

    #[test]
    fn leitura_que_falhou_aparece_escrita() {
        let mut d = dados_de_teste();
        d.nome_do_plano_ativo = None;
        d.plano_ativo = None;
        d.nucleos_fisicos = None;

        let texto = montar(&d);

        assert_eq!(texto.matches("(não deu para ler)").count(), 3);
    }

    #[test]
    fn campo_vazio_conta_como_nao_lido() {
        assert_eq!(ou_nao_lido(&Some("  ".to_string())), "(não deu para ler)");
        assert_eq!(ou_nao_lido(&Some("Pro".to_string())), "Pro");
        assert_eq!(ou_nao_lido(&None), "(não deu para ler)");
    }

    #[test]
    fn o_cabecalho_nao_leva_nada_que_identifique_a_pessoa() {
        // A trava é sobre a ESTRUTURA: nenhum campo de `Dados` pode carregar nome de usuário ou caminho de perfil.
        let texto = montar(&dados_de_teste());

        for proibido in ["\\Users\\", "C:\\Users", "USERNAME", "USERPROFILE"] {
            assert!(
                !texto.contains(proibido),
                "o cabeçalho carrega `{}`",
                proibido
            );
        }
    }

    /// `cargo test --lib cabecalho -- --ignored --nocapture`
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
