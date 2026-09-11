// Medições de quadros feitas sozinhas, durante as partidas
//
// POR QUE ISTO EXISTE
//
// A prova de antes e depois (`prova.rs`) só existe quando o cliente lembra de
// medir — e quase ninguém lembra. O cliente que pediu reembolso porque "o FiveM
// continuou igual" nunca mediu nada: nem ele nem o Otimiza tinham um número
// para mostrar sobre as partidas dele.
//
// Aqui o vigia mede sozinho. Com o jogo em primeiro plano há alguns minutos e o
// Otimiza aberto como administrador, ele escuta por vinte segundos o mesmo canal
// de eventos do Windows que a medição manual usa — sem tocar no jogo, ver
// `frames.rs` — e guarda FPS, 1% piores quadros e engasgos. No máximo uma vez a
// cada vinte minutos de partida.
//
// O QUE ELE NÃO FAZ
//
// Não compara uma medição com outra e não diz que houve ganho. Duas medições
// automáticas foram feitas em lugares diferentes do jogo, e comparar menu com rua
// movimentada é fabricar prova — o cabeçalho de `prova.rs` conta por quê. Ele
// guarda o número, a hora e quantas mudanças do Otimiza estavam aplicadas
// naquele momento, e a tela mostra lado a lado, sem conclusão.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Uma medição feita pelo vigia, sem ninguém pedir.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedicaoAutomatica {
    /// Processo medido.
    pub jogo: String,
    pub quando: u64,
    pub fps: f64,
    pub low_1pct: f64,
    pub engasgos_por_minuto: f64,
    pub segundos: f64,
    /// Falso quando a amostra foi curta demais para os detalhes significarem
    /// alguma coisa.
    pub confiavel: bool,
    /// Quantas mudanças do Otimiza estavam aplicadas quando a medição foi feita.
    pub mudancas_aplicadas: usize,
}

/// Quantas medições ficam guardadas. Sessenta são semanas de partidas a uma
/// medição a cada vinte minutos, e o arquivo continua pequeno.
pub const GUARDADAS: usize = 60;

/// Quanto tempo cada medição escuta o canal de eventos. Vinte segundos é o
/// mesmo da prova manual: menos que isso o 1% pior não significa nada.
pub const SEGUNDOS_DE_MEDICAO: u64 = 20;

/// Quanto tempo o jogo precisa estar em primeiro plano antes da primeira medição.
///
/// Os primeiros minutos são carregamento, menu e tela de conexão — que rodam a
/// centenas de quadros e não dizem nada sobre a partida.
pub const SEGUNDOS_ANTES_DA_PRIMEIRA: u64 = 180;

/// Intervalo mínimo entre duas medições do mesmo jogo aberto.
pub const SEGUNDOS_ENTRE_MEDICOES: u64 = 20 * 60;

/// De quantos em quantos segundos o vigia olha se há jogo em primeiro plano.
///
/// Mais espaçado que o vigia do modo jogo, de propósito: a detecção consulta o
/// uso do motor 3D pelo PowerShell, e medir o jogo não pode virar peso no jogo.
pub const SEGUNDOS_ENTRE_OLHADAS: u64 = 30;

fn caminho() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    base.join("pc-optimizer").join("medicoes.json")
}

/// Lê o histórico.
///
/// Arquivo que não existe é histórico vazio. Arquivo que existe e não dá para
/// ler é `Err`: "nenhuma medição" sobre um arquivo ilegível seria a lista vazia
/// fingindo ser resposta.
pub fn ler_de(caminho: &Path) -> Result<Vec<MedicaoAutomatica>, String> {
    match fs::read_to_string(caminho) {
        Ok(bruto) => serde_json::from_str(&bruto)
            .map_err(|e| format!("o histórico de medições está ilegível: {}", e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("não consegui ler o histórico de medições: {}", e)),
    }
}

pub fn ler() -> Result<Vec<MedicaoAutomatica>, String> {
    ler_de(&caminho())
}

/// Acrescenta uma medição e guarda só as últimas `GUARDADAS`.
///
/// Histórico ilegível NÃO é sobrescrito: a medição nova é descartada e o arquivo
/// fica como está. Gravar por cima apagaria, sem aviso, tudo o que já tinha sido
/// medido.
pub fn registrar_em(caminho: &Path, medicao: MedicaoAutomatica) -> Result<(), String> {
    let mut todas = ler_de(caminho)?;
    todas.push(medicao);

    let excesso = todas.len().saturating_sub(GUARDADAS);
    todas.drain(..excesso);

    if let Some(pasta) = caminho.parent() {
        fs::create_dir_all(pasta)
            .map_err(|e| format!("não consegui criar a pasta de dados: {}", e))?;
    }

    let bruto = serde_json::to_string(&todas)
        .map_err(|e| format!("não consegui preparar o histórico de medições: {}", e))?;

    fs::write(caminho, bruto)
        .map_err(|e| format!("não consegui gravar o histórico de medições: {}", e))
}

pub fn registrar(medicao: MedicaoAutomatica) -> Result<(), String> {
    registrar_em(&caminho(), medicao)
}

/// Se é hora de medir. PURA.
///
/// `aberto_ha`: segundos com o mesmo jogo em primeiro plano, sem interrupção.
/// `desde_a_ultima`: segundos desde a última tentativa neste jogo aberto, ou
/// `None` quando ainda não houve nenhuma.
pub fn hora_de_medir(aberto_ha: u64, desde_a_ultima: Option<u64>) -> bool {
    match desde_a_ultima {
        None => aberto_ha >= SEGUNDOS_ANTES_DA_PRIMEIRA,
        Some(segundos) => segundos >= SEGUNDOS_ENTRE_MEDICOES,
    }
}

/// O que o vigia lembra de uma olhada para a outra.
///
/// Recebe o relógio de fora, e não lê `Instant` por dentro, para cada transição
/// ser provada em teste sem esperar três minutos.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Acompanhamento {
    pid: Option<u32>,
    desde: u64,
    ultima: Option<u64>,
}

impl Acompanhamento {
    /// Uma olhada: o PID do jogo em primeiro plano agora (ou nenhum) e o relógio.
    /// Devolve se é hora de medir.
    ///
    /// Sem jogo, ou com OUTRO jogo, a contagem recomeça: tempo de menu de outro
    /// jogo, ou de área de trabalho, não conta como partida deste.
    pub fn observar(&mut self, pid: Option<u32>, agora: u64) -> bool {
        match pid {
            None => {
                *self = Acompanhamento::default();
                false
            }
            Some(pid) if self.pid != Some(pid) => {
                *self = Acompanhamento {
                    pid: Some(pid),
                    desde: agora,
                    ultima: None,
                };
                false
            }
            Some(_) => hora_de_medir(
                agora.saturating_sub(self.desde),
                self.ultima.map(|ultima| agora.saturating_sub(ultima)),
            ),
        }
    }

    /// Marca a tentativa, deu certo ou não.
    ///
    /// Marcar também a que falhou é de propósito: uma medição recusada — pelo
    /// anticheat, por outra medição em andamento — tentada de novo a cada meio
    /// minuto viraria ruído no registro e trabalho à toa durante a partida.
    pub fn tentado(&mut self, agora: u64) {
        self.ultima = Some(agora);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exemplo(fps: f64) -> MedicaoAutomatica {
        MedicaoAutomatica {
            jogo: "FiveM_b3258_GTAProcess.exe".to_string(),
            quando: 1_757_600_000,
            fps,
            low_1pct: fps / 2.0,
            engasgos_por_minuto: 3.0,
            segundos: 20.0,
            confiavel: true,
            mudancas_aplicadas: 4,
        }
    }

    fn pasta_de_teste(nome: &str) -> PathBuf {
        let unico = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        std::env::temp_dir().join(format!("otimiza-medicoes-{}-{}-{}", nome, std::process::id(), unico))
    }

    #[test]
    fn a_primeira_medicao_espera_o_carregamento() {
        assert!(!hora_de_medir(0, None));
        assert!(!hora_de_medir(SEGUNDOS_ANTES_DA_PRIMEIRA - 1, None));
        assert!(hora_de_medir(SEGUNDOS_ANTES_DA_PRIMEIRA, None));
    }

    #[test]
    fn depois_da_primeira_espera_o_intervalo() {
        assert!(!hora_de_medir(3600, Some(SEGUNDOS_ENTRE_MEDICOES - 1)));
        assert!(hora_de_medir(3600, Some(SEGUNDOS_ENTRE_MEDICOES)));
    }

    #[test]
    fn o_acompanhamento_mede_so_o_mesmo_jogo_aberto_sem_parar() {
        let mut vigia = Acompanhamento::default();

        // O jogo aparece: começa a contar, não mede ainda.
        assert!(!vigia.observar(Some(42), 1000));
        assert!(!vigia.observar(Some(42), 1000 + SEGUNDOS_ANTES_DA_PRIMEIRA - 30));

        // Passou o carregamento.
        assert!(vigia.observar(Some(42), 1000 + SEGUNDOS_ANTES_DA_PRIMEIRA));
        vigia.tentado(1000 + SEGUNDOS_ANTES_DA_PRIMEIRA);

        // Logo depois, não mede de novo.
        assert!(!vigia.observar(Some(42), 1000 + SEGUNDOS_ANTES_DA_PRIMEIRA + 60));

        // Vinte minutos depois, mede.
        let depois = 1000 + SEGUNDOS_ANTES_DA_PRIMEIRA + SEGUNDOS_ENTRE_MEDICOES;
        assert!(vigia.observar(Some(42), depois));
    }

    #[test]
    fn sair_do_jogo_ou_trocar_de_jogo_recomeca_a_contagem() {
        let mut vigia = Acompanhamento::default();
        assert!(!vigia.observar(Some(42), 0));
        assert!(vigia.observar(Some(42), SEGUNDOS_ANTES_DA_PRIMEIRA));

        // Outro processo em primeiro plano: tempo do anterior não conta.
        assert!(!vigia.observar(Some(7), SEGUNDOS_ANTES_DA_PRIMEIRA + 30));
        assert!(!vigia.observar(Some(7), SEGUNDOS_ANTES_DA_PRIMEIRA + 60));

        // Nenhum jogo: tudo zera.
        assert!(!vigia.observar(None, 10_000));
        assert_eq!(vigia, Acompanhamento::default());
    }

    #[test]
    fn historico_que_nao_existe_e_vazio_e_ilegivel_e_erro() {
        let pasta = pasta_de_teste("leitura");
        let arquivo = pasta.join("medicoes.json");

        assert_eq!(ler_de(&arquivo), Ok(Vec::new()));

        fs::create_dir_all(&pasta).unwrap();
        fs::write(&arquivo, "isto não é json").unwrap();
        let resultado = ler_de(&arquivo);
        let _ = fs::remove_dir_all(&pasta);

        assert!(resultado.is_err(), "arquivo ilegível virou lista vazia");
    }

    #[test]
    fn o_historico_guarda_so_as_ultimas() {
        let pasta = pasta_de_teste("teto");
        let arquivo = pasta.join("medicoes.json");

        for indice in 0..(GUARDADAS + 5) {
            registrar_em(&arquivo, exemplo(indice as f64)).unwrap();
        }

        let todas = ler_de(&arquivo).unwrap();
        let _ = fs::remove_dir_all(&pasta);

        assert_eq!(todas.len(), GUARDADAS);
        // As mais antigas saem, a mais recente fica por último.
        assert_eq!(todas.first().unwrap().fps, 5.0);
        assert_eq!(todas.last().unwrap().fps, (GUARDADAS + 4) as f64);
    }

    #[test]
    fn historico_ilegivel_nao_e_apagado_por_uma_medicao_nova() {
        let pasta = pasta_de_teste("ilegivel");
        let arquivo = pasta.join("medicoes.json");
        fs::create_dir_all(&pasta).unwrap();
        fs::write(&arquivo, "corrompido").unwrap();

        let resultado = registrar_em(&arquivo, exemplo(60.0));
        let conteudo = fs::read_to_string(&arquivo).unwrap();
        let _ = fs::remove_dir_all(&pasta);

        assert!(resultado.is_err());
        assert_eq!(conteudo, "corrompido", "a medição nova apagou o que estava lá");
    }
}
