// AS CONFIGS DE TREINO DO CS2. Dois arquivos `.cfg` na pasta `cfg` do jogo, os mesmos que a comunidade troca em fórum:
// `exec otimiza_treino` (granada com a trajetória, munição infinita, dinheiro e rodada sem fim) e `exec otimiza_mira`
// (bots parados que renascem, para treinar flick). Só valem num servidor da própria pessoa: `sv_cheats` é recusado em
// servidor oficial, e o VAC não olha arquivo de configuração. Nada é executado sozinho: a pessoa digita o `exec`.
//
// O Otimiza nunca sobrescreve um arquivo com esse nome que não seja dele (a primeira linha diz de quem é), e remover
// apaga só os dele.

use std::path::{Path, PathBuf};

/// A primeira linha de todo arquivo do Otimiza: sem ela, o arquivo é da pessoa e fica.
pub const ASSINATURA: &str = "// Otimiza: config de treino.";

pub struct Treino {
    pub arquivo: &'static str,
    pub conteudo: &'static str,
}

pub const TREINOS: &[Treino] = &[
    Treino {
        arquivo: "otimiza_treino.cfg",
        conteudo: "// Otimiza: config de treino. Num servidor seu (Jogar > Praticar, ou \"map de_mirage\" no console): exec otimiza_treino
sv_cheats 1
bot_kick
mp_limitteams 0
mp_autoteambalance 0
mp_warmup_end
mp_freezetime 0
mp_roundtime 60
mp_roundtime_defuse 60
mp_buytime 9999
mp_buy_anywhere 1
mp_maxmoney 60000
mp_startmoney 60000
mp_respawn_on_death_ct 1
mp_respawn_on_death_t 1
sv_infinite_ammo 1
ammo_grenade_limit_total 5
sv_grenade_trajectory_prac_pipreview 1
sv_grenade_trajectory_prac_trailtime 8
sv_showimpacts 1
sv_showimpacts_time 4
mp_restartgame 1
",
    },
    Treino {
        arquivo: "otimiza_mira.cfg",
        conteudo: "// Otimiza: config de treino. Bots parados que renascem, para treinar flick: exec otimiza_mira
sv_cheats 1
bot_kick
mp_limitteams 0
mp_autoteambalance 0
mp_warmup_end
mp_freezetime 0
mp_roundtime 60
mp_roundtime_defuse 60
mp_buy_anywhere 1
mp_maxmoney 60000
mp_startmoney 60000
mp_respawn_on_death_ct 1
mp_respawn_on_death_t 1
sv_infinite_ammo 1
bot_difficulty 0
bot_dont_shoot 1
bot_stop 1
bot_add_ct
bot_add_ct
bot_add_ct
bot_add_ct
bot_add_ct
bot_add_ct
bot_add_ct
bot_add_ct
mp_restartgame 1
",
    },
];

/// Pura: a pasta `cfg` dentro da pasta do CS2.
pub fn pasta_cfg(pasta_do_jogo: &Path) -> PathBuf {
    pasta_do_jogo.join("game").join("csgo").join("cfg")
}

/// Pura: o arquivo é do Otimiza (pode ser trocado ou apagado).
pub fn e_do_otimiza(texto: &str) -> bool {
    texto.starts_with(ASSINATURA)
}

/// Escreve os arquivos de treino. Recusa se já existe um com o mesmo nome que não seja do Otimiza.
pub fn instalar(cfg: &Path) -> Result<(), String> {
    if !cfg.is_dir() {
        return Err(format!("A pasta de configuração do CS2 não existe: {}", cfg.display()));
    }
    for t in TREINOS {
        let alvo = cfg.join(t.arquivo);
        if let Ok(atual) = std::fs::read_to_string(&alvo) {
            if !e_do_otimiza(&atual) {
                return Err(format!(
                    "Já existe um {} seu na pasta do CS2: não mexi nele. Renomeie o seu e tente de novo.",
                    t.arquivo
                ));
            }
        }
    }
    for t in TREINOS {
        std::fs::write(cfg.join(t.arquivo), t.conteudo)
            .map_err(|e| format!("Não consegui escrever na pasta do CS2: {e}"))?;
    }
    Ok(())
}

/// Apaga só os arquivos de treino que são do Otimiza.
pub fn remover(cfg: &Path) -> Result<(), String> {
    for t in TREINOS {
        let alvo = cfg.join(t.arquivo);
        if std::fs::read_to_string(&alvo).is_ok_and(|x| e_do_otimiza(&x)) {
            std::fs::remove_file(&alvo).map_err(|e| format!("Não consegui apagar {}: {e}", alvo.display()))?;
        }
    }
    Ok(())
}

/// Os arquivos do Otimiza estão lá.
pub fn instalado(cfg: &Path) -> bool {
    TREINOS
        .iter()
        .all(|t| std::fs::read_to_string(cfg.join(t.arquivo)).is_ok_and(|x| e_do_otimiza(&x)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pasta() -> PathBuf {
        let p = std::env::temp_dir().join(format!("otimiza-cs2-treino-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn todo_arquivo_comeca_com_a_assinatura_e_so_usa_comando_de_servidor_proprio() {
        for t in TREINOS {
            assert!(e_do_otimiza(t.conteudo), "{}", t.arquivo);
            assert!(t.arquivo.ends_with(".cfg"));
            // Nada que mude a pessoa fora do servidor de treino: tecla, sensibilidade, vídeo.
            for proibido in ["bind ", "sensitivity", "unbindall", "host_writeconfig", "cl_"] {
                assert!(!t.conteudo.contains(proibido), "{} usa {}", t.arquivo, proibido);
            }
        }
    }

    #[test]
    fn instala_remove_e_nunca_toca_no_arquivo_da_pessoa() {
        let cfg = pasta();
        instalar(&cfg).unwrap();
        assert!(instalado(&cfg));
        remover(&cfg).unwrap();
        assert!(!instalado(&cfg));
        assert!(!cfg.join("otimiza_treino.cfg").exists());

        std::fs::write(cfg.join("otimiza_mira.cfg"), "bind mouse5 noclip\n").unwrap();
        assert!(instalar(&cfg).is_err(), "o arquivo da pessoa barra a instalação");
        assert!(!cfg.join("otimiza_treino.cfg").exists(), "nada é escrito pela metade");
        remover(&cfg).unwrap();
        assert_eq!(std::fs::read_to_string(cfg.join("otimiza_mira.cfg")).unwrap(), "bind mouse5 noclip\n");
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn pasta_inexistente_e_recusada() {
        assert!(instalar(Path::new("Z:/nao/existe/cfg")).is_err());
    }
}
