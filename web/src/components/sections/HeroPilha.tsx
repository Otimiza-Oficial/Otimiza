import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { CardSwap } from "@/components/reactbits/CardSwap";
import { cn } from "@/lib/cn";

/*
 * A pilha de telas do topo: três telas do programa que se revezam (CardSwap, do React Bits). Os números são
 * ilustrativos e cada tela diz isso — a mesma regra das prévias da seção de recursos.
 */

function Etiqueta({ escuro }: { escuro?: boolean }) {
  return <p className={cn("text-[10px]", escuro ? "text-[#8a8a8a]" : "text-subtle")}>Prévia ilustrativa</p>;
}

function Diagnostico() {
  const barras: [string, number, boolean][] = [
    ["CPU", 67, false],
    ["Memória", 90, true],
    ["Disco", 32, false],
  ];
  return (
    <div className="grain flex h-full w-full flex-col rounded-[16px] bg-[#0b0b0b] p-5 text-white shadow-[0_0_0_1px_#000,0_30px_60px_-30px_rgb(0_0_0/0.65)]">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <OtimizaLogo mark={15} className="text-white" />
          <span className="text-[12.5px] font-semibold">Diagnóstico</span>
        </div>
        <span className="rounded-md bg-white/10 px-1.5 py-0.5 font-mono text-[10px] text-[#d4d4d4]">ao vivo</span>
      </div>
      <p className="mt-5 font-mono text-[10px] tracking-[0.12em] text-[#8a8a8a] uppercase">O que está travando este PC</p>
      <p className="font-display mt-1.5 text-[17px] leading-[1.25] font-semibold tracking-[-0.03em]">
        Programas pedindo mais memória do que existe
      </p>
      <div className="mt-auto space-y-2.5">
        {barras.map(([nome, v, alerta]) => (
          <div key={nome}>
            <div className="flex justify-between font-mono text-[10.5px] text-[#a3a3a3]">
              <span>{nome}</span>
              <span className={alerta ? "text-white" : ""}>{v}%</span>
            </div>
            <div className="mt-1 h-1.5 rounded-full bg-white/10">
              <div className={cn("h-full rounded-full", alerta ? "bg-white" : "bg-white/45")} style={{ width: `${v}%` }} />
            </div>
          </div>
        ))}
        <Etiqueta escuro />
      </div>
    </div>
  );
}

function Prova() {
  const linhas: [string, string, string, boolean][] = [
    ["Travada no pior caso", "6,8 ms", "4,1 ms", true],
    ["Engasgos por minuto", "14", "9", true],
    ["CPU em segundo plano", "3,2%", "3,0%", false],
  ];
  return (
    <div className="sheet flex h-full w-full flex-col p-5">
      <div className="flex items-center gap-2">
        <OtimizaLogo mark={15} />
        <span className="text-[12.5px] font-semibold">Prova de resultado</span>
      </div>
      <ul className="mt-3 divide-y divide-line">
        {linhas.map(([nome, antes, depois, melhorou]) => (
          <li key={nome} className="flex items-center justify-between gap-2 py-2.5">
            <div>
              <p className="text-[11px] text-muted">{nome}</p>
              <p className="tabular mt-0.5 text-[13px] font-semibold">
                <span className="text-subtle line-through decoration-black/20">{antes}</span>
                <span className="mx-1.5 text-subtle">→</span>
                {depois}
              </p>
            </div>
            <span className={cn("rounded-md px-1.5 py-0.5 text-[10.5px] font-semibold", melhorou ? "bg-ink text-white" : "bg-sunken text-muted")}>
              {melhorou ? "Melhorou" : "Dentro do ruído"}
            </span>
          </li>
        ))}
      </ul>
      <div className="mt-auto">
        <Etiqueta />
      </div>
    </div>
  );
}

const QUADROS = [16, 17, 16, 15, 17, 16, 18, 16, 15, 16, 17, 40, 17, 16, 15, 16, 17, 16];

function Quadros() {
  return (
    <div className="sheet flex h-full w-full flex-col p-5">
      <div className="flex items-center justify-between">
        <p className="text-[12.5px] font-semibold">Tempo de cada quadro</p>
        <p className="tabular text-[11px] text-muted">média 60 FPS</p>
      </div>
      <div className="relative mt-8 flex h-[110px] items-end gap-[5px]">
        <div className="absolute inset-x-0 bottom-[40%] border-t border-dashed border-line-strong" />
        {QUADROS.map((ms, i) => (
          <div key={i} className={cn("relative flex-1 rounded-[3px]", ms > 30 ? "bg-ink" : "bg-[#d9d9d7]")} style={{ height: `${(ms / 40) * 100}%` }}>
            {ms > 30 && (
              <span className="tabular absolute -top-6 left-1/2 -translate-x-1/2 rounded bg-ink px-1.5 py-0.5 text-[10px] font-semibold whitespace-nowrap text-white">
                {ms} ms
              </span>
            )}
          </div>
        ))}
      </div>
      <p className="mt-3 text-[11px] text-muted">Um quadro de 40 ms: você sente, a média nem nota.</p>
      <div className="mt-auto">
        <Etiqueta />
      </div>
    </div>
  );
}

export function HeroPilha({ className }: { className?: string }) {
  return (
    <CardSwap className={className} largura={400} altura={290}>
      <Diagnostico />
      <Prova />
      <Quadros />
    </CardSwap>
  );
}
