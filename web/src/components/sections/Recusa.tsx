import { Ban } from "lucide-react";
import { Reveal } from "@/components/motion/Reveal";
import { Section, SectionHeading } from "@/components/ui/Section";

/*
 * "O que ele se recusa a fazer", do README da raiz, com o motivo de cada recusa. Uma lista curta e dura de propósito:
 * é o que separa o Otimiza do otimizador que promete "PC 300% mais rápido".
 */
const RECUSAS = [
  {
    item: "Desligar as proteções do processador contra Spectre e Meltdown",
    motivo: "Mesmo rendendo FPS de verdade: o ganho não vale abrir o PC para ataque.",
  },
  {
    item: "Desligar o Windows Update, o Defender ou o firewall",
    motivo: "Nunca desliga. O máximo que a 1.0 faz é adiar download automático enquanto você joga.",
  },
  {
    item: "“Limpeza de registro”",
    motivo: "Não tem ganho que se meça, e quebra programa instalado.",
  },
  {
    item: "Liberar RAM à força",
    motivo: "Deixa o gráfico bonito e o PC mais lento logo depois.",
  },
  {
    item: "Escrever na BIOS",
    motivo: "Em placa de consumo, errar ali inutiliza a placa-mãe. Ele lê a BIOS e explica o que ajustar.",
  },
];

export function Recusa() {
  return (
    <Section id="recusa" labelledBy="recusa-titulo">
      <div className="card p-6 sm:p-10">
        <div className="grid gap-10 lg:grid-cols-[0.9fr_1.1fr] lg:gap-14">
          <Reveal>
            <p className="eyebrow">O que ele se recusa a fazer</p>
            <SectionHeading id="recusa-titulo" className="mt-3" title="Tem coisa que dá FPS" lead="e não vale o risco." />
            <p className="mt-6 max-w-[420px] text-[15px] leading-[1.65] text-muted">
              Também não injeta nada no jogo e não instala driver próprio no núcleo do Windows. O que ele lê do
              hardware decide o que ele oferece: o que faria mal a esta máquina nem aparece.
            </p>
          </Reveal>

          <ul className="divide-y divide-line">
            {RECUSAS.map((r, i) => (
              <li key={r.item}>
                <Reveal delay={i * 0.04} className="flex gap-4 py-4">
                  <Ban size={18} strokeWidth={2} className="mt-0.5 shrink-0 text-ambar-700 dark:text-[#f0b25e]" aria-hidden="true" />
                  <div>
                    <p className="text-[15px] font-semibold tracking-[-0.01em]">{r.item}</p>
                    <p className="mt-1 text-[14px] leading-[1.55] text-muted">{r.motivo}</p>
                  </div>
                </Reveal>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </Section>
  );
}
