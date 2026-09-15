import { Check, KeyRound } from "lucide-react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { Ilustrativo } from "@/components/features/FeatureCard";
import { Reveal } from "@/components/motion/Reveal";
import { Avatar } from "@/components/ui/Avatar";
import { Section, SectionHeading } from "@/components/ui/Section";
import { pessoas } from "@/lib/site";

const PASSOS = [
  {
    titulo: "Baixe e instale",
    texto: "Grátis, sem cadastro. O diagnóstico completo já roda aqui, antes de qualquer pagamento.",
  },
  {
    titulo: "Copie o código da máquina",
    texto: "Na primeira abertura aparece o código deste PC, no formato OTZ-XXXX-XXXX-XXXX.",
  },
  {
    titulo: "Mande o código e pague",
    texto: "No Discord: Pix, cartão de crédito ou boleto, pelo Mercado Pago. A chave é emitida para esse código.",
  },
  {
    titulo: "Cole a chave",
    texto: "A conferência acontece na sua máquina, sem consultar servidor. É para sempre.",
  },
];

export function License() {
  return (
    <Section id="licenca" labelledBy="licenca-titulo">
      <Reveal>
        <SectionHeading
          id="licenca-titulo"
          title="Uma chave, um computador."
          lead="E formatar não custa chave nova."
        />
      </Reveal>

      <div className="mt-12 grid gap-4 lg:mt-14 lg:grid-cols-[1.15fr_1fr]">
        <ol className="grid gap-4 sm:grid-cols-2">
          {PASSOS.map((p, i) => (
            <li key={p.titulo}>
              <Reveal delay={i * 0.05} className="card h-full p-6">
                <span className="tabular font-mono text-[12px] text-subtle">0{i + 1}</span>
                <h3 className="font-display mt-6 text-[18px] font-semibold tracking-[-0.03em]">{p.titulo}</h3>
                <p className="mt-2 text-[13.5px] leading-[1.55] text-muted">{p.texto}</p>
              </Reveal>
            </li>
          ))}
        </ol>

        <Reveal delay={0.1} className="h-full">
          <div className="card grid h-full min-h-[380px] place-items-center p-6 sm:p-10">
            <AtivacaoMock />
          </div>
        </Reveal>
      </div>
    </Section>
  );
}

function AtivacaoMock() {
  return (
    <div aria-hidden="true" className="w-full max-w-[380px]">
      <div className="sheet rotate-[-2deg] p-6">
        <div className="flex items-center gap-2">
          <OtimizaLogo mark={18} />
          <span className="text-[13px] font-semibold">Ativar o Otimiza</span>
        </div>

        <p className="mt-5 text-[11.5px] font-medium text-muted">Código desta máquina</p>
        <p className="tabular mt-1 rounded-[8px] bg-sunken px-3 py-2 font-mono text-[12.5px]">OTZ-4M8C-T2QP-9LZE</p>

        <p className="mt-4 text-[11.5px] font-medium text-muted">A sua chave</p>
        <div className="mt-1 flex items-center gap-2 rounded-[8px] px-3 py-2 shadow-[0_0_0_1px_rgb(10_10_10/0.14)]">
          <KeyRound size={14} strokeWidth={2} className="shrink-0 text-subtle" />
          <span className="truncate font-mono text-[12.5px] tracking-[0.12em]">●●●●●●●●●●●●●●●●</span>
        </div>

        <div className="mt-4 flex h-9 items-center justify-center rounded-[8px] bg-ink text-[12.5px] font-semibold text-white">
          Ativar
        </div>
      </div>

      <div className="sheet relative z-10 -mt-3 ml-8 flex rotate-[1.5deg] items-center gap-3 p-3.5 pr-4">
        <Avatar src={pessoas.julia.foto} nome={pessoas.julia.nome} size={34} />
        <div className="min-w-0 flex-1">
          <p className="text-[12.5px] font-semibold">Licença vitalícia ativada</p>
          <p className="text-[11px] text-muted">{pessoas.julia.nome} · neste computador</p>
        </div>
        <span className="grid size-6 place-items-center rounded-full bg-ink text-white">
          <Check size={13} strokeWidth={3} />
        </span>
      </div>
      <Ilustrativo className="mt-4" />
    </div>
  );
}
