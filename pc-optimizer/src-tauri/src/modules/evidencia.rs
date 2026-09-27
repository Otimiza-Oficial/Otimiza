// Núcleo de Sessão e Evidência (MASTER-PLAN, T1.2): a regra de quando duas medições podem ser comparadas mora
// aqui, uma vez só. Antes cada lugar tinha a sua: a prova alternada recusava medidor misturado, a prova antiga
// não. A série automática fica de fora, e o porquê está no fim do arquivo.
//
// Valor que não foi lido (`None`) não é mudança: medição antiga ou jogo sem arquivo de configuração conhecido
// continuam comparáveis, como já eram.

use crate::modules::medicoes::{Fonte, GeracaoNaPartida};

/// Por que duas medições não medem só a otimização. A ordem é a de gravidade, e a tela usa o primeiro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Motivo {
    /// Gráfico diferente: parte da diferença é qualidade, não o Otimiza.
    ConfiguracaoDoJogoMudou,
    /// Gerador de quadros divide a placa com o jogo.
    GeracaoMudou,
    /// PresentMon de um lado e o canal antigo do outro.
    MedidorMudou,
}

/// O que precisa ser igual dos dois lados de uma comparação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contexto<'a> {
    pub configuracao_do_jogo: Option<&'a str>,
    pub geracao: Option<GeracaoNaPartida>,
    pub fonte: Option<Fonte>,
}

fn mudou<T: PartialEq>(valores: impl Iterator<Item = Option<T>>) -> bool {
    let lidos: Vec<T> = valores.flatten().collect();
    lidos.windows(2).any(|par| par[0] != par[1])
}

/// Tudo o que mudou entre os contextos, do mais grave ao menos. Vazio: comparação justa.
pub fn diferencas<'a>(contextos: &[Contexto<'a>]) -> Vec<Motivo> {
    let mut motivos = Vec::new();
    if mudou(contextos.iter().map(|c| c.configuracao_do_jogo)) {
        motivos.push(Motivo::ConfiguracaoDoJogoMudou);
    }
    if mudou(contextos.iter().map(|c| c.geracao)) {
        motivos.push(Motivo::GeracaoMudou);
    }
    if mudou(contextos.iter().map(|c| c.fonte)) {
        motivos.push(Motivo::MedidorMudou);
    }
    motivos
}

// A série automática (portão, regressão, deriva, governador) NÃO passa por aqui, de propósito:
// - configuração: o item que o portão vigia é o próprio perfil gráfico (`config_jogo_*`), que muda o arquivo e a
//   impressão digital. Separar a série pela configuração tiraria o "antes" dele e o portão nunca decidiria.
// - medidor: o canal antigo conta todo Present do processo no tempo do relógio, o PresentMon só os quadros do jogo
//   na cadeia principal; não são a mesma conta. A série já mistura os dois (a medição cai no canal antigo quando o
//   PresentMon falha); separar zeraria o portão de todo cliente na atualização. Decidir isso pede os dois canais
//   medidos na mesma partida (MASTER-PLAN-TOP10, T1.2).

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(configuracao: Option<&str>, geracao: Option<GeracaoNaPartida>, fonte: Option<Fonte>) -> Contexto<'_> {
        Contexto { configuracao_do_jogo: configuracao, geracao, fonte }
    }

    #[test]
    fn contextos_iguais_comparam() {
        let a = ctx(Some("abc"), Some(GeracaoNaPartida::NenhumaVisivel), Some(Fonte::PresentMon));
        assert!(diferencas(&[a, a, a]).is_empty());
    }

    #[test]
    fn cada_diferenca_tem_o_seu_motivo_e_a_ordem_e_a_de_gravidade() {
        let a = ctx(Some("abc"), Some(GeracaoNaPartida::NenhumaVisivel), Some(Fonte::PresentMon));
        let b = ctx(Some("xyz"), Some(GeracaoNaPartida::LosslessScaling), Some(Fonte::CanalAntigo));
        assert_eq!(
            diferencas(&[a, b]),
            vec![Motivo::ConfiguracaoDoJogoMudou, Motivo::GeracaoMudou, Motivo::MedidorMudou]
        );
        assert_eq!(diferencas(&[a, ctx(Some("abc"), a.geracao, Some(Fonte::CanalAntigo))]), vec![Motivo::MedidorMudou]);
    }

    #[test]
    fn o_que_nao_foi_lido_nao_e_mudanca() {
        let a = ctx(Some("abc"), Some(GeracaoNaPartida::NenhumaVisivel), Some(Fonte::PresentMon));
        let sem_leitura = ctx(None, None, None);
        assert!(diferencas(&[a, sem_leitura, a]).is_empty());
    }

    /// A regra não pode voltar a nascer espalhada: comparar a configuração do jogo à mão fora deste arquivo falha.
    #[test]
    fn ninguem_compara_contexto_fora_daqui() {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut achados = Vec::new();
        let mut pastas = vec![raiz];
        while let Some(pasta) = pastas.pop() {
            for entrada in std::fs::read_dir(&pasta).unwrap().flatten() {
                let p = entrada.path();
                if p.is_dir() {
                    pastas.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("evidencia.rs") {
                    let texto = std::fs::read_to_string(&p).unwrap();
                    let codigo = texto.split("#[cfg(test)]").next().unwrap_or("");
                    for (n, linha) in codigo.lines().enumerate() {
                        let comparacao_a_mao = linha.contains("configuracao_do_jogo")
                            && (linha.contains("!=") || linha.contains("mudou("));
                        let fonte_a_mao = linha.contains("Fonte::PresentMon") && !p.ends_with("medicoes.rs");
                        if comparacao_a_mao || fonte_a_mao || linha.trim_start().starts_with("fn mudou<") {
                            achados.push(format!("{}:{}", p.display(), n + 1));
                        }
                    }
                }
            }
        }
        assert!(achados.is_empty(), "use evidencia::diferencas: {achados:?}");
    }
}
