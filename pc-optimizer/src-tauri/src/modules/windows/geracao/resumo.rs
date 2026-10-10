// O RESUMO DA SESSÃO: o "antes × depois" que o painel mostra quando o Frame Boost desliga. Só o que foi medido na
// própria sessão: o FPS do jogo com ele ligado, o que chegou à tela e quanto do FPS o jogo manteve (a guarda).
//
// O número da tela entra como MEDIDO só se o PresentMon contou quase a sessão inteira; senão a sessão toda vai pelo
// que o gerador apresentou, como ESTIMADO (misturar os dois daria um número que não é nenhum dos dois).

use serde::{Deserialize, Serialize};

/// Menos que isso não é sessão: ligou e desligou.
pub const MINIMO_S: f64 = 30.0;
/// A parte da sessão que o PresentMon precisa ter contado para o número da tela valer como medido.
pub const COBERTURA_MEDIDA: f64 = 0.9;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Acumulado {
    pub segundos: f64,
    soma_do_jogo: f64,
    soma_apresentados: f64,
    soma_exibidos: f64,
    segundos_exibidos: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resumo {
    pub segundos: f64,
    /// FPS do jogo com o Frame Boost ligado, média da sessão.
    pub fps_do_jogo: f64,
    /// Média do que chegou à tela (medido) ou do que o gerador apresentou (estimado).
    pub fps_na_tela: f64,
    pub na_tela_medido: bool,
}

impl Acumulado {
    /// Pura: soma `dt` segundos gerando, com os FPS daquele trecho. `exibidos`: o que o PresentMon viu chegar à tela.
    pub fn somar(&mut self, dt: f64, fps_do_jogo: f64, apresentados: f64, exibidos: Option<f64>) {
        if !(dt > 0.0) || !(fps_do_jogo > 0.0) {
            return;
        }
        self.segundos += dt;
        self.soma_do_jogo += fps_do_jogo * dt;
        self.soma_apresentados += apresentados * dt;
        if let Some(x) = exibidos.filter(|x| *x > 0.0) {
            self.soma_exibidos += x * dt;
            self.segundos_exibidos += dt;
        }
    }

    /// Pura.
    pub fn resumo(&self) -> Option<Resumo> {
        if self.segundos < MINIMO_S {
            return None;
        }
        let medido = self.segundos_exibidos >= self.segundos * COBERTURA_MEDIDA;
        Some(Resumo {
            segundos: self.segundos,
            fps_do_jogo: self.soma_do_jogo / self.segundos,
            fps_na_tela: if medido {
                self.soma_exibidos / self.segundos_exibidos
            } else {
                self.soma_apresentados / self.segundos
            },
            na_tela_medido: medido,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessao_curta_nao_tem_resumo() {
        let mut a = Acumulado::default();
        a.somar(29.0, 60.0, 180.0, Some(178.0));
        assert!(a.resumo().is_none());
    }

    #[test]
    fn medido_quando_o_presentmon_contou_a_sessao() {
        let mut a = Acumulado::default();
        a.somar(30.0, 60.0, 180.0, Some(176.0));
        a.somar(30.0, 50.0, 150.0, Some(146.0));
        let r = a.resumo().unwrap();
        assert!(r.na_tela_medido);
        assert!((r.fps_do_jogo - 55.0).abs() < 1e-9);
        assert!((r.fps_na_tela - 161.0).abs() < 1e-9);
    }

    #[test]
    fn sem_presentmon_quase_a_sessao_toda_vira_estimado_inteiro() {
        let mut a = Acumulado::default();
        a.somar(50.0, 60.0, 180.0, None);
        a.somar(10.0, 60.0, 180.0, Some(120.0));
        let r = a.resumo().unwrap();
        assert!(!r.na_tela_medido);
        assert!((r.fps_na_tela - 180.0).abs() < 1e-9, "não mistura o medido com o estimado");
    }

    #[test]
    fn jogo_parado_nao_entra() {
        let mut a = Acumulado::default();
        a.somar(40.0, 0.0, 0.0, None);
        assert_eq!(a.segundos, 0.0);
    }
}
