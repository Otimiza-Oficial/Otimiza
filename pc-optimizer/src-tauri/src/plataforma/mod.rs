// A plataforma da 3.3: o que deixa de ser "do FiveM" e passa a valer para qualquer jogo. Orquestra os módulos que já
// existem (`modules::*`), sem mover a lógica deles: cada peça aqui é a fronteira que a tela nova consome.

pub mod banda;
pub mod bandeja;
pub mod base_do_jogo;
pub mod capas;
pub mod compat;
pub mod comunidade;
pub mod contador_fivem;
pub mod cores;
pub mod cs2;
pub mod discord;
pub mod energia_jogo;
pub mod feedback;
pub mod flags;
pub mod frameboost;
pub mod guarda_termica;
pub mod fov_fivem;
pub mod fundos;
pub mod hardware;
pub mod historico;
// Sem os comandos `tela_*` ainda (moram em `commands/`, outra frente): até lá só o desfazer do histórico chama o
// módulo. Tirar o `allow` quando os comandos entrarem.
#[allow(dead_code)]
pub mod hz;
pub mod fb_por_jogo;
pub mod fila;
pub mod placa_prova;
pub mod rede_partida;
pub mod servidores_fivem;
pub mod jogo_liso;
pub mod jogos;
pub mod lab;
pub mod limpeza;
pub mod memoria_jogo;
pub mod metricas;
pub mod mira;
pub mod mouse_taxa;
pub mod operadora;
pub mod overlay;
pub mod partida;
pub mod por_cima;
pub mod ritmo_estavel;
pub mod rota;
pub mod rota_historico;
pub mod sessao;
pub mod turbo;
pub mod wifi;
pub mod wifi_varredura;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct RecursoAvaliado {
    pub recurso: compat::Recurso,
    pub nome: &'static str,
    pub suporte: compat::Suporte,
}

#[derive(Debug, Clone, Serialize)]
pub struct Compatibilidade {
    pub hardware: hardware::PerfilDeHardware,
    pub recursos: Vec<RecursoAvaliado>,
    pub frame_boost_auto: Option<RecursoAvaliado>,
}

/// O que dá para usar neste PC agora, sem jogo aberto (o jogo refina o contexto quando a sessão o detectar).
pub fn compatibilidade(
    hw: hardware::PerfilDeHardware,
    sobreposicao_barrada: Option<String>,
) -> Compatibilidade {
    let c = compat::Contexto {
        hz_do_monitor: hw.tela_principal().map(|t| t.hz_atual),
        sobreposicao_barrada,
        ..compat::Contexto::so_hardware(hw.placa.fabricante, hw.placa.familia)
    };
    let fab = hw.placa.fabricante;
    let avaliado = |r: compat::Recurso, s: compat::Suporte| RecursoAvaliado {
        recurso: r,
        nome: r.nome(fab),
        suporte: s,
    };

    let recursos = [
        compat::Recurso::FrameBoostSmoothMotion,
        compat::Recurso::FrameBoostAfmf,
        compat::Recurso::FrameBoostOtimiza,
        compat::Recurso::BaixaLatenciaDoDriver,
        compat::Recurso::LimitadorDoDriver,
        compat::Recurso::MiraNaTela,
        compat::Recurso::CoresDoMonitor,
    ]
    .into_iter()
    .map(|r| avaliado(r, compat::avaliar(r, &c)))
    .collect();

    Compatibilidade {
        frame_boost_auto: compat::melhor_frame_boost(&c).map(|(r, s)| avaliado(r, s)),
        hardware: hw,
        recursos,
    }
}
