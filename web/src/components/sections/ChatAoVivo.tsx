"use client";

/*
 * A conversa do suporte chegando mensagem por mensagem, como no Discord — no estilo do AnimatedList do React Bits
 * (https://reactbits.dev/components/animated-list; Copyright (c) 2026 David Haz, MIT + Commons Clause). Antes de
 * cada resposta da equipe aparece "digitando…". Cada mensagem ocupa o lugar dela desde o começo (só fica invisível),
 * então a seção não pula. Menos movimento = conversa inteira de uma vez.
 */
import { AnimatePresence, motion, useInView } from "framer-motion";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";
import { Avatar } from "@/components/ui/Avatar";

export type Fala = { autor: string; foto?: string; hora: string; equipe?: boolean; texto: ReactNode };

const DIGITANDO_MS = 1100;
const ENTRE_MS = 900;

function Pontos() {
  return (
    <span className="inline-flex items-center gap-1" aria-hidden="true">
      {[0, 1, 2].map((i) => (
        <motion.span
          key={i}
          className="size-1.5 rounded-full bg-subtle"
          animate={{ opacity: [0.25, 1, 0.25], y: [0, -2, 0] }}
          transition={{ duration: 0.9, repeat: Infinity, delay: i * 0.15 }}
        />
      ))}
    </span>
  );
}

export function ChatAoVivo({ falas }: { falas: Fala[] }) {
  const ref = useRef<HTMLUListElement>(null);
  const visto = useInView(ref, { once: true, margin: "0px 0px -20% 0px" });
  const reduce = useReducedMotionSafe();
  // Quantas já chegaram; "digitando" vale para a próxima, se for da equipe.
  const [chegaram, setChegaram] = useState(0);
  const [digitando, setDigitando] = useState(false);

  useEffect(() => {
    if (!visto || reduce || chegaram >= falas.length) return;
    const proxima = falas[chegaram];
    if (proxima.equipe && !digitando) {
      const t = setTimeout(() => setDigitando(true), chegaram === 0 ? 200 : ENTRE_MS);
      return () => clearTimeout(t);
    }
    const t = setTimeout(
      () => {
        setDigitando(false);
        setChegaram((n) => n + 1);
      },
      digitando ? DIGITANDO_MS : chegaram === 0 ? 200 : ENTRE_MS,
    );
    return () => clearTimeout(t);
  }, [visto, reduce, chegaram, digitando, falas]);

  const todas = reduce ? falas.length : chegaram;

  return (
    <ul ref={ref} className="space-y-5 px-5 py-5">
      {falas.map((f, i) => {
        const mostrada = i < todas;
        const escrevendo = !mostrada && digitando && i === chegaram;
        return (
          <li key={i} className="relative">
            <motion.div
              className="flex gap-3"
              initial={false}
              animate={mostrada ? { opacity: 1, y: 0, scale: 1 } : { opacity: 0, y: 10, scale: 0.98 }}
              transition={reduce ? { duration: 0 } : { type: "spring", stiffness: 260, damping: 24 }}
              style={{ transformOrigin: "left center" }}
            >
              {f.foto ? (
                <Avatar src={f.foto} nome={f.autor} size={36} />
              ) : (
                <span className="grid size-9 shrink-0 place-items-center rounded-full bg-ink text-white">
                  <OtimizaLogo mark={18} className="text-white" />
                </span>
              )}
              <div className="min-w-0">
                <p className="flex items-center gap-2 text-[13px]">
                  <span className="font-semibold">{f.autor}</span>
                  {f.equipe && (
                    <span className="rounded bg-ink px-1.5 py-px text-[9.5px] font-semibold tracking-[0.06em] text-white uppercase">
                      Equipe
                    </span>
                  )}
                  <span className="text-[11px] text-subtle">{f.hora}</span>
                </p>
                <div className="mt-1 text-[13.5px] leading-[1.55] text-[#2b2b2b]">{f.texto}</div>
              </div>
            </motion.div>
            <AnimatePresence>
              {escrevendo && (
                <motion.p
                  className="absolute top-1 left-0 flex items-center gap-2 text-[12px] text-subtle"
                  initial={{ opacity: 0, y: 4 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0 }}
                >
                  <span className="grid size-9 shrink-0 place-items-center rounded-full bg-ink text-white">
                    <OtimizaLogo mark={18} className="text-white" />
                  </span>
                  <Pontos />
                  Otimiza está digitando…
                </motion.p>
              )}
            </AnimatePresence>
          </li>
        );
      })}
    </ul>
  );
}
