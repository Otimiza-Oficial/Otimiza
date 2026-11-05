/*
 * O céu atrás de todas as páginas — o mesmo do app (`pc-optimizer/ui/src/componentes/Ceu.svelte`), com o céu
 * "limpo, algumas nuvens". Fixo atrás do conteúdo, sem script: as nuvens são gradientes radiais que derivam devagar
 * (e param com menos movimento pedido). Nada de `filter: blur` em tela cheia, que custaria placa o tempo todo.
 */

// Posições fixas: o mesmo céu em toda visita.
const NUVENS = [
  { x: 4, y: 10, l: 34, a: 0 },
  { x: 60, y: 6, l: 28, a: 8 },
  { x: 28, y: 58, l: 40, a: 3 },
  { x: 72, y: 52, l: 30, a: 12 },
];

export function Ceu() {
  return (
    <div className="ceu" aria-hidden="true">
      {NUVENS.map((n, i) => (
        <div
          key={i}
          className="ceu-nuvem"
          style={{
            left: `${n.x}%`,
            top: `${n.y}%`,
            width: `max(${n.l}vw, 260px)`,
            animationDelay: `-${n.a * 7}s`,
          }}
        />
      ))}
    </div>
  );
}
