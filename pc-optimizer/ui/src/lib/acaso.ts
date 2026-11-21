// Um sorteio com semente (mulberry32): as partículas da marca saem sempre no mesmo lugar, de uma abertura para a
// outra e entre o print e a tela. Nada de `Math.random()` em desenho.
export function sorteio(semente: number) {
  let a = semente >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Um valor com distribuição de sino (Box-Muller), para a poeira se juntar no centro do brilho. */
export function sino(acaso: () => number) {
  const u = Math.max(acaso(), 1e-9);
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * acaso());
}

/** O brilho de quatro pontas da marca, centrado em (x, y), com o raio r. */
export function estrela(x: number, y: number, r: number) {
  const f = (n: number) => n.toFixed(1);
  return `M${f(x)} ${f(y - r)}Q${f(x)} ${f(y)} ${f(x + r)} ${f(y)}Q${f(x)} ${f(y)} ${f(x)} ${f(y + r)}Q${f(x)} ${f(y)} ${f(x - r)} ${f(y)}Q${f(x)} ${f(y)} ${f(x)} ${f(y - r)}Z`;
}
