import {
  Activity,
  Check,
  Cpu,
  Gauge,
  HardDrive,
  MemoryStick,
  RotateCcw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { FeatureCard, Ilustrativo } from "@/components/features/FeatureCard";
import { Reveal } from "@/components/motion/Reveal";
import { Section, SectionHeading } from "@/components/ui/Section";
import { cn } from "@/lib/cn";

/*
 * Os números das telas abaixo são ilustrativos e dizem isso. Os NOMES — das
 * métricas, das otimizações, da chave de registro — são os do app.
 */

export function Features() {
  return (
    <Section id="recursos" labelledBy="recursos-titulo">
      <Reveal>
        <SectionHeading
          id="recursos-titulo"
          title="Tudo o que ele faz, com número."
          lead="E tudo o que ele se recusa a fazer, com o motivo."
        />
      </Reveal>

      <div className="mt-12 grid gap-4 lg:mt-14 lg:grid-cols-3">
        <Reveal className="lg:row-span-2">
          <FeatureCard
            icon={Gauge}
            title="Mede antes e depois."
            lead="Prova com número."
            text="Meça, otimize e meça de novo. Se o ganho não aparecer nos números, a tela diz que não houve ganho."
            stageClassName="min-h-[340px]"
          >
            <ProvaMock />
          </FeatureCard>
        </Reveal>

        <Reveal delay={0.05} className="lg:col-span-2">
          <FeatureCard
            icon={Activity}
            title="Mede o engasgo,"
            lead="não a média de FPS."
            text="Um congelamento de 40 ms arruína a suavidade e quase não mexe na média de 60 quadros por segundo."
            className="lg:flex-row lg:[&>div:first-child]:w-[46%] lg:[&>div:first-child]:shrink-0 lg:[&>div:first-child]:pb-7"
            stageClassName="min-h-[250px] lg:mt-0"
          >
            <EngasgoMock />
          </FeatureCard>
        </Reveal>

        <Reveal delay={0.08} className="lg:col-span-2">
          <FeatureCard
            icon={Cpu}
            title="Lê a sua máquina"
            lead="antes de oferecer."
            text="Desativar o SysMain ajuda em SSD e atrapalha em HD. Só aparece o que faz bem a este hardware."
            className="lg:flex-row lg:[&>div:first-child]:w-[46%] lg:[&>div:first-child]:shrink-0 lg:[&>div:first-child]:pb-7"
            stageClassName="min-h-[200px] lg:mt-0"
          >
            <HardwareMock />
          </FeatureCard>
        </Reveal>

        <Reveal>
          <FeatureCard icon={RotateCcw} title="Desfaz idêntico," lead="byte a byte." stageClassName="min-h-[230px]">
            <RegistroMock />
          </FeatureCard>
        </Reveal>

        <Reveal delay={0.05}>
          <FeatureCard
            icon={ShieldCheck}
            title="Não toca no que"
            lead="protege o seu PC."
            stageClassName="lg:flex lg:items-end"

          >
            <ProtecoesMock />
          </FeatureCard>
        </Reveal>

        <Reveal delay={0.1}>
          <FeatureCard
            icon={Sparkles}
            title="Diz quando não há"
            lead="nada a fazer."
            stageClassName="min-h-[230px]"
          >
            <CatalogoMock />
          </FeatureCard>
        </Reveal>
      </div>
    </Section>
  );
}

/* ───────────────────────── telas ───────────────────────── */

const METRICAS = [
  { nome: "Travada no pior caso", antes: "6,8 ms", depois: "4,1 ms", veredito: "Melhorou" },
  { nome: "Engasgos por minuto", antes: "14", depois: "9", veredito: "Melhorou" },
  { nome: "CPU consumida em segundo plano", antes: "3,2%", depois: "3,0%", veredito: "Dentro do ruído" },
  { nome: "RAM ocupada em segundo plano", antes: "4,6 GB", depois: "4,5 GB", veredito: "Dentro do ruído" },
];

function ProvaMock() {
  return (
    <div aria-hidden="true" className="absolute inset-x-0 bottom-0 top-0 overflow-hidden">
      <div className="sheet absolute top-3 -right-10 left-6 rotate-[-3deg] py-5 pr-16 pl-5 sm:left-8">
        <div className="flex items-center gap-2">
          <OtimizaLogo mark={16} />
          <span className="text-[12.5px] font-semibold">Prova de resultado</span>
        </div>
        <ul className="mt-4 divide-y divide-line">
          {METRICAS.map((m) => (
            <li key={m.nome} className="py-2.5">
              <p className="text-[11.5px] text-muted">{m.nome}</p>
              <div className="mt-1 flex items-center justify-between gap-2">
                <p className="tabular text-[13px] font-semibold">
                  <span className="text-subtle line-through decoration-black/20">{m.antes}</span>
                  <span className="mx-1.5 text-subtle">→</span>
                  {m.depois}
                </p>
                <span
                  className={cn(
                    "rounded-md px-1.5 py-0.5 text-[10.5px] font-semibold",
                    m.veredito === "Melhorou" ? "bg-ink text-white" : "bg-sunken text-muted",
                  )}
                >
                  {m.veredito}
                </span>
              </div>
            </li>
          ))}
        </ul>
        <Ilustrativo className="mt-3" />
      </div>
    </div>
  );
}

// Tempo de cada quadro em ms: um trecho estável a ~16 ms, com uma travada.
const QUADROS = [16, 17, 16, 15, 17, 16, 18, 16, 15, 16, 17, 40, 17, 16, 15, 16, 17, 16, 16, 15, 17, 16];

function EngasgoMock() {
  return (
    <div aria-hidden="true" className="absolute inset-0 overflow-hidden">
      <div className="sheet absolute top-6 -right-10 -bottom-10 left-6 rotate-[2deg] py-5 pr-16 pl-5 lg:top-10 lg:left-2">
        <div className="flex items-center justify-between">
          <p className="text-[12.5px] font-semibold">Tempo de cada quadro</p>
          <p className="tabular text-[11px] text-muted">média 60 FPS</p>
        </div>
        <div className="relative mt-8 flex h-[100px] items-end gap-[5px]">
          <div className="absolute inset-x-0 bottom-[40%] border-t border-dashed border-line-strong" />
          {QUADROS.map((ms, i) => (
            <div
              key={i}
              className={cn("relative flex-1 rounded-[3px]", ms > 30 ? "bg-ink" : "bg-[#d9d9d7]")}
              style={{ height: `${(ms / 40) * 100}%` }}
            >
              {ms > 30 && (
                <span className="tabular absolute -top-6 left-1/2 -translate-x-1/2 rounded bg-ink px-1.5 py-0.5 text-[10px] font-semibold whitespace-nowrap text-white">
                  {ms} ms
                </span>
              )}
            </div>
          ))}
        </div>
        <p className="mt-3 text-[11px] text-muted">Um quadro de 40 ms: você sente, a média nem nota.</p>
      </div>
    </div>
  );
}

function Chip({ children, label }: { children: React.ReactNode; label: string }) {
  return (
    <div className="flex flex-col items-center gap-2">
      <div className="grid size-12 place-items-center rounded-[12px] bg-white shadow-[0_0_0_1px_rgb(10_10_10/0.08),0_4px_10px_-4px_rgb(0_0_0/0.15)]">
        {children}
      </div>
      <span className="text-[10.5px] font-medium whitespace-nowrap text-muted">{label}</span>
    </div>
  );
}

function HardwareMock() {
  const linha = <span className="mb-6 h-px w-2 bg-line-strong sm:w-4" />;
  return (
    <div aria-hidden="true" className="flex h-full flex-col items-center justify-center gap-5 px-4 pb-7 lg:pt-7">
      <div className="flex items-center gap-2 sm:gap-3">
        <Chip label="Processador">
          <Cpu size={22} strokeWidth={1.75} />
        </Chip>
        {linha}
        <Chip label="Memória">
          <MemoryStick size={22} strokeWidth={1.75} />
        </Chip>
        {linha}
        <Chip label="SSD">
          <HardDrive size={22} strokeWidth={1.75} />
        </Chip>
        {linha}
        <Chip label="Vídeo">
          <BrandLogo brand="nvidia" size={24} decorative />
        </Chip>
      </div>
      <p className="flex items-center gap-1.5 text-[12px] font-medium text-fg">
        <Check size={14} strokeWidth={2.5} /> Hardware lido antes de oferecer
      </p>
    </div>
  );
}

function RegistroMock() {
  return (
    <div aria-hidden="true" className="absolute inset-0 overflow-hidden">
      <div className="absolute top-0 -right-4 -bottom-8 left-6 rotate-[-3deg] rounded-[14px] bg-[#141414] p-5 font-mono text-[11.5px] leading-[1.75] text-[#d4d4d4] shadow-[0_24px_50px_-20px_rgb(0_0_0/0.6)]">
        <div className="flex items-center justify-between text-[10px] tracking-[0.14em] text-[#8a8a8a] uppercase">
          <span>Estado anterior</span>
          <span>PriorityControl</span>
        </div>
        <div className="my-3 h-px bg-white/10" />
        <p className="text-[#8a8a8a]">Win32PrioritySeparation</p>
        <p>
          antes:&nbsp; <span className="text-white">2</span>
        </p>
        <p>
          depois: <span className="text-white">38</span>
        </p>
        <p className="mt-2 flex items-center gap-1.5 text-white">
          <Check size={13} strokeWidth={2.5} /> gravado antes de escrever
        </p>
      </div>
    </div>
  );
}

function ProtecoesMock() {
  const itens = ["Windows Update intacto", "Defender e firewall intactos", "Proteção Spectre/Meltdown ligada"];
  return (
    <ul className="space-y-2.5 px-6 pb-7 sm:px-7">
      {itens.map((t) => (
        <li
          key={t}
          className="flex items-center gap-2.5 rounded-[10px] bg-white px-3.5 py-3 text-[13px] shadow-[0_0_0_1px_rgb(10_10_10/0.07),0_1px_2px_rgb(0_0_0/0.04)]"
        >
          <Check size={15} strokeWidth={2.5} className="shrink-0" aria-hidden="true" />
          {t}
        </li>
      ))}
    </ul>
  );
}

function CatalogoMock() {
  const itens = [
    { nome: "Plano de energia Alto Desempenho", estado: "aplicar" },
    { nome: "Liberar limites de inicialização", estado: "feito" },
    { nome: "Desativar hibernação", estado: "feito" },
  ];
  return (
    <div aria-hidden="true" className="absolute inset-0 overflow-hidden">
      <div className="sheet absolute top-0 -right-10 -bottom-8 left-6 rotate-[-3deg] py-4 pr-16 pl-4">
        <p className="eyebrow text-[10px]">Catálogo · Sistema</p>
        <ul className="mt-3 divide-y divide-line">
          {itens.map((i) => (
            <li key={i.nome} className="flex items-center justify-between gap-3 py-2.5">
              <span className="text-[12px] font-medium">{i.nome}</span>
              {i.estado === "aplicar" ? (
                <span className="rounded-md bg-ink px-2 py-1 text-[10px] font-semibold text-white">Aplicar</span>
              ) : (
                <span className="text-[10px] font-semibold tracking-[0.08em] whitespace-nowrap text-muted uppercase">
                  Já otimizado
                </span>
              )}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
