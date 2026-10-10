// O placar do Frame Boost: lê as capturas do `ab-frameboost.ps1` (PresentMon 2.x, --v2_metrics) e mostra, por modo, o
// que o JOGO entregou e o que CHEGOU À TELA. Com o Frame Boost, a tela é a sobreposição do Otimiza (pc-optimizer.exe);
// sem ele, os quadros exibidos do próprio jogo.
//
//   node scripts/laboratorio/ab-resumo.mjs <pasta> [segundos iniciais a descartar = 6]
//
// "Tempo na tela": quanto cada quadro ficou exibido (DisplayedTime). Ritmo liso = todos iguais (180 Hz: 5,56 ms); o
// desvio e a fração acima de 1,5× a mediana são o judder que o olho vê.
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, basename } from "node:path";

const pasta = process.argv[2];
const pular = Number(process.argv[3] ?? 6);
if (!pasta) {
  console.error("uso: node ab-resumo.mjs <pasta> [segundos a descartar]");
  process.exit(2);
}

const num = (v) => {
  const n = Number(String(v ?? "").trim());
  return Number.isFinite(n) ? n : null;
};
const media = (v) => (v.length ? v.reduce((a, b) => a + b, 0) / v.length : null);
const mediana = (v) => {
  if (!v.length) return null;
  const s = [...v].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
};
const desvio = (v) => {
  const m = media(v);
  return m === null ? null : Math.sqrt(media(v.map((x) => (x - m) ** 2)));
};
const f1 = (v, casas = 1) => (v === null || v === undefined ? "—" : v.toFixed(casas));

function ler(caminho) {
  const linhas = readFileSync(caminho, "utf8").replace(/^﻿/, "").split(/\r?\n/).filter(Boolean);
  const cab = linhas[0].split(",").map((c) => c.trim());
  const col = (n) => cab.indexOf(n);
  const c = {
    app: col("Application"),
    tipo: col("FrameType"),
    modo: col("PresentMode"),
    inicio: col("CPUStartTime"),
    quadro: col("FrameTime"),
    cpu: col("CPUBusy"),
    gpu: col("GPUBusy"),
    tela: col("DisplayedTime"),
  };
  return linhas.slice(1).map((l) => {
    const p = l.split(",");
    return {
      app: p[c.app] ?? "",
      tipo: p[c.tipo] ?? "",
      modo: p[c.modo] ?? "",
      inicio: num(p[c.inicio]),
      quadro: num(p[c.quadro]),
      cpu: num(p[c.cpu]),
      gpu: num(p[c.gpu]),
      tela: num(p[c.tela]),
    };
  });
}

function resumir(arquivo) {
  const todas = ler(arquivo);
  const t0 = Math.min(...todas.map((q) => q.inicio).filter((x) => x !== null));
  const validas = todas.filter((q) => q.inicio !== null && q.inicio - t0 >= pular * 1000);
  const doJogo = validas.filter((q) => !/^pc-optimizer/i.test(q.app) && /Application|NA|^$/.test(q.tipo.trim()));
  const doOtimiza = validas.filter((q) => /^pc-optimizer/i.test(q.app));

  const intervalos = doJogo.map((q) => q.quadro).filter((x) => x && x > 0);
  const seg = intervalos.reduce((a, b) => a + b, 0) / 1000;
  const m = media(intervalos);
  const piores = [...intervalos].sort((a, b) => b - a).slice(0, Math.max(1, Math.floor(intervalos.length / 100)));
  const trancos = intervalos.filter((x) => x > 2.5 * m).length;
  const modos = {};
  for (const q of doJogo) modos[q.modo] = (modos[q.modo] ?? 0) + 1;

  // A tela: a sobreposição do Otimiza quando ela apresentou; senão os quadros exibidos do jogo.
  const fonteDaTela = doOtimiza.length > 50 ? doOtimiza : doJogo;
  const naTela = fonteDaTela.map((q) => q.tela).filter((x) => x && x > 0);
  const segTela = naTela.reduce((a, b) => a + b, 0) / 1000;
  const medTela = mediana(naTela);

  return {
    modo: basename(arquivo, ".csv"),
    segundos: seg,
    jogo_fps: intervalos.length / seg,
    jogo_mediana: m ? 1000 / mediana(intervalos) : null,
    jogo_1pct: piores.length ? 1000 / media(piores) : null,
    jogo_p99_ms: [...intervalos].sort((a, b) => a - b)[Math.floor(intervalos.length * 0.99)] ?? null,
    trancos_min: seg ? (trancos / seg) * 60 : null,
    cpu_ms: media(doJogo.map((q) => q.cpu).filter((x) => x !== null)),
    gpu_ms: media(doJogo.map((q) => q.gpu).filter((x) => x !== null)),
    apresentacao: Object.entries(modos).sort((a, b) => b[1] - a[1])[0]?.[0] ?? "—",
    tela_fonte: fonteDaTela === doOtimiza ? "Otimiza" : "jogo",
    tela_fps: segTela ? naTela.length / segTela : null,
    tela_desvio_ms: desvio(naTela),
    tela_judder_pct: naTela.length && medTela ? (naTela.filter((x) => x > 1.5 * medTela).length / naTela.length) * 100 : null,
  };
}

const arquivos = readdirSync(pasta)
  .filter((n) => n.endsWith(".csv"))
  .sort()
  .map((n) => join(pasta, n));
const linhas = arquivos.map(resumir);

console.log(`\nPlacar do Frame Boost (${pasta}; ${pular} s iniciais descartados)\n`);
console.log("| Modo | Jogo fps | Mediana | 1% low | p99 ms | Trancos/min | Na tela fps | Tempo na tela ±ms | Judder | Apresentação |");
console.log("|---|---|---|---|---|---|---|---|---|---|");
for (const l of linhas) {
  console.log(
    `| ${l.modo} | ${f1(l.jogo_fps)} | ${f1(l.jogo_mediana)} | ${f1(l.jogo_1pct)} | ${f1(l.jogo_p99_ms)} | ${f1(l.trancos_min)} | ${f1(l.tela_fps)} (${l.tela_fonte}) | ${f1(l.tela_desvio_ms, 2)} | ${f1(l.tela_judder_pct)}% | ${l.apresentacao} |`,
  );
}
writeFileSync(join(pasta, "placar.json"), JSON.stringify(linhas, null, 2));
console.log(`\nGravado em ${join(pasta, "placar.json")}`);
