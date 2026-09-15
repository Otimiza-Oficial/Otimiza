"use client";

import Image from "next/image";
import { useState, type KeyboardEvent } from "react";
import { Reveal } from "@/components/motion/Reveal";
import { Section, SectionHeading } from "@/components/ui/Section";
import { asset } from "@/lib/asset";
import { cn } from "@/lib/cn";
import { telas, texturaCetim } from "@/lib/site";

/*
 * Vídeo da vitrine. Quando o vídeo do produto existir, coloque o arquivo em
 * `public/video/` e o caminho aqui (ex.: "/video/otimiza.mp4"). Enquanto for
 * null, a vitrine mostra as capturas reais das sete abas.
 */
const VIDEO: string | null = null;

/**
 * A vitrine: uma moldura de cetim preto com o app de verdade por cima.
 * As abas embaixo trocam a captura — são as sete telas do programa, sem
 * nenhuma escondida.
 */
export function Showcase() {
  const [ativa, setAtiva] = useState(0);
  const tela = telas[ativa];

  const aoTeclar = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const proxima = (ativa + (e.key === "ArrowRight" ? 1 : telas.length - 1)) % telas.length;
    setAtiva(proxima);
    document.getElementById(`aba-${telas[proxima].id}`)?.focus();
  };

  return (
    <Section id="programa" labelledBy="programa-titulo">
      <Reveal>
        <SectionHeading
          id="programa-titulo"
          title="Um console só para o desempenho do seu PC."
          lead="Sete telas, e nenhuma escondida atrás de um botão a mais."
        />
      </Reveal>

      <Reveal delay={0.06}>
        <div className="grain relative mt-12 overflow-hidden rounded-[22px] bg-[#070707] shadow-[0_0_0_1px_rgb(10_10_10/0.1),0_30px_80px_-40px_rgb(0_0_0/0.55)] lg:mt-14">
          <Image
            src={texturaCetim}
            alt=""
            fill
            sizes="(min-width: 1160px) 1060px, 100vw"
            className="object-cover opacity-80"
            priority={false}
          />
          {/* Vinheta: escurece as bordas para a janela do app ser o ponto de luz. */}
          <div
            aria-hidden="true"
            className="absolute inset-0 bg-[radial-gradient(ellipse_at_50%_35%,transparent_20%,rgb(0_0_0/0.55)_85%)]"
          />

          <div className="relative px-4 pt-8 sm:px-10 sm:pt-12 lg:px-16 lg:pt-16">
            <figure
              id="painel-tela"
              role="tabpanel"
              aria-labelledby={`aba-${tela.id}`}
              className="overflow-hidden rounded-t-[12px] shadow-[0_0_0_1px_rgb(255_255_255/0.08),0_-10px_60px_-10px_rgb(0_0_0/0.9)]"
            >
              {VIDEO ? (
                <video
                  src={asset(VIDEO)}
                  className="block aspect-[1529/945] w-full"
                  autoPlay
                  muted
                  loop
                  playsInline
                  aria-label="Demonstração do Otimiza"
                />
              ) : (
                // As sete capturas têm tamanhos um pouco diferentes; a moldura
                // de proporção fixa impede a página de pular ao trocar de aba.
                <div className="relative aspect-[1529/945] w-full bg-[#0b0b0b]">
                  <Image
                    key={tela.id}
                    src={asset(`/capturas/${tela.id}.png`)}
                    alt={`Captura da tela ${tela.nome} do Otimiza`}
                    fill
                    sizes="(min-width: 1160px) 930px, 92vw"
                    className="object-cover object-top"
                    priority={ativa === 0}
                  />
                </div>
              )}
              <figcaption className="sr-only">{tela.legenda}</figcaption>
            </figure>
          </div>
        </div>
      </Reveal>

      {!VIDEO && (
        <div className="mt-6 flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
          <div
            role="tablist"
            aria-label="Telas do Otimiza"
            onKeyDown={aoTeclar}
            className="-mx-1 flex gap-1 overflow-x-auto px-1 py-1 [scrollbar-width:none]"
          >
            {telas.map((t, i) => (
              <button
                key={t.id}
                id={`aba-${t.id}`}
                type="button"
                role="tab"
                aria-selected={i === ativa}
                aria-controls="painel-tela"
                tabIndex={i === ativa ? 0 : -1}
                onClick={() => setAtiva(i)}
                className={cn(
                  "h-10 shrink-0 rounded-[var(--radius-control)] px-3.5 text-[13px] font-medium transition-[background-color,color,box-shadow] lg:h-[34px]",
                  i === ativa
                    ? "bg-white text-fg shadow-[0_0_0_1px_rgb(10_10_10/0.1),0_1px_3px_rgb(0_0_0/0.08)]"
                    : "text-muted hover:text-fg",
                )}
              >
                {t.nome}
              </button>
            ))}
          </div>
          <p aria-live="polite" className="max-w-[420px] text-[13.5px] leading-[1.55] text-muted lg:text-right">
            {tela.legenda}
          </p>
        </div>
      )}
    </Section>
  );
}
