import { Check, Crosshair, FlaskConical, Gauge } from "lucide-react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { FeatureCard, Ilustrativo } from "@/components/features/FeatureCard";
import { Reveal } from "@/components/motion/Reveal";
import { Section, SectionHeading } from "@/components/ui/Section";
import { cn } from "@/lib/cn";

/*
 * O Turbo da 3.5. Os números das telas são ilustrativos e dizem isso; o que cada tela AFIRMA (o gargalo decide a
 * receita, a prova de 8 rodadas, desligar se piorar, a escolha por jogo) é o que o app faz.
 */

export function Turbo() {
  return (
    <Section id="turbo" labelledBy="turbo-titulo">
      <Reveal>
        <p className="eyebrow">Novo na 3.5</p>
        <SectionHeading
          id="turbo-titulo"
          className="mt-3"
          title="Turbo: um botão para o jogo aberto."
          lead="Mede o que está no limite, liga só o que ajuda e prova o ganho."
        />
      </Reveal>

      <div className="mt-12 grid gap-4 lg:mt-14 lg:grid-cols-3">
        <Reveal>
          <FeatureCard
            icon={Gauge}
            title="Mede o gargalo,"
            lead="liga só o que ajuda."
            text="Dez segundos de jogo dizem o que está no limite. Placa: o jogo renderiza menos pixels e o Otimiza amplia com FSR 1. Processador: energia de jogo e geração de quadros."
            stageClassName="min-h-[250px]"
          >
            <ReceitaMock />
          </FeatureCard>
        </Reveal>

        <Reveal delay={0.05}>
          <FeatureCard
            icon={FlaskConical}
            title="Prova sozinho,"
            lead="e desliga se piorar."
            text="Na primeira vez em cada jogo: 8 rodadas alternadas, com e sem o Turbo, e o resultado com margem de erro. Piorou, ele desliga e não liga de novo aquela receita."
            stageClassName="min-h-[250px]"
          >
            <ProvaMock />
          </FeatureCard>
        </Reveal>

        <Reveal delay={0.1}>
          <FeatureCard
            icon={Crosshair}
            title="Menor atraso"
            lead="ou mais FPS."
            text="Você escolhe por jogo. Jogo de tiro já vem em Menor atraso: sem geração de quadros, só o FPS real a mais."
            stageClassName="min-h-[250px]"
          >
            <EscolhaMock />
          </FeatureCard>
        </Reveal>
      </div>
    </Section>
  );
}

/* ───────────────────────── telas ───────────────────────── */

function ReceitaMock() {
  const passos = ["Escala Qualidade: 1280×720 → 1920×1080", "Energia de jogo ligada", "Sem geração: Menor atraso"];
  return (
    <div aria-hidden="true" className="absolute inset-0 overflow-hidden">
      <div className="sheet absolute top-2 -right-10 -bottom-10 left-6 rotate-[-3deg] py-5 pr-16 pl-5">
        <div className="flex items-center gap-2">
          <OtimizaLogo mark={16} />
          <span className="text-[12.5px] font-semibold">Turbo</span>
        </div>
        <p className="mt-3 text-[11.5px] text-muted">Placa de vídeo no limite · 62 fps</p>
        <ul className="mt-3 space-y-2">
          {passos.map((p) => (
            <li key={p} className="flex items-center gap-2 text-[12px] font-medium">
              <Check size={13} strokeWidth={2.5} className="shrink-0" /> {p}
            </li>
          ))}
        </ul>
        <Ilustrativo className="mt-4" />
      </div>
    </div>
  );
}

// A ordem das rodadas da prova: com (C) e sem (S) o Turbo, alternadas para o aquecimento não favorecer um lado.
const RODADAS: { com: boolean; fps: number }[] = [
  { com: true, fps: 88 },
  { com: false, fps: 62 },
  { com: false, fps: 63 },
  { com: true, fps: 87 },
  { com: false, fps: 61 },
  { com: true, fps: 89 },
  { com: true, fps: 88 },
  { com: false, fps: 62 },
];

function ProvaMock() {
  return (
    <div aria-hidden="true" className="absolute inset-0 overflow-hidden">
      <div className="sheet absolute top-2 -right-10 -bottom-10 left-6 rotate-[2deg] py-5 pr-16 pl-5">
        <p className="text-[12.5px] font-semibold">Prova do Turbo · 8 rodadas</p>
        <div className="mt-4 flex h-[70px] items-end gap-[5px]">
          {RODADAS.map((r, i) => (
            <div
              key={i}
              className={cn("flex-1 rounded-[3px]", r.com ? "bg-ink" : "bg-[#d9d9d7]")}
              style={{ height: `${(r.fps / 90) * 100}%` }}
            />
          ))}
        </div>
        <p className="tabular mt-4 text-[17px] font-semibold tracking-[-0.02em]">
          62 ± 1 <span className="text-subtle">→</span> 88 ± 1 fps
        </p>
        <span className="mt-2 inline-block rounded-md bg-ink px-1.5 py-0.5 text-[10.5px] font-semibold text-white">
          provado
        </span>
        <Ilustrativo className="mt-3" />
      </div>
    </div>
  );
}

function EscolhaMock() {
  return (
    <div aria-hidden="true" className="flex h-full flex-col items-center justify-center gap-4 px-6 pb-7">
      <div className="inline-flex gap-0.5 rounded-[12px] bg-sunken p-1 shadow-[inset_0_0_0_1px_rgb(10_10_10/0.06)]">
        <span className="rounded-[9px] bg-white px-3.5 py-2 text-[12.5px] font-semibold shadow-[0_1px_2px_rgb(0_0_0/0.08)]">
          Menor atraso
        </span>
        <span className="px-3.5 py-2 text-[12.5px] font-medium text-muted">Mais FPS</span>
      </div>
      <p className="max-w-[240px] text-center text-[12px] text-muted">
        Sem geração de quadros: só o FPS real a mais, para não acrescentar atraso.
      </p>
    </div>
  );
}
