// REMOÇÃO DE LIXO. Só o que o Windows e os programas jogam fora e esquecem: as pastas temporárias e os relatórios de
// erro. Nunca a Lixeira, Downloads, Documentos ou qualquer coisa da pessoa. Só arquivo com mais de 2 dias (o que um
// programa aberto agora usa é recente) e o que estiver em uso fica: o Windows recusa e a limpeza segue.
// Não dá FPS: libera disco, e o cartão diz isso.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Arquivo mais novo que isto fica: pode ser de um programa aberto agora.
pub const IDADE_MINIMA: Duration = Duration::from_secs(2 * 24 * 3600);
/// Teto de arquivos por pasta numa passada: pasta temporária com milhões de arquivos não trava a tela.
const LIMITE_DE_ARQUIVOS: usize = 200_000;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: &'static str,
    pub nome: &'static str,
    pub explica: &'static str,
    pub bytes: u64,
    pub arquivos: u64,
    pub precisa_admin: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Resultado {
    pub bytes_liberados: u64,
    pub apagados: u64,
    /// Em uso ou protegidos: ficaram.
    pub ficaram: u64,
}

struct Alvo {
    id: &'static str,
    nome: &'static str,
    explica: &'static str,
    precisa_admin: bool,
    pastas: fn() -> Vec<PathBuf>,
}

fn env(nome: &str) -> Option<PathBuf> {
    std::env::var_os(nome).map(PathBuf::from)
}

const ALVOS: &[Alvo] = &[
    Alvo {
        id: "temp_usuario",
        nome: "Temporários do seu usuário",
        explica: "O que os programas deixam na pasta TEMP e não apagam.",
        precisa_admin: false,
        pastas: || env("TEMP").into_iter().collect(),
    },
    Alvo {
        id: "temp_windows",
        nome: "Temporários do Windows",
        explica: "Restos de instalações e atualizações em Windows\\Temp.",
        precisa_admin: true,
        pastas: || {
            env("SystemRoot")
                .map(|r| r.join("Temp"))
                .into_iter()
                .collect()
        },
    },
    Alvo {
        id: "relatorios_de_erro",
        nome: "Relatórios de erro",
        explica: "Os relatórios que o Windows guarda quando um programa trava.",
        precisa_admin: true,
        pastas: || {
            env("ProgramData")
                .map(|p| {
                    let wer = p.join("Microsoft").join("Windows").join("WER");
                    vec![wer.join("ReportArchive"), wer.join("ReportQueue")]
                })
                .unwrap_or_default()
        },
    },
];

/// **Pura.** Só pasta temporária ou de relatório do Windows: um caminho errado aqui apagaria coisa da pessoa.
pub fn pasta_permitida(p: &Path) -> bool {
    let t = p.to_string_lossy().to_lowercase().replace('/', "\\");
    let proibido = [
        "\\downloads",
        "\\documents",
        "\\desktop",
        "\\pictures",
        "\\$recycle.bin",
        "\\onedrive",
    ];
    if proibido.iter().any(|x| t.contains(x)) {
        return false;
    }
    t.ends_with("\\temp")
        || t.ends_with("\\wer\\reportarchive")
        || t.ends_with("\\wer\\reportqueue")
}

/// **Pura.** Velho o bastante para ser lixo.
pub fn velho(modificado: SystemTime, agora: SystemTime) -> bool {
    agora
        .duration_since(modificado)
        .is_ok_and(|d| d >= IDADE_MINIMA)
}

/// Os arquivos velhos dentro de `raiz`, sem seguir atalhos (junções e links) para fora dela.
fn varrer(raiz: &Path, mut cada: impl FnMut(&Path, u64)) {
    let agora = SystemTime::now();
    let mut pilha = vec![raiz.to_path_buf()];
    let mut vistos = 0usize;
    while let Some(d) = pilha.pop() {
        let Ok(it) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in it.flatten() {
            let Ok(meta) = std::fs::symlink_metadata(e.path()) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                pilha.push(e.path());
            } else if meta.is_file() {
                vistos += 1;
                if vistos > LIMITE_DE_ARQUIVOS {
                    return;
                }
                if meta.modified().is_ok_and(|m| velho(m, agora)) {
                    cada(&e.path(), meta.len());
                }
            }
        }
    }
}

fn admin() -> bool {
    #[cfg(windows)]
    {
        crate::modules::windows::registry::is_elevated()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Quanto cada item libera agora. Só lê.
pub fn medir() -> Vec<Item> {
    ALVOS
        .iter()
        .map(|a| {
            let (mut bytes, mut arquivos) = (0u64, 0u64);
            for p in (a.pastas)().iter().filter(|p| pasta_permitida(p)) {
                varrer(p, |_, n| {
                    bytes += n;
                    arquivos += 1;
                });
            }
            Item {
                id: a.id,
                nome: a.nome,
                explica: a.explica,
                bytes,
                arquivos,
                precisa_admin: a.precisa_admin && !admin(),
            }
        })
        .collect()
}

/// Apaga os arquivos velhos de um item. O que estiver em uso fica (o Windows recusa) e entra em `ficaram`.
pub fn limpar(id: &str) -> Result<Resultado, String> {
    let a = ALVOS
        .iter()
        .find(|a| a.id == id)
        .ok_or("Item de limpeza desconhecido.")?;
    if a.precisa_admin && !admin() {
        return Err("Esta limpeza exige abrir o Otimiza como administrador.".to_string());
    }
    let mut r = Resultado {
        bytes_liberados: 0,
        apagados: 0,
        ficaram: 0,
    };
    for p in (a.pastas)().iter().filter(|p| pasta_permitida(p)) {
        let mut lista = Vec::new();
        varrer(p, |c, n| lista.push((c.to_path_buf(), n)));
        for (c, n) in lista {
            match std::fs::remove_file(&c) {
                Ok(()) => {
                    r.apagados += 1;
                    r.bytes_liberados += n;
                }
                Err(_) => r.ficaram += 1,
            }
        }
    }
    // Apagar sem rastro deixava a pergunta "o que sumiu daqui?" sem resposta (09/10: a pasta de quadros gravados para
    // o laboratório do Frame Boost esvaziou e o log não dizia nada).
    crate::utils::Logger::info(&format!(
        "remoção de lixo: {} — {} arquivo(s), {} MB liberados, {} ficaram (em uso ou protegidos), pastas {:?}",
        a.nome,
        r.apagados,
        r.bytes_liberados / 1_048_576,
        r.ficaram,
        (a.pastas)()
    ));
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_pasta_temporaria_ou_de_relatorio() {
        assert!(pasta_permitida(Path::new(
            r"C:\Users\SnyX\AppData\Local\Temp"
        )));
        assert!(pasta_permitida(Path::new(r"C:\Windows\Temp")));
        assert!(pasta_permitida(Path::new(
            r"C:\ProgramData\Microsoft\Windows\WER\ReportArchive"
        )));
        assert!(!pasta_permitida(Path::new(r"C:\Users\SnyX\Downloads")));
        assert!(
            !pasta_permitida(Path::new(r"C:\Users\SnyX\Documents\Temp")),
            "Temp dentro de Documentos é da pessoa"
        );
        assert!(!pasta_permitida(Path::new(r"C:\$Recycle.Bin")));
        assert!(!pasta_permitida(Path::new(r"C:\Users\SnyX")));
        assert!(!pasta_permitida(Path::new(r"C:\")));
    }

    #[test]
    fn arquivo_recente_fica() {
        let agora = SystemTime::now();
        assert!(!velho(agora - Duration::from_secs(3600), agora));
        assert!(velho(agora - Duration::from_secs(3 * 24 * 3600), agora));
        assert!(
            !velho(agora + Duration::from_secs(60), agora),
            "data no futuro não é lixo"
        );
    }

    #[test]
    fn os_alvos_apontam_so_para_pastas_permitidas() {
        for a in ALVOS {
            for p in (a.pastas)() {
                assert!(pasta_permitida(&p), "{} aponta para {}", a.id, p.display());
            }
        }
    }
}

#[cfg(test)]
mod na_maquina {
    /// Só mede, não apaga: `cargo test --lib -- --ignored limpeza::na_maquina --nocapture`.
    #[test]
    #[ignore]
    fn mede_sem_apagar() {
        for i in super::medir() {
            println!(
                "{} · {} MB · {} arquivos · admin={}",
                i.nome,
                i.bytes / 1_048_576,
                i.arquivos,
                i.precisa_admin
            );
        }
    }
}
