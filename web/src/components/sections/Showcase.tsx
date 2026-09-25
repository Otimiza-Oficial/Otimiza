"use client";

import Image from "next/image";
import { motion } from "framer-motion";
import { useEffect, useState, type KeyboardEvent } from "react";
import { Reveal } from "@/components/motion/Reveal";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";
import { Section, SectionHeading } from "@/components/ui/Section";
import { cn } from "@/lib/cn";
import { telas, texturaCetim } from "@/lib/site";
import { AppJanela } from "./AppJanela";

const INTERVALO_MS = 5200;

/**
 * A vitrine: uma moldura de cetim preto com o app desenhado em código por cima. A janela abre quando entra
 * na tela e as telas se sucedem sozinhas; clicar numa aba para a troca automática.
 */
export function Showcase() {
  const [ativa, setAtiva] = useState(0);
  const [escolhida, setEscolhida] = useState(false);
  const [sobre, setSobre] = useState(false);
  const [visivel, setVisivel] = useState(false);
  const [aberta, setAberta] = useState(false);
  const parado = useReducedMotionSafe();
  const tela = telas[ativa];

  useEffect(() => {
    if (parado || escolhida || sobre || !visivel) return;
    const t = setTimeout(() => setAtiva((a) => (a + 1) % telas.length), INTERVALO_MS);
    return () => clearTimeout(t);
  }, [ativa, parado, escolhida, sobre, visivel]);

  const escolher = (i: number) => {
    setEscolhida(true);
    setAtiva(i);
  };

  const aoTeclar = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const proxima = (ativa + (e.key === "ArrowRight" ? 1 : telas.length - 1)) % telas.length;
    escolher(proxima);
    document.getElementById(`aba-${telas[proxima].id}`)?.focus();
  };

  return (
    <Section id="programa" labelledBy="programa-titulo">
      <Reveal>
        <SectionHeading
          id="programa-titulo"
          title="Um console só para o desempenho do seu PC."
          lead="As telas principais do programa, com os números de uma máquina de teste."
        />
      </Reveal>

      <motion.div
        viewport={{ margin: "0px 0px -15% 0px" }}
        onViewportEnter={() => {
          setVisivel(true);
          setAberta(true);
        }}
        onViewportLeave={() => setVisivel(false)}
        onMouseEnter={() => setSobre(true)}
        onMouseLeave={() => setSobre(false)}
        className="grain relative mt-12 overflow-hidden rounded-[22px] bg-[#070707] shadow-[0_0_0_1px_rgb(10_10_10/0.1),0_30px_80px_-40px_rgb(0_0_0/0.55)] lg:mt-14"
      >
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
            <AppJanela tela={tela.id} aberta={aberta} parado={parado} />
            <figcaption className="sr-only">
              Tela {tela.nome} do Otimiza: {tela.legenda}
            </figcaption>
          </figure>
        </div>
      </motion.div>

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
              onClick={() => escolher(i)}
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
        <p aria-live={escolhida ? "polite" : "off"} className="max-w-[420px] text-[13.5px] leading-[1.55] text-muted lg:text-right">
          {tela.legenda}
        </p>
      </div>
    </Section>
  );
}
