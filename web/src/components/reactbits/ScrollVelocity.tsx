"use client";

/*
 * ScrollVelocity — adaptado de React Bits (https://reactbits.dev/text-animations/scroll-velocity).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: `framer-motion` no lugar de `motion/react`; parado fora da tela (não gasta quadro); menos
 * movimento = faixa parada; as cópias repetidas ficam escondidas do leitor de tela (ele lê cada frase uma vez).
 */
import { motion, useAnimationFrame, useInView, useMotionValue, useScroll, useSpring, useTransform, useVelocity } from "framer-motion";
import { useLayoutEffect, useRef, useState } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

function Faixa({ texto, velocidade, className }: { texto: string; velocidade: number; className?: string }) {
  const caixa = useRef<HTMLDivElement>(null);
  const copia = useRef<HTMLSpanElement>(null);
  const [largura, setLargura] = useState(0);
  const naTela = useInView(caixa, { margin: "100px 0px" });
  const reduce = useReducedMotionSafe();
  const baseX = useMotionValue(0);
  const { scrollY } = useScroll();
  const vel = useSpring(useVelocity(scrollY), { damping: 50, stiffness: 400 });
  const fator = useTransform(vel, [0, 1000], [0, 5], { clamp: false });
  const sentido = useRef(1);

  useLayoutEffect(() => {
    const medir = () => setLargura(copia.current?.offsetWidth ?? 0);
    medir();
    window.addEventListener("resize", medir);
    return () => window.removeEventListener("resize", medir);
  }, []);

  const x = useTransform(baseX, (v) => {
    if (!largura) return "0px";
    const r = (((v + largura) % largura) + largura) % largura;
    return `${r - largura}px`;
  });

  useAnimationFrame((_, delta) => {
    if (!naTela || reduce) return;
    let passo = sentido.current * velocidade * (delta / 1000);
    const f = fator.get();
    if (f < 0) sentido.current = -1;
    else if (f > 0) sentido.current = 1;
    passo += sentido.current * passo * f;
    baseX.set(baseX.get() + passo);
  });

  return (
    <div ref={caixa} className="relative overflow-hidden">
      <motion.div className="flex whitespace-nowrap" style={{ x }}>
        {Array.from({ length: 6 }, (_, i) => (
          <span key={i} ref={i === 0 ? copia : undefined} aria-hidden={i > 0} className={`shrink-0 ${className ?? ""}`}>
            {texto}&nbsp;
          </span>
        ))}
      </motion.div>
    </div>
  );
}

export function ScrollVelocity({
  faixas,
  velocidade = 60,
}: {
  faixas: { texto: string; className?: string }[];
  velocidade?: number;
}) {
  return (
    <div>
      {faixas.map((f, i) => (
        <Faixa key={i} texto={f.texto} className={f.className} velocidade={i % 2 ? -velocidade : velocidade} />
      ))}
    </div>
  );
}
