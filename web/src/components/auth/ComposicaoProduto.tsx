import { Check, Cpu, KeyRound } from "lucide-react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { Mascote } from "@/components/brand/Mascote";
import { VERSAO_EXIBIDA } from "@/lib/site";

/**
 * O produto ao lado do formulário — em PEDAÇOS, e não uma captura inteira.
 *
 * Uma imagem do programa reduzida a 40% da tela não se lê: vira textura. Três
 * cartões recortados da interface, cada um dizendo uma coisa (a licença é
 * vitalícia, a prova é medida, o suporte é gente), cabem no tamanho que existe
 * e lembram o que a pessoa comprou.
 *
 * Os números são ilustrativos e a tela diz isso.
 */
export function ComposicaoProduto() {
  return (
    <div className="relative flex h-full flex-col justify-center overflow-hidden px-10 py-12 xl:px-14">
      <div className="relative">
        <Mascote tamanho={110} className="-ml-3 mb-4" />
        <p className="eyebrow">Dentro do Otimiza</p>
        <h2 className="font-display mt-3 max-w-[420px] text-[30px] leading-[1.1] font-semibold tracking-[-0.045em]">
          A sua licença, o seu PC e a prova do que mudou.
        </h2>

        <div className="mt-10 max-w-[460px] space-y-3">
          {/* Licença */}
          <article className="vidro-denso rounded-[16px]! p-4 shadow-[0_0_0_1px_var(--color-line),0_12px_24px_-16px_rgb(30_60_120/0.35)]">
            <div className="flex items-center gap-3">
              <span className="grid size-9 place-items-center rounded-[9px] bg-sunken">
                <KeyRound size={16} strokeWidth={2} aria-hidden="true" />
              </span>
              <div className="min-w-0 flex-1">
                <p className="text-[12.5px] font-semibold">Licença vitalícia</p>
                <p className="font-mono text-[11px] text-subtle">OTZ-4M8C-T2QP-9LZE</p>
              </div>
              <span className="inline-flex items-center gap-1 rounded-full bg-ink px-2 py-1 text-[10.5px] font-semibold text-white">
                <Check size={11} strokeWidth={3} aria-hidden="true" />
                Ativa
              </span>
            </div>
          </article>

          {/* Prova de resultado, recortada */}
          <article className="ml-8 vidro-denso rounded-[16px]! p-4 shadow-[0_0_0_1px_var(--color-line),0_12px_24px_-16px_rgb(30_60_120/0.35)]">
            <div className="flex items-center justify-between">
              <p className="flex items-center gap-2 text-[12.5px] font-semibold">
                <Cpu size={15} strokeWidth={2} aria-hidden="true" />
                Prova de resultado
              </p>
              <span className="text-[10.5px] text-subtle">antes → depois</span>
            </div>
            <dl className="mt-3 space-y-2">
              {[
                { m: "Travada no pior caso", a: "6,8 ms", d: "4,1 ms", v: "Melhorou" },
                { m: "Engasgos por minuto", a: "14", d: "9", v: "Melhorou" },
                { m: "CPU em segundo plano", a: "3,2%", d: "3,0%", v: "Dentro do ruído" },
              ].map((l) => (
                <div key={l.m} className="flex items-center justify-between gap-3">
                  <dt className="text-[11.5px] text-muted">{l.m}</dt>
                  <dd className="flex items-center gap-2">
                    <span className="tabular text-[11.5px] font-semibold">
                      <span className="text-subtle line-through decoration-current/30">{l.a}</span>{" "}
                      <span className="text-subtle">→</span> {l.d}
                    </span>
                    <span
                      className={
                        l.v === "Melhorou"
                          ? "rounded-full bg-ink px-2 py-0.5 text-[9.5px] font-semibold text-white"
                          : "rounded-[5px] bg-sunken px-1.5 py-0.5 text-[9.5px] font-semibold text-muted"
                      }
                    >
                      {l.v}
                    </span>
                  </dd>
                </div>
              ))}
            </dl>
          </article>

          {/* Suporte */}
          <article className="flex items-center gap-3 rounded-[16px] bg-noite p-4 text-white shadow-[0_0_0_1px_rgb(14_22_48/0.6),0_16px_28px_-20px_rgb(30_60_120/0.6)]">
            <span className="grid size-9 place-items-center rounded-[9px] bg-white/10">
              <BrandLogo brand="discord" size={17} decorative />
            </span>
            <div className="min-w-0 flex-1">
              <p className="text-[12.5px] font-semibold">Suporte no Discord</p>
              <p className="text-[11px] text-[#b7c2e4]">Quem emite a sua chave é uma pessoa.</p>
            </div>
            <OtimizaLogo mark={18} className="text-white" />
          </article>
        </div>

        <p className="mt-8 text-[11px] text-subtle">
          Prévia ilustrativa · Otimiza {VERSAO_EXIBIDA} para Windows 10 e 11
        </p>
      </div>
    </div>
  );
}
