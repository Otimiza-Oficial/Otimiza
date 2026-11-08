// O alerta do Oti por cima do jogo: o 1% pior CAIU em relação ao que esta partida vinha fazendo. Pura, sem DOM, para
// rodar no teste (`node --test ui/testes/queda.test.ts`).
//
// Referência: a mediana das últimas leituras (meio minuto a duas por segundo), não um número fixo — 60 no CS2 é
// desastre, no FiveM pode ser o normal. Cai abaixo de 75% dela em duas leituras seguidas: acende. Fica aceso pelo
// menos 3 s (uma piscada que some antes de a pessoa olhar não serve) e apaga quando volta acima de 85%.

export const JANELA = 60;
export const MINIMO_PARA_REFERENCIA = 20;
const CAI = 0.75;
const VOLTA = 0.85;
const SEGURA_MS = 3000;

export interface Vigia {
  historico: number[];
  abaixo: number;
  aceso: boolean;
  acesoEm: number;
}

export function nova(): Vigia {
  return { historico: [], abaixo: 0, aceso: false, acesoEm: 0 };
}

function mediana(v: number[]) {
  const o = [...v].sort((a, b) => a - b);
  return o[Math.floor(o.length / 2)];
}

/** Recebe a leitura nova (ms desde o início, 1% pior em FPS ou null) e devolve se o alerta está aceso. */
export function passo(v: Vigia, agora: number, low: number | null): boolean {
  if (low == null || !(low > 0)) {
    // Sem número (menu, carregando): não acende nem apaga por isso.
    return v.aceso;
  }
  const referencia = v.historico.length >= MINIMO_PARA_REFERENCIA ? mediana(v.historico) : null;
  if (referencia != null) {
    if (low < referencia * CAI) {
      v.abaixo += 1;
      if (v.abaixo >= 2 && !v.aceso) {
        v.aceso = true;
        v.acesoEm = agora;
      }
    } else {
      v.abaixo = 0;
      if (v.aceso && low >= referencia * VOLTA && agora - v.acesoEm >= SEGURA_MS) v.aceso = false;
    }
  }
  // A queda não entra na referência enquanto está acesa: senão a referência desce junto e "normaliza" o tranco.
  if (!v.aceso) {
    v.historico.push(low);
    if (v.historico.length > JANELA) v.historico.shift();
  }
  return v.aceso;
}
